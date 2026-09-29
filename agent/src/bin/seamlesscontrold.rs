#[cfg(target_os = "linux")]
mod linux {
    use ashpd::desktop::input_capture::{
        Barrier, BarrierID, BarrierPosition, Capabilities, CreateSessionOptions, InputCapture,
        ReleaseOptions,
    };
    use futures_util::StreamExt;
    use reis::{
        ei,
        event::{DeviceCapability, EiEvent},
    };
    use seamlesscontrol_core::control::{self, ControlHandle, ControlServer};
    use seamlesscontrol_core::omarchy::VirtualInput;
    use seamlesscontrol_core::protocol::{Frame, FrameError, Kind};
    use seamlesscontrol_core::receiver::{Injector, run_receiver_with_first_until};
    use seamlesscontrol_core::secure::{Identity, Role, SecureChannel, SecureError};
    use seamlesscontrol_core::state::InputEvent;
    use seamlesscontrol_core::storage::{
        is_revoked, key_fingerprint, list_peer_keys, load_or_create_identity, load_peer_key,
        load_topology, remember_peer_key, revoke_peer_key, save_topology,
    };
    use seamlesscontrol_core::topology::{Edge as LogicalEdge, Machine, Slot};
    use std::error::Error;
    use std::io;
    use std::net::{IpAddr, Shutdown, SocketAddr, TcpListener, TcpStream};
    use std::os::unix::net::UnixStream;
    use std::path::PathBuf;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    const PAIRING_SOCKET_TIMEOUT: Duration = Duration::from_secs(330);

    #[derive(Clone, Copy)]
    enum Edge {
        Left,
        Right,
        Top,
        Bottom,
    }

    impl Edge {
        fn from_logical(edge: LogicalEdge) -> Self {
            match edge {
                LogicalEdge::Left => Self::Left,
                LogicalEdge::Right => Self::Right,
                LogicalEdge::Top => Self::Top,
                LogicalEdge::Bottom => Self::Bottom,
            }
        }

        fn parse(value: &str) -> Result<Self, Box<dyn Error>> {
            match value {
                "left" | "izquierda" => Ok(Self::Left),
                "right" | "derecha" => Ok(Self::Right),
                "top" | "arriba" => Ok(Self::Top),
                "bottom" | "abajo" => Ok(Self::Bottom),
                _ => Err("edge must be left, right, top or bottom".into()),
            }
        }

        fn barrier(self, x: i32, y: i32, width: i32, height: i32) -> BarrierPosition {
            match self {
                Self::Left => BarrierPosition::new(x, y, x, y + height - 1),
                Self::Right => BarrierPosition::new(x + width, y, x + width, y + height - 1),
                Self::Top => BarrierPosition::new(x, y, x + width - 1, y),
                Self::Bottom => BarrierPosition::new(x, y + height, x + width - 1, y + height),
            }
        }

        fn release_position(self, position: (f32, f32)) -> (f64, f64) {
            let (x, y) = (position.0 as f64, position.1 as f64);
            match self {
                Self::Left => (x + 1.0, y),
                Self::Right => (x - 1.0, y),
                Self::Top => (x, y + 1.0),
                Self::Bottom => (x, y - 1.0),
            }
        }
    }

    fn input_event(event: EiEvent) -> Option<InputEvent> {
        match event {
            EiEvent::PointerMotion(value) => Some(InputEvent::Motion {
                dx_milli: (value.dx * 1000.0) as i32,
                dy_milli: (value.dy * 1000.0) as i32,
            }),
            EiEvent::Button(value) => Some(match value.state {
                ei::button::ButtonState::Press => InputEvent::ButtonDown(value.button),
                ei::button::ButtonState::Released => InputEvent::ButtonUp(value.button),
            }),
            EiEvent::KeyboardKey(value) => Some(match value.state {
                ei::keyboard::KeyState::Press => InputEvent::KeyDown(value.key),
                ei::keyboard::KeyState::Released => InputEvent::KeyUp(value.key),
            }),
            EiEvent::ScrollDelta(value) => Some(InputEvent::Scroll {
                horizontal_milli: (value.dx * 1000.0) as i32,
                vertical_milli: (value.dy * 1000.0) as i32,
            }),
            _ => None,
        }
    }

    fn send_frame(
        channel: &mut SecureChannel<TcpStream>,
        kind: Kind,
        epoch: u64,
        sequence: &mut u64,
        payload: Vec<u8>,
    ) -> Result<(), Box<dyn Error>> {
        *sequence = sequence.checked_add(1).ok_or("sequence exhausted")?;
        Frame {
            kind,
            epoch,
            sequence: *sequence,
            payload,
        }
        .write_to(channel)?;
        Ok(())
    }

    async fn capture_loop(
        mut channel: SecureChannel<TcpStream>,
        edge: Edge,
        control: ControlHandle,
    ) -> Result<(), Box<dyn Error>> {
        let portal = InputCapture::new().await?;
        let (session, _) = portal
            .create_session(
                None,
                CreateSessionOptions::default()
                    .set_capabilities(Capabilities::Keyboard | Capabilities::Pointer),
            )
            .await?;
        let zones = portal
            .zones(&session, Default::default())
            .await?
            .response()?;
        let region = zones
            .regions()
            .first()
            .ok_or("the portal returned no monitor regions")?;
        let barrier_id = BarrierID::new(1).ok_or("invalid barrier ID")?;
        let barrier = Barrier::new(
            barrier_id,
            edge.barrier(
                region.x_offset(),
                region.y_offset(),
                region.width() as i32,
                region.height() as i32,
            ),
        );
        let response = portal
            .set_pointer_barriers(&session, &[barrier], zones.zone_set(), Default::default())
            .await?
            .response()?;
        if !response.failed_barriers().is_empty() {
            return Err("the compositor rejected the selected edge".into());
        }
        let eis = portal.connect_to_eis(&session, Default::default()).await?;
        let stream = UnixStream::from(eis);
        stream.set_nonblocking(true)?;
        let context = ei::Context::new(stream)?;
        context.flush()?;
        let (_connection, mut events) = context
            .handshake_tokio("seamlesscontrol", ei::handshake::ContextType::Receiver)
            .await?;
        let mut activated = portal.receive_activated().await?;
        let mut deactivated = portal.receive_deactivated().await?;
        portal.enable(&session, Default::default()).await?;
        control.set_phase("ready");
        println!(
            "Captura activa. Cruce el borde elegido; Escape recupera el control, Ctrl+C termina."
        );

        let mut heartbeat = tokio::time::interval(Duration::from_secs(2));
        let mut pause_tick = tokio::time::interval(Duration::from_millis(100));
        pause_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut capture_enabled = true;
        let mut epoch = 0;
        let mut sequence = 0;
        let mut current_activation = None;
        let mut active = false;
        let mut release_position = None;
        let result: Result<(), Box<dyn Error>> = async {
            loop {
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => break,
                    _ = pause_tick.tick() => {
                        if control.revoked_active() { break; }
                        if control.paused() && capture_enabled {
                            if active {
                                send_frame(&mut channel, Kind::Control, epoch, &mut sequence, b"END".to_vec())?;
                                let mut options = ReleaseOptions::default().set_activation_id(current_activation.take());
                                if let Some(position) = release_position { options = options.set_cursor_position(position); }
                                portal.release(&session, options).await?;
                                active = false;
                            }
                            portal.disable(&session, Default::default()).await?;
                            capture_enabled = false;
                            control.set_phase("paused");
                        } else if !control.paused() && !capture_enabled {
                            portal.enable(&session, Default::default()).await?;
                            capture_enabled = true;
                            control.set_phase("ready");
                        }
                    }
                    _ = heartbeat.tick() => {
                        send_frame(&mut channel, Kind::Heartbeat, epoch, &mut sequence, Vec::new())?;
                    }
                    signal = activated.next() => {
                        let signal = signal.ok_or("capture activation stream closed")?;
                        if control.paused() {
                            let mut options = ReleaseOptions::default().set_activation_id(signal.activation_id());
                            if let Some(position) = signal.cursor_position() { options = options.set_cursor_position(edge.release_position(position)); }
                            portal.release(&session, options).await?;
                            continue;
                        }
                        epoch = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos() as u64;
                        sequence = 0;
                        current_activation = signal.activation_id();
                        active = true;
                        release_position = signal.cursor_position().map(|p| edge.release_position(p));
                        send_frame(&mut channel, Kind::Control, epoch, &mut sequence, b"BEGIN".to_vec())?;
                        control.set_phase("controlling");
                        println!("Control remoto activo. Escape devuelve el puntero.");
                    }
                    signal = deactivated.next() => {
                        if signal.is_none() { return Err("capture deactivation stream closed".into()); }
                        if active {
                            send_frame(&mut channel, Kind::Control, epoch, &mut sequence, b"END".to_vec())?;
                            active = false;
                            current_activation = None;
                            control.set_phase(if control.paused() { "paused" } else { "ready" });
                        }
                    }
                    event = events.next() => {
                        let event = event.ok_or("EIS event stream closed")??;
                        if let EiEvent::SeatAdded(seat) = &event {
                            seat.seat.bind_capabilities(DeviceCapability::Pointer | DeviceCapability::Keyboard | DeviceCapability::Scroll | DeviceCapability::Button);
                            context.flush()?;
                        }
                        if active {
                            if matches!(&event, EiEvent::KeyboardKey(key) if key.key == 1 && key.state == ei::keyboard::KeyState::Press) {
                                send_frame(&mut channel, Kind::Control, epoch, &mut sequence, b"END".to_vec())?;
                                let mut options = ReleaseOptions::default().set_activation_id(current_activation.take());
                                active = false;
                                if let Some(position) = release_position { options = options.set_cursor_position(position); }
                                portal.release(&session, options).await?;
                                control.set_phase("ready");
                                println!("Control local restaurado.");
                            } else if let Some(event) = input_event(event) {
                                send_frame(&mut channel, Kind::Input, epoch, &mut sequence, event.encode())?;
                            }
                        }
                    }
                }
            }
            Ok(())
        }.await;
        if active {
            let _ = send_frame(
                &mut channel,
                Kind::Control,
                epoch,
                &mut sequence,
                b"END".to_vec(),
            );
            let mut options = ReleaseOptions::default().set_activation_id(current_activation);
            if let Some(position) = release_position {
                options = options.set_cursor_position(position);
            }
            let _ = portal.release(&session, options).await;
        }
        let _ = portal.disable(&session, Default::default()).await;
        control.set_phase("disconnected");
        result
    }

    struct OmarchyInjector {
        input: VirtualInput,
        started: Instant,
    }

    impl Injector for OmarchyInjector {
        fn inject(&mut self, event: &InputEvent) -> io::Result<()> {
            let time_ms = self.started.elapsed().as_millis() as u32;
            self.input
                .apply(event, time_ms)
                .map_err(|error| io::Error::other(error.to_string()))
        }
    }

    fn local_address(ip: IpAddr) -> bool {
        match ip {
            IpAddr::V4(v4) => v4.is_private() || v4.is_loopback() || v4.is_link_local(),
            IpAddr::V6(v6) => {
                v6.is_loopback()
                    || v6.is_unicast_link_local()
                    || (v6.segments()[0] & 0xfe00) == 0xfc00
            }
        }
    }

    fn config_dir() -> Result<PathBuf, Box<dyn Error>> {
        if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
            return Ok(PathBuf::from(xdg).join("seamlesscontrol"));
        }
        Ok(
            PathBuf::from(std::env::var_os("HOME").ok_or("HOME is not set")?)
                .join(".config/seamlesscontrol"),
        )
    }

    fn serve_connection(
        stream: TcpStream,
        peer_ip: IpAddr,
        identity: &Identity,
        config: &std::path::Path,
        control: &ControlHandle,
    ) -> Result<(), Box<dyn Error>> {
        let peers = config.join("peers");
        let pinned = load_peer_key(&peers, peer_ip)?;
        stream.set_nodelay(true)?;
        stream.set_read_timeout(Some(PAIRING_SOCKET_TIMEOUT))?;
        stream.set_write_timeout(Some(PAIRING_SOCKET_TIMEOUT))?;
        let (mut channel, peer) =
            SecureChannel::connect(stream, Role::Responder, identity, pinned.as_ref(), |peer| {
                !is_revoked(&peers, &peer.public_key).unwrap_or(true) && control.confirm_pair(peer)
            })?;
        if is_revoked(&peers, &peer.public_key)? {
            return Err("peer identity has been revoked".into());
        }
        channel
            .stream_mut()
            .set_read_timeout(Some(Duration::from_secs(120)))?;
        channel
            .stream_mut()
            .set_write_timeout(Some(Duration::from_secs(120)))?;
        control.set_peer(&peer_ip.to_string());
        let greeting = Frame::read_from(&mut channel)?;
        if greeting.kind != Kind::Hello || greeting.payload != b"seamlesscontrol/1" {
            return Err("incompatible peer protocol".into());
        }
        Frame {
            kind: Kind::Hello,
            epoch: 0,
            sequence: 0,
            payload: b"seamlesscontrol/1".to_vec(),
        }
        .write_to(&mut channel)?;
        remember_peer_key(&peers, peer_ip, &peer.public_key)?;
        let first = Frame::read_from(&mut channel)?;
        if first.kind == Kind::Control && first.payload == b"PAIR" {
            Frame {
                kind: Kind::Control,
                epoch: 0,
                sequence: 0,
                payload: b"PAIRED".to_vec(),
            }
            .write_to(&mut channel)?;
            println!("Par {peer_ip} emparejado; todavía no se inició la captura.");
            return Ok(());
        }
        channel
            .stream_mut()
            .set_read_timeout(Some(Duration::from_secs(5)))?;
        channel
            .stream_mut()
            .set_write_timeout(Some(Duration::from_secs(5)))?;
        println!("Conexión autenticada con {peer_ip}. Esperando control...");
        control.set_phase("connected");
        let injector = OmarchyInjector {
            input: VirtualInput::connect()?,
            started: Instant::now(),
        };
        let result = run_receiver_with_first_until(&mut channel, injector, Some(first), || {
            control.revoked_active()
        });
        control.set_phase("listening");
        control.set_peer("");
        result?;
        println!("Control terminado; teclado y botones liberados.");
        Ok(())
    }

    enum AttemptError {
        Retry(Box<dyn Error>),
        Stop(Box<dyn Error>),
    }

    fn frame_attempt(error: FrameError) -> AttemptError {
        if matches!(error, FrameError::Io(_)) {
            AttemptError::Retry(Box::new(error))
        } else {
            AttemptError::Stop(Box::new(error))
        }
    }

    async fn connect_once(
        address: SocketAddr,
        edge: Option<Edge>,
        identity: &Identity,
        config: &std::path::Path,
        control: &ControlHandle,
    ) -> Result<(), AttemptError> {
        let peers = config.join("peers");
        let pinned =
            load_peer_key(&peers, address.ip()).map_err(|e| AttemptError::Stop(Box::new(e)))?;
        let stream = TcpStream::connect_timeout(&address, Duration::from_secs(5))
            .map_err(|e| AttemptError::Retry(Box::new(e)))?;
        stream
            .set_nodelay(true)
            .map_err(|e| AttemptError::Retry(Box::new(e)))?;
        stream
            .set_read_timeout(Some(PAIRING_SOCKET_TIMEOUT))
            .map_err(|e| AttemptError::Retry(Box::new(e)))?;
        stream
            .set_write_timeout(Some(PAIRING_SOCKET_TIMEOUT))
            .map_err(|e| AttemptError::Retry(Box::new(e)))?;
        let (mut channel, peer) =
            SecureChannel::connect(stream, Role::Initiator, identity, pinned.as_ref(), |peer| {
                !is_revoked(&peers, &peer.public_key).unwrap_or(true) && control.confirm_pair(peer)
            })
            .map_err(|error| match error {
                SecureError::Io(_) => AttemptError::Retry(Box::new(error)),
                _ => AttemptError::Stop(Box::new(error)),
            })?;
        if is_revoked(&peers, &peer.public_key).map_err(|e| AttemptError::Stop(Box::new(e)))? {
            return Err(AttemptError::Stop("peer identity has been revoked".into()));
        }
        channel
            .stream_mut()
            .set_read_timeout(Some(Duration::from_secs(120)))
            .map_err(|e| AttemptError::Retry(Box::new(e)))?;
        channel
            .stream_mut()
            .set_write_timeout(Some(Duration::from_secs(120)))
            .map_err(|e| AttemptError::Retry(Box::new(e)))?;
        control.set_peer(&address.ip().to_string());
        Frame {
            kind: Kind::Hello,
            epoch: 0,
            sequence: 0,
            payload: b"seamlesscontrol/1".to_vec(),
        }
        .write_to(&mut channel)
        .map_err(frame_attempt)?;
        let greeting = Frame::read_from(&mut channel).map_err(frame_attempt)?;
        if greeting.kind != Kind::Hello || greeting.payload != b"seamlesscontrol/1" {
            return Err(AttemptError::Stop("incompatible peer protocol".into()));
        }
        remember_peer_key(&peers, address.ip(), &peer.public_key)
            .map_err(|e| AttemptError::Stop(Box::new(e)))?;
        channel
            .stream_mut()
            .set_write_timeout(Some(Duration::from_secs(5)))
            .map_err(|e| AttemptError::Retry(Box::new(e)))?;
        if let Some(edge) = edge {
            println!("Conexión autenticada con {address}.");
            capture_loop(channel, edge, control.clone())
                .await
                .map_err(|error| {
                    if error
                        .downcast_ref::<FrameError>()
                        .is_some_and(|frame| matches!(frame, FrameError::Io(_)))
                    {
                        AttemptError::Retry(error)
                    } else {
                        AttemptError::Stop(error)
                    }
                })
        } else {
            Frame {
                kind: Kind::Control,
                epoch: 0,
                sequence: 0,
                payload: b"PAIR".to_vec(),
            }
            .write_to(&mut channel)
            .map_err(frame_attempt)?;
            let reply = Frame::read_from(&mut channel).map_err(frame_attempt)?;
            if reply.kind != Kind::Control || reply.payload != b"PAIRED" {
                return Err(AttemptError::Stop(
                    "the peer did not acknowledge pairing".into(),
                ));
            }
            println!("Par {address} emparejado sin iniciar la captura.");
            Ok(())
        }
    }

    pub async fn run() -> Result<(), Box<dyn Error>> {
        let args: Vec<String> = std::env::args().collect();
        if args.len() >= 2 && args[1] == "topology" {
            let config = config_dir()?;
            let path = config.join("topology");
            let mut layout = load_topology(&path)?;
            match args.as_slice() {
                [_, command] if command == "topology" => {}
                [_, command, action, who, column, row]
                    if command == "topology" && action == "set" =>
                {
                    let machine = if who == "local" {
                        Machine::Local
                    } else {
                        let address: IpAddr = who.parse()?;
                        if load_peer_key(&config.join("peers"), address)?.is_none() {
                            return Err("peer must be paired before placement".into());
                        }
                        Machine::Peer(address)
                    };
                    let column: u8 = column.parse()?;
                    let row: u8 = row.parse()?;
                    let slot = Slot::new(column, row).ok_or("slot must be within the 2x2 grid")?;
                    layout.place(machine, slot)?;
                    save_topology(&path, &layout)?;
                }
                [_, command, action, who] if command == "topology" && action == "remove" => {
                    let address: IpAddr = who.parse()?;
                    layout.remove_peer(address);
                    save_topology(&path, &layout)?;
                }
                _ => {
                    return Err(
                        "usage: topology [set <local|IP> <column> <row> | remove <IP>]".into(),
                    );
                }
            }
            for (machine, slot) in layout.positions() {
                let name = match machine {
                    Machine::Local => "local".to_owned(),
                    Machine::Peer(address) => address.to_string(),
                };
                println!("SLOT\t{name}\t{}\t{}", slot.column, slot.row);
            }
            return Ok(());
        }
        if args.len() == 2 && args[1] == "peers" {
            let config = config_dir()?;
            for (address, key) in list_peer_keys(&config.join("peers"))? {
                println!("PEER\t{address}\t{}", key_fingerprint(&key));
            }
            return Ok(());
        }
        if args.len() == 2 && matches!(args[1].as_str(), "status" | "pause" | "resume" | "reject")
            || args.len() == 3 && args[1] == "approve"
        {
            let command = if args[1] == "approve" {
                if args[2].len() != 6 || !args[2].bytes().all(|byte| byte.is_ascii_digit()) {
                    return Err("approval code must be six digits".into());
                }
                format!("approve {}", args[2])
            } else {
                args[1].clone()
            };
            let response = control::request(&command)?;
            print!("{response}");
            if response.starts_with("ERR") {
                return Err("local control command failed".into());
            }
            return Ok(());
        }
        if args.len() == 3 && args[1] == "revoke" {
            let address: IpAddr = args[2].parse()?;
            let command = format!("revoke {address}");
            match control::request(&command) {
                Ok(response) => {
                    print!("{response}");
                    if response.starts_with("ERR") {
                        return Err("revocation failed".into());
                    }
                }
                Err(error)
                    if matches!(
                        error.kind(),
                        io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused
                    ) =>
                {
                    let config = config_dir()?;
                    revoke_peer_key(&config.join("peers"), address)?;
                    println!("Par {address} revocado.");
                }
                Err(error) => return Err(error.into()),
            }
            let path = config_dir()?.join("topology");
            let mut layout = load_topology(&path)?;
            layout.remove_peer(address);
            save_topology(&path, &layout)?;
            return Ok(());
        }
        if !(args.len() == 3 && matches!(args[1].as_str(), "serve" | "pair" | "connect")
            || args.len() == 4 && args[1] == "connect")
        {
            eprintln!(
                "Uso: seamlesscontrold serve <IP-LAN:PUERTO>\n     seamlesscontrold pair <IP-LAN:PUERTO>\n     seamlesscontrold connect <IP-LAN:PUERTO> [left|right|top|bottom]"
            );
            return Err("invalid arguments".into());
        }
        let address: SocketAddr = args[2].parse()?;
        if !local_address(address.ip()) {
            return Err("listen address must be loopback, link-local or private LAN".into());
        }
        let config = config_dir()?;
        let identity = load_or_create_identity(&config.join("identity"))?;
        println!("Identidad local: {}", key_fingerprint(&identity.public));
        if args[1] == "connect" || args[1] == "pair" {
            let _local_control = ControlServer::start("connect", &config)?;
            let control = _local_control.handle();
            let edge = if args[1] == "connect" && args.len() == 4 {
                Some(Edge::parse(&args[3])?)
            } else if args[1] == "connect" {
                let topology = load_topology(&config.join("topology"))?;
                Some(Edge::from_logical(topology.edge_to(address.ip())?))
            } else {
                None
            };
            let mut delay = Duration::from_secs(1);
            loop {
                match connect_once(address, edge, &identity, &config, &control).await {
                    Ok(()) => return Ok(()),
                    Err(AttemptError::Stop(error)) => return Err(error),
                    Err(AttemptError::Retry(error)) if edge.is_none() => return Err(error),
                    Err(AttemptError::Retry(error)) => {
                        if control.phase() == "disconnected" {
                            delay = Duration::from_secs(1);
                        }
                        eprintln!(
                            "Conexión interrumpida: {error}. Reintentando en {} s.",
                            delay.as_secs()
                        );
                        control.set_peer("");
                        control.set_phase("reconnecting");
                        tokio::select! {
                            _ = tokio::signal::ctrl_c() => return Ok(()),
                            _ = tokio::time::sleep(delay) => {}
                        }
                        delay = delay.saturating_mul(2).min(Duration::from_secs(30));
                    }
                }
            }
        }
        let _local_control = ControlServer::start("serve", &config)?;
        let control = _local_control.handle();
        let listener = TcpListener::bind(address)?;
        listener.set_nonblocking(true)?;
        let listener = tokio::net::TcpListener::from_std(listener)?;
        println!("SeamlessControl escucha en {address}");
        let stop = tokio::signal::ctrl_c();
        tokio::pin!(stop);
        loop {
            let incoming = tokio::select! {
                _ = &mut stop => break,
                incoming = listener.accept() => incoming,
            };
            let (stream, address) = match incoming {
                Ok(value) => value,
                Err(error) => {
                    eprintln!("Error al aceptar conexión: {error}");
                    continue;
                }
            };
            let ip = address.ip();
            if !local_address(ip) {
                eprintln!("Se rechazó una conexión fuera de la red local: {ip}");
                continue;
            }
            let stream = stream.into_std()?;
            stream.set_nonblocking(false)?;
            let shutdown = stream.try_clone()?;
            let identity = identity.clone();
            let config = config.clone();
            let connection_control = control.clone();
            let mut worker = tokio::task::spawn_blocking(move || {
                serve_connection(stream, ip, &identity, &config, &connection_control)
                    .map_err(|error| error.to_string())
            });
            let result = tokio::select! {
                _ = &mut stop => {
                    control.cancel_pair();
                    let _ = shutdown.shutdown(Shutdown::Both);
                    let _ = worker.await;
                    break;
                }
                result = &mut worker => result,
            };
            if let Err(error) = result? {
                eprintln!("Conexión con {ip} terminada: {error}");
            }
            control.set_phase("listening");
            control.set_peer("");
        }
        Ok(())
    }
}

#[cfg(target_os = "linux")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    linux::run().await
}

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("El receptor Omarchy sólo se compila en Linux por ahora.");
}
