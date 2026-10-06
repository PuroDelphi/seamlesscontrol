#[cfg(target_os = "linux")]
mod linux {
    use ashpd::desktop::Session;
    use ashpd::desktop::file_chooser::SelectedFiles;
    use ashpd::desktop::input_capture::{
        ActivatedBarrier, Barrier, BarrierID, BarrierPosition, Capabilities, CreateSessionOptions,
        InputCapture, ReleaseOptions,
    };
    use futures_util::StreamExt;
    use reis::{
        ei,
        event::{DeviceCapability, EiEvent},
    };
    use seamlesscontrol_core::clipboard::{
        ClipboardEvent, ClipboardPacket, ClipboardSync, MeshClipboard,
    };
    use seamlesscontrol_core::clipboard_file;
    use seamlesscontrol_core::clipboard_omarchy::{
        self, ClipboardWatch, spawn_apply_events, spawn_apply_worker,
    };
    use seamlesscontrol_core::control::{self, ControlHandle, ControlServer};
    use seamlesscontrol_core::discovery::{self, ServiceAdvertisement};
    use seamlesscontrol_core::file_session;
    use seamlesscontrol_core::handoff::HandoffCoordinator;
    use seamlesscontrol_core::hypr_ipc::{HyprIpc, SessionLockState};
    use seamlesscontrol_core::omarchy::VirtualInput;
    use seamlesscontrol_core::protocol::{
        AGENT_PROTOCOL, EntryPosition, Frame, FrameError, Kind, ReturnRequest, SwitchRequest,
    };
    use seamlesscontrol_core::receiver::{Injector, run_receiver_with_first_until};
    use seamlesscontrol_core::secure::{
        Identity, Role, SecureChannel, SecureError, SecureWriter, negotiate_pairing,
    };
    use seamlesscontrol_core::state::InputEvent;
    use seamlesscontrol_core::storage::{
        is_revoked, key_fingerprint, list_peer_keys, load_or_create_identity, load_peer_key,
        load_topology, reapprove_peer_key, relocate_peer_key, remember_peer_key, revoke_peer_key,
        rotate_identity, save_topology,
    };
    use seamlesscontrol_core::topology::{
        Edge as LogicalEdge, EdgeReturnDetector, Machine, Rect, Slot, clear_of_external_edge,
        edge_entry_point, edge_fraction, edge_release_point, external_barriers,
    };
    use std::collections::{BTreeMap, BTreeSet, HashMap};
    use std::error::Error;
    use std::future::Future;
    use std::io::{self, Write};
    use std::net::{IpAddr, Ipv4Addr, Shutdown, SocketAddr, TcpListener, TcpStream};
    use std::os::unix::ffi::OsStringExt;
    use std::os::unix::net::UnixStream;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    const PAIRING_SOCKET_TIMEOUT: Duration = Duration::from_secs(330);
    const LATENCY_SAMPLES: u64 = 20;
    const MAX_INBOUND_CONNECTIONS: usize = 8;

    fn portal_file_path(uri: &str) -> Result<PathBuf, Box<dyn Error>> {
        let encoded = uri
            .strip_prefix("file://")
            .ok_or("the file chooser returned a non-local URI")?;
        let encoded = encoded.strip_prefix("localhost").unwrap_or(encoded);
        if !encoded.starts_with('/') || encoded.contains(['?', '#']) {
            return Err("the file chooser returned an invalid local URI".into());
        }
        let mut decoded = Vec::with_capacity(encoded.len());
        let mut bytes = encoded.as_bytes().iter().copied();
        while let Some(byte) = bytes.next() {
            if byte == b'%' {
                let digit = |value: u8| match value {
                    b'0'..=b'9' => Some(value - b'0'),
                    b'A'..=b'F' => Some(value - b'A' + 10),
                    b'a'..=b'f' => Some(value - b'a' + 10),
                    _ => None,
                };
                let high = bytes
                    .next()
                    .and_then(digit)
                    .ok_or("invalid file URI escape")?;
                let low = bytes
                    .next()
                    .and_then(digit)
                    .ok_or("invalid file URI escape")?;
                decoded.push(high << 4 | low);
            } else {
                decoded.push(byte);
            }
        }
        if decoded.contains(&0) {
            return Err("file URI contains a NUL byte".into());
        }
        Ok(PathBuf::from(std::ffi::OsString::from_vec(decoded)))
    }

    async fn choose_local_path(directory: bool, spanish: bool) -> Result<(), Box<dyn Error>> {
        let title = match (directory, spanish) {
            (false, false) => "Choose a file for SeamlessControl",
            (false, true) => "Elija un archivo para SeamlessControl",
            (true, false) => "Choose a destination folder for SeamlessControl",
            (true, true) => "Elija una carpeta de destino para SeamlessControl",
        };
        let request = SelectedFiles::open_file()
            .title(title)
            .directory(directory)
            .send()
            .await?;
        let selected = match request.response() {
            Ok(selected) => selected,
            Err(ashpd::Error::Response(ashpd::desktop::ResponseError::Cancelled)) => {
                return Ok(());
            }
            Err(error) => return Err(error.into()),
        };
        let uri = selected
            .uris()
            .first()
            .ok_or("the file chooser returned no path")?;
        let path = portal_file_path(uri.as_str())?;
        if !(if directory {
            path.is_dir()
        } else {
            path.is_file()
        }) {
            return Err("the selected path is not accessible".into());
        }
        let path = path.to_str().ok_or("the selected path is not UTF-8")?;
        println!("{}", serde_json::to_string(path)?);
        Ok(())
    }

    #[derive(Clone, Copy)]
    enum Edge {
        Left,
        Right,
        Top,
        Bottom,
    }

    #[derive(Clone, Copy)]
    enum ConnectionMode {
        Pair,
        Capture(Edge),
        CaptureMapped(Edge),
        Latency,
    }

    #[derive(Debug)]
    struct TopologyChanged;

    impl std::fmt::Display for TopologyChanged {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "topology changed; rebuilding capture")
        }
    }

    impl Error for TopologyChanged {}

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

        fn logical(self) -> LogicalEdge {
            match self {
                Self::Left => LogicalEdge::Left,
                Self::Right => LogicalEdge::Right,
                Self::Top => LogicalEdge::Top,
                Self::Bottom => LogicalEdge::Bottom,
            }
        }

        fn release_position(self, position: (f32, f32)) -> (f64, f64) {
            let (x, y) = (position.0 as f64, position.1 as f64);
            match self {
                Self::Left => (x + 32.0, y),
                Self::Right => (x - 32.0, y),
                Self::Top => (x, y + 32.0),
                Self::Bottom => (x, y - 32.0),
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

    async fn bind_initial_eis_seat(
        context: &ei::Context,
        events: &mut reis::tokio::EiConvertEventStream,
    ) -> Result<(), Box<dyn Error>> {
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                match events.next().await {
                    Some(Ok(EiEvent::SeatAdded(seat))) => {
                        seat.seat.bind_capabilities(
                            DeviceCapability::Pointer
                                | DeviceCapability::Keyboard
                                | DeviceCapability::Scroll
                                | DeviceCapability::Button,
                        );
                        context.flush()?;
                        return Ok::<_, Box<dyn Error>>(());
                    }
                    Some(Ok(_)) => {}
                    Some(Err(error)) => return Err(error.into()),
                    None => return Err("EIS ended before announcing an input seat".into()),
                }
            }
        })
        .await
        .map_err(|_| "EIS did not announce an input seat in five seconds")?
    }

    fn send_frame(
        channel: &mut impl Write,
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

    async fn install_barriers(
        portal: &InputCapture,
        session: &Session<InputCapture>,
        edge: Edge,
    ) -> Result<u32, Box<dyn Error>> {
        let zones = portal
            .zones(session, Default::default())
            .await?
            .response()?;
        let regions: Vec<Rect> = zones
            .regions()
            .iter()
            .map(|region| {
                let width = i32::try_from(region.width())?;
                let height = i32::try_from(region.height())?;
                Rect::new(region.x_offset(), region.y_offset(), width, height)
                    .ok_or("invalid monitor region")
                    .map_err(Into::into)
            })
            .collect::<Result<_, Box<dyn Error>>>()?;
        let segments = external_barriers(&regions, edge.logical());
        if segments.is_empty() {
            return Err("the portal returned no exposed monitor edge".into());
        }
        let barriers: Vec<Barrier> = segments
            .into_iter()
            .enumerate()
            .map(|(index, segment)| {
                let id = BarrierID::new(u32::try_from(index + 1)?).ok_or("invalid barrier ID")?;
                Ok(Barrier::new(
                    id,
                    BarrierPosition::new(segment.x1, segment.y1, segment.x2, segment.y2),
                ))
            })
            .collect::<Result<_, Box<dyn Error>>>()?;
        let response = portal
            .set_pointer_barriers(session, &barriers, zones.zone_set(), Default::default())
            .await?
            .response()?;
        if !response.failed_barriers().is_empty() {
            return Err("the compositor rejected one or more selected edges".into());
        }
        Ok(zones.zone_set())
    }

    async fn install_mesh_barriers(
        portal: &InputCapture,
        session: &Session<InputCapture>,
        topology: &seamlesscontrol_core::topology::Topology,
    ) -> Result<(u32, BTreeMap<u32, (IpAddr, Edge)>), Box<dyn Error>> {
        let zones = portal
            .zones(session, Default::default())
            .await?
            .response()?;
        let regions: Vec<Rect> = zones
            .regions()
            .iter()
            .map(|region| {
                Rect::new(
                    region.x_offset(),
                    region.y_offset(),
                    i32::try_from(region.width())?,
                    i32::try_from(region.height())?,
                )
                .ok_or("invalid monitor region")
                .map_err(Into::into)
            })
            .collect::<Result<_, Box<dyn Error>>>()?;
        let mut barriers = Vec::new();
        let mut targets = BTreeMap::new();
        for edge in [
            LogicalEdge::Left,
            LogicalEdge::Right,
            LogicalEdge::Top,
            LogicalEdge::Bottom,
        ] {
            let Some(Machine::Peer(peer)) = topology.neighbor(Machine::Local, edge) else {
                continue;
            };
            for segment in external_barriers(&regions, edge) {
                let number = u32::try_from(barriers.len() + 1)?;
                let id = BarrierID::new(number).ok_or("invalid barrier ID")?;
                barriers.push(Barrier::new(
                    id,
                    BarrierPosition::new(segment.x1, segment.y1, segment.x2, segment.y2),
                ));
                targets.insert(number, (peer, Edge::from_logical(edge)));
            }
        }
        if barriers.is_empty() {
            return Err("the topology has no exposed edge to a neighboring peer".into());
        }
        let response = portal
            .set_pointer_barriers(session, &barriers, zones.zone_set(), Default::default())
            .await?
            .response()?;
        if !response.failed_barriers().is_empty() {
            return Err("the compositor rejected one or more mesh edges".into());
        }
        Ok((zones.zone_set(), targets))
    }

    struct MeshLink {
        writer: SecureWriter<TcpStream>,
        peer_key: [u8; 32],
        sequence: u64,
        reader: Option<thread::JoinHandle<()>>,
    }

    enum MeshSession {
        Stopped,
        Retry(Box<dyn Error>, Duration),
    }

    impl MeshLink {
        fn send(&mut self, kind: Kind, epoch: u64, payload: Vec<u8>) -> Result<(), Box<dyn Error>> {
            send_frame(&mut self.writer, kind, epoch, &mut self.sequence, payload)
        }
    }

    fn connect_mesh_peer(
        address: SocketAddr,
        identity: &Identity,
        config: &std::path::Path,
        feedback: tokio::sync::mpsc::Sender<(IpAddr, [u8; 32], Result<Frame, FrameError>)>,
    ) -> Result<MeshLink, Box<dyn Error>> {
        let peers = config.join("peers");
        let pinned = load_peer_key(&peers, address.ip())?
            .ok_or("every mesh peer must be paired before capture")?;
        let stream = TcpStream::connect_timeout(&address, Duration::from_secs(5))?;
        stream.set_nodelay(true)?;
        stream.set_read_timeout(Some(Duration::from_secs(5)))?;
        stream.set_write_timeout(Some(Duration::from_secs(5)))?;
        let (mut channel, peer) =
            SecureChannel::connect(stream, Role::Initiator, identity, Some(&pinned), |_| false)?;
        if is_revoked(&peers, &peer.public_key)? {
            return Err("mesh peer identity has been revoked".into());
        }
        Frame {
            kind: Kind::Hello,
            epoch: 0,
            sequence: 0,
            payload: AGENT_PROTOCOL.to_vec(),
        }
        .write_to(&mut channel)?;
        let greeting = Frame::read_from(&mut channel)?;
        if greeting.kind != Kind::Hello || greeting.payload != AGENT_PROTOCOL {
            return Err("incompatible mesh peer protocol".into());
        }
        Frame {
            kind: Kind::Control,
            epoch: 0,
            sequence: 0,
            payload: b"CLAIM-MESH".to_vec(),
        }
        .write_to(&mut channel)?;
        let response = Frame::read_from(&mut channel)?;
        if response.kind != Kind::Control || response.payload != b"READY" {
            return Err("mesh peer did not grant exclusive input claim".into());
        }
        channel.stream_mut().set_read_timeout(None)?;
        channel
            .stream_mut()
            .set_write_timeout(Some(Duration::from_secs(5)))?;
        let (mut reader, writer) = channel.into_tcp_halves()?;
        let peer_ip = address.ip();
        let peer_key = peer.public_key;
        let reader = thread::spawn(move || {
            loop {
                let frame = Frame::read_from(&mut reader);
                let done = frame.is_err();
                if feedback.blocking_send((peer_ip, peer_key, frame)).is_err() || done {
                    break;
                }
            }
        });
        let mut link = MeshLink {
            writer,
            peer_key: peer.public_key,
            sequence: 0,
            reader: Some(reader),
        };
        link.send(Kind::Heartbeat, 0, Vec::new())?;
        Ok(link)
    }

    fn next_epoch(previous: u64) -> Result<u64, Box<dyn Error>> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos() as u64;
        let after_previous = previous.checked_add(1).ok_or("input epoch exhausted")?;
        Ok(now.max(after_previous).max(1))
    }

    fn crossing_fraction(ipc: &HyprIpc, edge: LogicalEdge, position: Option<(f32, f32)>) -> u16 {
        match (ipc.monitor_rects(), position) {
            (Ok(regions), Some((x, y))) if x.is_finite() && y.is_finite() => {
                edge_fraction(&regions, edge, x.round() as i32, y.round() as i32)
                    .unwrap_or(u16::MAX / 2)
            }
            _ => u16::MAX / 2,
        }
    }

    fn return_position(ipc: &HyprIpc, edge: LogicalEdge, fraction: u16) -> Option<(f64, f64)> {
        let regions = ipc.monitor_rects().ok()?;
        let (x, y) = edge_release_point(&regions, edge, fraction)?;
        Some((f64::from(x), f64::from(y)))
    }

    fn ready_to_rearm(ipc: &HyprIpc, edge: LogicalEdge) -> bool {
        let Ok(regions) = ipc.monitor_rects() else {
            return false;
        };
        let Ok((x, y)) = ipc.cursor_position() else {
            return false;
        };
        clear_of_external_edge(&regions, edge, x, y)
    }

    fn remember_held(event: &InputEvent, keys: &mut BTreeSet<u32>, buttons: &mut BTreeSet<u32>) {
        match event {
            InputEvent::KeyDown(key) => {
                keys.insert(*key);
            }
            InputEvent::KeyUp(key) => {
                keys.remove(key);
            }
            InputEvent::ButtonDown(button) => {
                buttons.insert(*button);
            }
            InputEvent::ButtonUp(button) => {
                buttons.remove(button);
            }
            _ => {}
        }
    }

    fn mesh_link(
        links: &mut BTreeMap<IpAddr, MeshLink>,
        peer: IpAddr,
    ) -> Result<&mut MeshLink, Box<dyn Error>> {
        links
            .get_mut(&peer)
            .ok_or_else(|| "mesh peer is not connected".into())
    }

    fn reconcile_mesh_links(
        links: &mut BTreeMap<IpAddr, MeshLink>,
        peers_dir: &std::path::Path,
        coordinator: &HandoffCoordinator,
    ) -> Result<(), Box<dyn Error>> {
        let mut removed = Vec::new();
        for (&address, link) in links.iter() {
            if load_peer_key(peers_dir, address)? != Some(link.peer_key)
                || is_revoked(peers_dir, &link.peer_key)?
            {
                removed.push(address);
            }
        }
        let mut active_revoked = false;
        for address in removed {
            if let Some(mut link) = links.remove(&address) {
                let _ = link.writer.stream_mut().shutdown(Shutdown::Both);
                // The reader reports through feedback_rx. It must not be joined
                // here because it may be waiting for space in that channel.
            }
            if coordinator.owner() == Machine::Peer(address)
                || coordinator.pending().is_some_and(|pending| {
                    pending.from == address || pending.target == Machine::Peer(address)
                })
            {
                active_revoked = true;
            }
        }
        if active_revoked {
            return Err("active mesh peer was revoked; releasing capture".into());
        }
        Ok(())
    }

    async fn mesh_loop(
        port: u16,
        identity: &Identity,
        config: &std::path::Path,
        control: ControlHandle,
    ) -> Result<MeshSession, Box<dyn Error>> {
        let topology = load_topology(&config.join("topology"))?;
        let peers: Vec<IpAddr> = topology
            .positions()
            .filter_map(|(machine, _)| match machine {
                Machine::Peer(peer) => Some(peer),
                Machine::Local => None,
            })
            .collect();
        if peers.is_empty() {
            return Err("place at least one paired peer in the topology".into());
        }
        for peer in &peers {
            if !local_address(*peer) || load_peer_key(&config.join("peers"), *peer)?.is_none() {
                return Err(format!("mesh peer {peer} must have a paired LAN identity").into());
            }
        }
        let lock_ipc =
            HyprIpc::from_env().ok_or("Hyprland IPC is required to guard mesh capture")?;
        let mut locked = lock_ipc
            .session_lock_state()
            .unwrap_or(SessionLockState::Undetermined)
            != SessionLockState::Unlocked;
        let portal = InputCapture::new().await?;
        let (session, _) = portal
            .create_session(
                None,
                CreateSessionOptions::default()
                    .set_capabilities(Capabilities::Keyboard | Capabilities::Pointer),
            )
            .await?;
        let mut zones_changed = portal.receive_zones_changed().await?;
        let (mut zone_set, mut barriers) =
            install_mesh_barriers(&portal, &session, &topology).await?;
        let eis = portal.connect_to_eis(&session, Default::default()).await?;
        let stream = UnixStream::from(eis);
        stream.set_nonblocking(true)?;
        let context = ei::Context::new(stream)?;
        context.flush()?;
        let (_connection, mut events) = context
            .handshake_tokio("seamlesscontrol", ei::handshake::ContextType::Receiver)
            .await?;
        bind_initial_eis_seat(&context, &mut events).await?;
        let mut activated = portal.receive_activated().await?;
        let mut deactivated = portal.receive_deactivated().await?;
        if !locked {
            portal.enable(&session, Default::default()).await?;
        }
        control.set_phase(if locked { "locked" } else { "ready" });
        println!(
            "Malla activa para {} pares. Cruce un borde; Escape devuelve el control.",
            peers.len()
        );
        let started = Instant::now();

        let (feedback_tx, mut feedback_rx) =
            tokio::sync::mpsc::channel::<(IpAddr, [u8; 32], Result<Frame, FrameError>)>(32);
        let mut clipboard_mesh = MeshClipboard::new(identity.public);
        let (clipboard_apply, clipboard_apply_worker) = spawn_apply_events();
        let (clipboard_tx, mut clipboard_rx) = tokio::sync::mpsc::channel::<ClipboardEvent>(8);
        let clipboard_watch = match ClipboardWatch::start_events(move |event| {
            let _ = clipboard_tx.blocking_send(event);
        }) {
            Ok(watch) => Some(watch),
            Err(error) => {
                eprintln!("SeamlessControl: observación del portapapeles no disponible: {error}");
                None
            }
        };
        let mut links = BTreeMap::<IpAddr, MeshLink>::new();
        let peers_dir = config.join("peers");
        let mut coordinator = HandoffCoordinator::new(topology.clone());
        let mut heartbeat = tokio::time::interval(Duration::from_secs(2));
        let mut topology_check = tokio::time::interval(Duration::from_secs(1));
        let mut pause_tick = tokio::time::interval(Duration::from_millis(100));
        pause_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut capture_enabled = !locked;
        let mut current_activation = None;
        let mut release_position = None;
        let mut portal_active = false;
        let mut last_epoch = 0_u64;
        let mut keys = BTreeSet::new();
        let mut buttons = BTreeSet::new();
        let mut pending_since: Option<Instant> = None;
        let mut pending_entry: Option<EntryPosition> = None;
        let result: Result<(), Box<dyn Error>> = async {
            loop {
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => break,
                    _ = topology_check.tick() => {
                        if load_topology(&config.join("topology"))? != topology {
                            return Err("mesh topology changed; rebuilding capture".into());
                        }
                    }
                    _ = pause_tick.tick() => {
                        reconcile_mesh_links(&mut links, &peers_dir, &coordinator)?;
                        if control.revoked_active() { return Err("active mesh peer was revoked".into()); }
                        locked = lock_ipc.session_lock_state().unwrap_or(SessionLockState::Undetermined) != SessionLockState::Unlocked;
                        if pending_since.is_some_and(|since| since.elapsed() > Duration::from_secs(2)) {
                            return Err("mesh handoff acknowledgement timed out".into());
                        }
                        if control.paused() || locked {
                            if let (Machine::Peer(peer), Some(epoch)) = (coordinator.owner(), coordinator.epoch()) {
                                let _ = mesh_link(&mut links, peer)?.send(Kind::Control, epoch, b"END".to_vec());
                                coordinator.reset_local();
                                keys.clear(); buttons.clear(); pending_since = None; pending_entry = None;
                                let mut options = ReleaseOptions::default().set_activation_id(current_activation.take());
                                if let Some(position) = release_position { options = options.set_cursor_position(position); }
                                portal.release(&session, options).await?;
                                portal_active = false;
                            }
                            if capture_enabled { portal.disable(&session, Default::default()).await?; capture_enabled = false; }
                            control.set_phase(if locked { "locked" } else { "paused" });
                        } else if !capture_enabled {
                            portal.enable(&session, Default::default()).await?;
                            capture_enabled = true;
                            control.set_phase("ready");
                        }
                    }
                    _ = heartbeat.tick() => {
                        reconcile_mesh_links(&mut links, &peers_dir, &coordinator)?;
                        for link in links.values_mut() { link.send(Kind::Heartbeat, 0, Vec::new())?; }
                    }
                    Some(event) = clipboard_rx.recv() => {
                        reconcile_mesh_links(&mut links, &peers_dir, &coordinator)?;
                        if !locked && let Some(packet) = clipboard_mesh.local_changed(event)? {
                            for link in links.values_mut() {
                                link.send(Kind::Clipboard, 0, packet.encode())?;
                            }
                        }
                    }
                    feedback = feedback_rx.recv() => {
                        reconcile_mesh_links(&mut links, &peers_dir, &coordinator)?;
                        let (from, from_key, frame) = feedback.ok_or("mesh feedback channel closed")?;
                        // A removed reader can still have a queued frame. Never
                        // process it after revocation or re-pairing at that IP.
                        if !links.get(&from).is_some_and(|link| link.peer_key == from_key) { continue; }
                        let frame = frame?;
                        if frame.kind == Kind::Clipboard {
                            let expected = mesh_link(&mut links, from)?.peer_key;
                            let packet = ClipboardPacket::decode(&frame.payload)?;
                            if !locked && let Some(relay) = clipboard_mesh.peer_changed(expected, packet)? {
                                clipboard_apply.try_send(relay.event.clone()).map_err(|_| "mesh clipboard apply queue is full")?;
                                for link in links.values_mut() {
                                    link.send(Kind::Clipboard, 0, relay.encode())?;
                                }
                            }
                            continue;
                        }
                        if frame.kind != Kind::Control { return Err("unexpected mesh feedback".into()); }
                        if frame.payload == b"ENDED" {
                            if !coordinator.pending().is_some_and(|pending| pending.from == from && pending.epoch == frame.epoch) {
                                continue;
                            }
                            let entry = pending_entry.ok_or("mesh handoff has no entry edge")?;
                            let next = next_epoch(last_epoch)?;
                            let target = coordinator.acknowledge(from, frame.epoch, next)?;
                            pending_entry = None;
                            pending_since = None;
                            match target {
                                Machine::Peer(peer) => {
                                    last_epoch = next;
                                    let link = mesh_link(&mut links, peer)?;
                                    link.send(Kind::Control, next, entry.begin_payload())?;
                                    for key in &keys { link.send(Kind::Input, next, InputEvent::KeyDown(*key).encode())?; }
                                    for button in &buttons { link.send(Kind::Input, next, InputEvent::ButtonDown(*button).encode())?; }
                                    control.set_peer(&peer.to_string());
                                    control.set_phase("controlling");
                                    println!("Control transferido a {peer}.");
                                }
                                Machine::Local => {
                                    keys.clear(); buttons.clear();
                                    let mut options = ReleaseOptions::default().set_activation_id(current_activation.take());
                                    if let Some(position) = release_position { options = options.set_cursor_position(position); }
                                    portal.release(&session, options).await?;
                                    portal_active = false;
                                    control.set_peer(""); control.set_phase("ready");
                                }
                            }
                        } else if frame.payload == b"RETURN" || frame.payload.starts_with(b"RETURN\t") {
                            if coordinator.owner() == Machine::Peer(from) && coordinator.epoch() == Some(frame.epoch) {
                                if frame.payload != b"RETURN" {
                                    let request = ReturnRequest::parse(&frame.payload)?;
                                    if topology.neighbor(Machine::Peer(from), request.exit_edge) != Some(Machine::Local) {
                                        return Err("mesh return edge does not match topology".into());
                                    }
                                    release_position = return_position(&lock_ipc, request.exit_edge.opposite(), request.fraction)
                                        .or(release_position);
                                }
                                mesh_link(&mut links, from)?.send(Kind::Control, frame.epoch, b"END".to_vec())?;
                                coordinator.reset_local(); pending_since = None; pending_entry = None; keys.clear(); buttons.clear();
                                let mut options = ReleaseOptions::default().set_activation_id(current_activation.take());
                                if let Some(position) = release_position { options = options.set_cursor_position(position); }
                                portal.release(&session, options).await?;
                                portal_active = false;
                                control.set_peer(""); control.set_phase("ready");
                            }
                        } else if frame.payload.starts_with(b"SWITCH\t") {
                            let request = SwitchRequest::parse(&frame.payload)?;
                            let target = request.target;
                            if coordinator.owner() != Machine::Peer(from) || coordinator.epoch() != Some(frame.epoch) {
                                return Err("stale mesh switch request".into());
                            }
                            if topology.neighbor(Machine::Peer(from), request.exit_edge) != Some(Machine::Peer(target)) {
                                return Err("mesh switch edge does not match topology".into());
                            }
                            coordinator.request(from, frame.epoch, Machine::Peer(target))?;
                            pending_entry = Some(EntryPosition {
                                edge: request.exit_edge.opposite(),
                                fraction: request.fraction,
                            });
                            if load_peer_key(&config.join("peers"), target)?.is_none() { return Err("mesh target was revoked".into()); }
                            if let std::collections::btree_map::Entry::Vacant(entry) = links.entry(target) {
                                let address = SocketAddr::new(target, port);
                                let mut link = connect_mesh_peer(address, identity, config, feedback_tx.clone())?;
                                if let Some(packet) = clipboard_mesh.latest() {
                                    link.send(Kind::Clipboard, 0, packet.encode())?;
                                }
                                entry.insert(link);
                            }
                            mesh_link(&mut links, from)?.send(Kind::Control, frame.epoch, b"RELEASE".to_vec())?;
                            pending_since = Some(Instant::now());
                            control.set_phase("handoff");
                        } else { return Err("unknown mesh feedback control".into()); }
                    }
                    signal = activated.next() => {
                        reconcile_mesh_links(&mut links, &peers_dir, &coordinator)?;
                        let signal = signal.ok_or("capture activation stream closed")?;
                        let target = match signal.barrier_id() {
                            Some(ActivatedBarrier::Barrier(id)) => barriers.get(&id.get()).copied(),
                            _ => None,
                        };
                        let Some((peer, edge)) = target else {
                            portal.release(&session, ReleaseOptions::default().set_activation_id(signal.activation_id())).await?;
                            continue;
                        };
                        if !capture_enabled || coordinator.owner() != Machine::Local {
                            portal.release(&session, ReleaseOptions::default().set_activation_id(signal.activation_id())).await?;
                            continue;
                        }
                        portal_active = true;
                        current_activation = signal.activation_id();
                        release_position = signal.cursor_position().map(|p| edge.release_position(p));
                        if load_peer_key(&config.join("peers"), peer)?.is_none() { return Err("mesh peer was revoked".into()); }
                        if let std::collections::btree_map::Entry::Vacant(entry) = links.entry(peer) {
                            let mut link = match connect_mesh_peer(SocketAddr::new(peer, port), identity, config, feedback_tx.clone()) {
                                Ok(link) => link,
                                Err(error) => {
                                    let mut options = ReleaseOptions::default().set_activation_id(signal.activation_id());
                                    if let Some(position) = signal.cursor_position() { options = options.set_cursor_position(edge.release_position(position)); }
                                    let _ = portal.release(&session, options).await;
                                    portal_active = false;
                                    return Err(error);
                                }
                            };
                            if let Some(packet) = clipboard_mesh.latest() {
                                link.send(Kind::Clipboard, 0, packet.encode())?;
                            }
                            entry.insert(link);
                        }
                        last_epoch = next_epoch(last_epoch)?;
                        coordinator.activate_local_edge(edge.logical(), peer, last_epoch)?;
                        let entry = EntryPosition {
                            edge: edge.logical().opposite(),
                            fraction: crossing_fraction(&lock_ipc, edge.logical(), signal.cursor_position()),
                        };
                        mesh_link(&mut links, peer)?.send(Kind::Control, last_epoch, entry.begin_payload())?;
                        control.set_peer(&peer.to_string()); control.set_phase("controlling");
                        println!("Control remoto activo en {peer}.");
                    }
                    signal = deactivated.next() => {
                        reconcile_mesh_links(&mut links, &peers_dir, &coordinator)?;
                        if signal.is_none() { return Err("capture deactivation stream closed".into()); }
                        if let (Machine::Peer(peer), Some(epoch)) = (coordinator.owner(), coordinator.epoch()) {
                            mesh_link(&mut links, peer)?.send(Kind::Control, epoch, b"END".to_vec())?;
                        }
                        portal_active = false;
                        coordinator.reset_local(); pending_since = None; pending_entry = None; keys.clear(); buttons.clear(); current_activation = None;
                        control.set_peer(""); control.set_phase("ready");
                    }
                    signal = zones_changed.next() => {
                        reconcile_mesh_links(&mut links, &peers_dir, &coordinator)?;
                        let signal = signal.ok_or("capture zones stream closed")?;
                        if signal.zone_set().is_some_and(|id| id != zone_set) { continue; }
                        if let (Machine::Peer(peer), Some(epoch)) = (coordinator.owner(), coordinator.epoch()) {
                            mesh_link(&mut links, peer)?.send(Kind::Control, epoch, b"END".to_vec())?;
                        }
                        if portal_active {
                            let mut options = ReleaseOptions::default().set_activation_id(current_activation.take());
                            if let Some(position) = release_position { options = options.set_cursor_position(position); }
                            let _ = portal.release(&session, options).await;
                            portal_active = false;
                        }
                        coordinator.reset_local(); pending_since = None; pending_entry = None; keys.clear(); buttons.clear(); current_activation = None;
                        if capture_enabled { portal.disable(&session, Default::default()).await?; capture_enabled = false; }
                        (zone_set, barriers) = install_mesh_barriers(&portal, &session, &topology).await?;
                        if !control.paused() && !locked { portal.enable(&session, Default::default()).await?; capture_enabled = true; }
                        control.set_peer(""); control.set_phase(if locked { "locked" } else if control.paused() { "paused" } else { "ready" });
                    }
                    event = events.next() => {
                        reconcile_mesh_links(&mut links, &peers_dir, &coordinator)?;
                        let event = event.ok_or("EIS event stream closed")??;
                        if let EiEvent::SeatAdded(seat) = &event {
                            seat.seat.bind_capabilities(DeviceCapability::Pointer | DeviceCapability::Keyboard | DeviceCapability::Scroll | DeviceCapability::Button);
                            context.flush()?;
                        }
                        if let (Machine::Peer(peer), Some(epoch)) = (coordinator.owner(), coordinator.epoch()) {
                            if matches!(&event, EiEvent::KeyboardKey(key) if key.key == 1 && key.state == ei::keyboard::KeyState::Press) {
                                mesh_link(&mut links, peer)?.send(Kind::Control, epoch, b"END".to_vec())?;
                                coordinator.reset_local(); pending_since = None; pending_entry = None; keys.clear(); buttons.clear();
                                let mut options = ReleaseOptions::default().set_activation_id(current_activation.take());
                                if let Some(position) = release_position { options = options.set_cursor_position(position); }
                                portal.release(&session, options).await?;
                                portal_active = false;
                                control.set_peer(""); control.set_phase("ready");
                            } else if let Some(event) = input_event(event) {
                                remember_held(&event, &mut keys, &mut buttons);
                                if pending_since.is_none() { mesh_link(&mut links, peer)?.send(Kind::Input, epoch, event.encode())?; }
                            }
                        }
                    }
                }
            }
            Ok(())
        }.await;
        if let (Machine::Peer(peer), Some(epoch)) = (coordinator.owner(), coordinator.epoch())
            && let Ok(link) = mesh_link(&mut links, peer)
        {
            let _ = link.send(Kind::Control, epoch, b"END".to_vec());
        }
        if portal_active {
            let mut options = ReleaseOptions::default().set_activation_id(current_activation);
            if let Some(position) = release_position {
                options = options.set_cursor_position(position);
            }
            let _ = portal.release(&session, options).await;
        }
        let _ = portal.disable(&session, Default::default()).await;
        drop(clipboard_rx);
        if let Some(watch) = clipboard_watch {
            watch.stop();
        }
        drop(feedback_rx);
        for link in links.values_mut() {
            let _ = link.writer.stream_mut().shutdown(Shutdown::Both);
        }
        for (_, mut link) in links {
            if let Some(reader) = link.reader.take() {
                let _ = reader.join();
            }
        }
        drop(clipboard_apply);
        let _ = clipboard_apply_worker.join();
        control.set_peer("");
        control.set_phase("disconnected");
        Ok(match result {
            Ok(()) => MeshSession::Stopped,
            Err(error) => MeshSession::Retry(error, started.elapsed()),
        })
    }

    async fn capture_loop(
        mut channel: SecureChannel<TcpStream>,
        edge: Edge,
        control: ControlHandle,
        local_id: [u8; 32],
        remote_id: [u8; 32],
        topology_watch: Option<(&std::path::Path, IpAddr)>,
    ) -> Result<(), Box<dyn Error>> {
        let local_lock =
            HyprIpc::from_env().ok_or("Hyprland IPC is required to guard local capture")?;
        let mut locked = local_lock
            .session_lock_state()
            .unwrap_or(SessionLockState::Undetermined)
            != SessionLockState::Unlocked;
        let portal = InputCapture::new().await?;
        let (session, _) = portal
            .create_session(
                None,
                CreateSessionOptions::default()
                    .set_capabilities(Capabilities::Keyboard | Capabilities::Pointer),
            )
            .await?;
        let mut zones_changed = portal.receive_zones_changed().await?;
        let mut current_zone_set = install_barriers(&portal, &session, edge).await?;
        let eis = portal.connect_to_eis(&session, Default::default()).await?;
        let stream = UnixStream::from(eis);
        stream.set_nonblocking(true)?;
        let context = ei::Context::new(stream)?;
        context.flush()?;
        let (_connection, mut events) = context
            .handshake_tokio("seamlesscontrol", ei::handshake::ContextType::Receiver)
            .await?;
        bind_initial_eis_seat(&context, &mut events).await?;
        let mut activated = portal.receive_activated().await?;
        let mut deactivated = portal.receive_deactivated().await?;
        if !locked {
            portal.enable(&session, Default::default()).await?;
        }
        control.set_phase(if locked { "locked" } else { "ready" });
        println!(
            "Captura activa. Cruce el borde elegido; Escape recupera el control, Ctrl+C termina."
        );

        channel.stream_mut().set_read_timeout(None)?;
        let (mut reader, mut writer) = channel.into_tcp_halves()?;
        let (feedback_tx, mut feedback_rx) = tokio::sync::mpsc::channel(8);
        let feedback_reader = thread::spawn(move || {
            loop {
                let frame = Frame::read_from(&mut reader);
                let done = frame.is_err();
                if feedback_tx.blocking_send(frame).is_err() || done {
                    break;
                }
            }
        });
        let clipboard_sync = Arc::new(Mutex::new(ClipboardSync::new(local_id, remote_id)));
        let (clipboard_apply, clipboard_apply_worker) =
            spawn_apply_worker(Arc::clone(&clipboard_sync));
        let (clipboard_tx, mut clipboard_rx) = tokio::sync::mpsc::channel(8);
        let clipboard_watch = match ClipboardWatch::start(clipboard_sync, move |event| {
            let _ = clipboard_tx.blocking_send(event);
        }) {
            Ok(watch) => Some(watch),
            Err(error) => {
                eprintln!("SeamlessControl: observación del portapapeles no disponible: {error}");
                None
            }
        };

        let mut heartbeat = tokio::time::interval(Duration::from_secs(2));
        let mut topology_check = tokio::time::interval(Duration::from_secs(1));
        let mut pause_tick = tokio::time::interval(Duration::from_millis(100));
        pause_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut capture_enabled = !locked;
        let mut epoch = 0;
        let mut sequence = 0;
        let mut current_activation = None;
        let mut active = false;
        let mut release_position = None;
        let mut rearm_after_return = false;
        let result: Result<(), Box<dyn Error>> = async {
            loop {
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => break,
                    _ = topology_check.tick(), if topology_watch.is_some() => {
                        let (path, peer) = topology_watch.expect("guarded topology watch");
                        if load_topology(path)?.edge_to(peer).ok() != Some(edge.logical()) {
                            return Err(TopologyChanged.into());
                        }
                    }
                    _ = pause_tick.tick() => {
                        if control.revoked_active() { break; }
                        locked = local_lock.session_lock_state().unwrap_or(SessionLockState::Undetermined) != SessionLockState::Unlocked;
                        if (control.paused() || locked) && capture_enabled {
                            if active {
                                send_frame(&mut writer, Kind::Control, epoch, &mut sequence, b"END".to_vec())?;
                                let mut options = ReleaseOptions::default().set_activation_id(current_activation.take());
                                if let Some(position) = release_position { options = options.set_cursor_position(position); }
                                portal.release(&session, options).await?;
                                active = false;
                            }
                            portal.disable(&session, Default::default()).await?;
                            capture_enabled = false;
                            control.set_phase(if locked { "locked" } else { "paused" });
                        } else if !control.paused() && !locked && !capture_enabled {
                            if rearm_after_return && !ready_to_rearm(&local_lock, edge.logical()) {
                                control.set_phase("rearming");
                            } else {
                                rearm_after_return = false;
                                portal.enable(&session, Default::default()).await?;
                                capture_enabled = true;
                                control.set_phase("ready");
                            }
                        } else if locked {
                            control.set_phase("locked");
                        } else if control.paused() {
                            control.set_phase("paused");
                        }
                    }
                    _ = heartbeat.tick() => {
                        send_frame(&mut writer, Kind::Heartbeat, epoch, &mut sequence, Vec::new())?;
                    }
                    Some(event) = clipboard_rx.recv() => {
                        if !locked { send_frame(&mut writer, Kind::Clipboard, 0, &mut sequence, event.encode())?; }
                    }
                    feedback = feedback_rx.recv() => {
                        let frame = feedback.ok_or("feedback reader stopped")??;
                        if frame.kind == Kind::Clipboard {
                            let packet = ClipboardPacket::decode(&frame.payload)?;
                            if packet.origin != remote_id {
                                return Err("clipboard origin does not match peer".into());
                            }
                            if !locked { clipboard_apply.try_send(packet).map_err(|_| "clipboard apply queue is full")?; }
                            continue;
                        }
                        if frame.kind != Kind::Control
                            || (frame.payload != b"RETURN" && !frame.payload.starts_with(b"RETURN\t")) {
                            return Err("unexpected feedback from remote peer".into());
                        }
                        if active && frame.epoch == epoch {
                            if frame.payload != b"RETURN" {
                                let request = ReturnRequest::parse(&frame.payload)?;
                                if request.exit_edge.opposite() != edge.logical() {
                                    return Err("return edge does not match capture edge".into());
                                }
                                release_position = return_position(&local_lock, edge.logical(), request.fraction)
                                    .or(release_position);
                            }
                            send_frame(&mut writer, Kind::Control, epoch, &mut sequence, b"END".to_vec())?;
                            let mut options = ReleaseOptions::default().set_activation_id(current_activation.take());
                            if let Some(position) = release_position { options = options.set_cursor_position(position); }
                            portal.release(&session, options).await?;
                            active = false;
                            portal.disable(&session, Default::default()).await?;
                            capture_enabled = false;
                            rearm_after_return = true;
                            control.set_phase("rearming");
                            println!("El equipo remoto devolvió el control local.");
                        }
                    }
                    signal = activated.next() => {
                        let signal = signal.ok_or("capture activation stream closed")?;
                        if !capture_enabled || control.paused() || locked {
                            let mut options = ReleaseOptions::default().set_activation_id(signal.activation_id());
                            if let Some(position) = signal.cursor_position() { options = options.set_cursor_position(edge.release_position(position)); }
                            portal.release(&session, options).await?;
                            continue;
                        }
                        epoch = next_epoch(epoch)?;
                        sequence = 0;
                        current_activation = signal.activation_id();
                        active = true;
                        release_position = signal.cursor_position().map(|p| edge.release_position(p));
                        let entry = EntryPosition {
                            edge: edge.logical().opposite(),
                            fraction: crossing_fraction(&local_lock, edge.logical(), signal.cursor_position()),
                        };
                        send_frame(&mut writer, Kind::Control, epoch, &mut sequence, entry.begin_payload())?;
                        control.set_phase("controlling");
                        println!("Control remoto activo. Escape devuelve el puntero.");
                    }
                    signal = deactivated.next() => {
                        if signal.is_none() { return Err("capture deactivation stream closed".into()); }
                        if active {
                            send_frame(&mut writer, Kind::Control, epoch, &mut sequence, b"END".to_vec())?;
                            active = false;
                            current_activation = None;
                            control.set_phase(if locked { "locked" } else if control.paused() { "paused" } else { "ready" });
                        }
                    }
                    signal = zones_changed.next() => {
                        let signal = signal.ok_or("capture zones stream closed")?;
                        if signal.zone_set().is_some_and(|id| id != current_zone_set) { continue; }
                        if active {
                            send_frame(&mut writer, Kind::Control, epoch, &mut sequence, b"END".to_vec())?;
                            let mut options = ReleaseOptions::default().set_activation_id(current_activation.take());
                            if let Some(position) = release_position { options = options.set_cursor_position(position); }
                            let _ = portal.release(&session, options).await;
                            active = false;
                        }
                        if capture_enabled {
                            portal.disable(&session, Default::default()).await?;
                            capture_enabled = false;
                        }
                        current_zone_set = install_barriers(&portal, &session, edge).await?;
                        if !control.paused() && !locked && (!rearm_after_return || ready_to_rearm(&local_lock, edge.logical())) {
                            rearm_after_return = false;
                            portal.enable(&session, Default::default()).await?;
                            capture_enabled = true;
                            control.set_phase("ready");
                        } else {
                            control.set_phase(if locked { "locked" } else if control.paused() { "paused" } else { "rearming" });
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
                                send_frame(&mut writer, Kind::Control, epoch, &mut sequence, b"END".to_vec())?;
                                let mut options = ReleaseOptions::default().set_activation_id(current_activation.take());
                                active = false;
                                if let Some(position) = release_position { options = options.set_cursor_position(position); }
                                portal.release(&session, options).await?;
                                portal.disable(&session, Default::default()).await?;
                                capture_enabled = false;
                                rearm_after_return = true;
                                control.set_phase("rearming");
                                println!("Control local restaurado.");
                            } else if let Some(event) = input_event(event) {
                                send_frame(&mut writer, Kind::Input, epoch, &mut sequence, event.encode())?;
                            }
                        }
                    }
                }
            }
            Ok(())
        }.await;
        if active {
            let _ = send_frame(
                &mut writer,
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
        drop(clipboard_rx);
        if let Some(watch) = clipboard_watch {
            watch.stop();
        }
        drop(feedback_rx);
        let _ = writer.stream_mut().shutdown(Shutdown::Both);
        let _ = feedback_reader.join();
        drop(clipboard_apply);
        let _ = clipboard_apply_worker.join();
        control.set_phase("disconnected");
        result
    }

    struct OmarchyInjector {
        input: VirtualInput,
        started: Instant,
        control: ControlHandle,
        motion_generation: Arc<AtomicU64>,
        clipboard_apply: tokio::sync::mpsc::Sender<ClipboardPacket>,
        clipboard_remote_id: [u8; 32],
        lock_ipc: HyprIpc,
        acknowledge_release: std::sync::mpsc::SyncSender<u64>,
        entry_return_edge: Arc<Mutex<Option<LogicalEdge>>>,
    }

    impl Injector for OmarchyInjector {
        fn place_cursor(&mut self, entry: EntryPosition) -> io::Result<()> {
            if self.lock_ipc.session_lock_state()? != SessionLockState::Unlocked {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "cursor entry blocked by session lock",
                ));
            }
            let regions = self.lock_ipc.monitor_rects()?;
            let target =
                edge_entry_point(&regions, entry.edge, entry.fraction).ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidData, "no destination entry edge")
                })?;
            if let Ok(mut return_edge) = self.entry_return_edge.lock() {
                *return_edge = Some(entry.edge);
            }
            let current = self.lock_ipc.cursor_position()?;
            self.input
                .motion(
                    f64::from(target.0) - f64::from(current.0),
                    f64::from(target.1) - f64::from(current.1),
                    self.started.elapsed().as_millis() as u32,
                )
                .map_err(|error| io::Error::other(error.to_string()))
        }

        fn inject(&mut self, event: &InputEvent) -> io::Result<()> {
            if matches!(event, InputEvent::KeyDown(_) | InputEvent::ButtonDown(_))
                && self.lock_ipc.session_lock_state()? != SessionLockState::Unlocked
            {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "remote input blocked by session lock",
                ));
            }
            let time_ms = self.started.elapsed().as_millis() as u32;
            self.input
                .apply(event, time_ms)
                .map_err(|error| io::Error::other(error.to_string()))?;
            if matches!(event, InputEvent::Motion { .. }) {
                self.motion_generation.fetch_add(1, Ordering::Relaxed);
            }
            Ok(())
        }

        fn ownership_changed(&mut self, controlling: bool) {
            self.control.set_phase(if controlling {
                "controlling"
            } else {
                "connected"
            });
        }

        fn active_epoch_changed(&mut self, epoch: Option<u64>) {
            self.control.set_active_epoch(epoch);
            if epoch.is_none()
                && let Ok(mut return_edge) = self.entry_return_edge.lock()
            {
                *return_edge = None;
            }
        }

        fn control_released(&mut self, epoch: u64) -> io::Result<()> {
            self.acknowledge_release
                .try_send(epoch)
                .map_err(|_| io::Error::other("release acknowledgement queue is unavailable"))
        }

        fn clipboard_received(&mut self, packet: ClipboardPacket) -> io::Result<()> {
            if packet.origin != self.clipboard_remote_id {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "clipboard origin does not match peer",
                ));
            }
            self.clipboard_apply
                .try_send(packet)
                .map_err(|_| io::Error::other("clipboard apply queue is full"))
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

    fn clipboard_staging_dir() -> Result<PathBuf, Box<dyn Error>> {
        let base = if let Some(xdg) = std::env::var_os("XDG_CACHE_HOME") {
            PathBuf::from(xdg)
        } else {
            PathBuf::from(std::env::var_os("HOME").ok_or("HOME is not set")?).join(".cache")
        };
        Ok(clipboard_file::staging_dir(&base.join("seamlesscontrol"))?)
    }

    #[cfg(debug_assertions)]
    #[derive(Default)]
    struct Observation {
        begin: AtomicU64,
        motion: AtomicU64,
        keys: AtomicU64,
        buttons: AtomicU64,
        scroll: AtomicU64,
        clipboard: AtomicU64,
    }

    #[cfg(debug_assertions)]
    struct ObserveInjector {
        counts: Arc<Observation>,
        peer_id: [u8; 32],
    }

    #[cfg(debug_assertions)]
    impl Injector for ObserveInjector {
        fn inject(&mut self, event: &InputEvent) -> io::Result<()> {
            let counter = match event {
                InputEvent::Motion { .. } => &self.counts.motion,
                InputEvent::KeyDown(_) | InputEvent::KeyUp(_) => &self.counts.keys,
                InputEvent::ButtonDown(_) | InputEvent::ButtonUp(_) => &self.counts.buttons,
                InputEvent::Scroll { .. } => &self.counts.scroll,
            };
            counter.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }

        fn ownership_changed(&mut self, controlling: bool) {
            if controlling {
                self.counts.begin.fetch_add(1, Ordering::Relaxed);
            }
        }

        fn clipboard_received(&mut self, packet: ClipboardPacket) -> io::Result<()> {
            if packet.origin != self.peer_id {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "clipboard origin mismatch",
                ));
            }
            self.counts.clipboard.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
    }

    #[cfg(debug_assertions)]
    fn observe_connection(
        channel: SecureChannel<TcpStream>,
        first: Frame,
        peer_id: [u8; 32],
    ) -> Result<(), Box<dyn Error>> {
        let (mut reader, _writer) = channel.into_tcp_halves()?;
        let counts = Arc::new(Observation::default());
        let result = run_receiver_with_first_until(
            &mut reader,
            ObserveInjector {
                counts: Arc::clone(&counts),
                peer_id,
            },
            Some(first),
            || false,
        );
        println!(
            "OBSERVED\tbegin={}\tmotion={}\tkeys={}\tbuttons={}\tscroll={}\tclipboard={}",
            counts.begin.load(Ordering::Relaxed),
            counts.motion.load(Ordering::Relaxed),
            counts.keys.load(Ordering::Relaxed),
            counts.buttons.load(Ordering::Relaxed),
            counts.scroll.load(Ordering::Relaxed),
            counts.clipboard.load(Ordering::Relaxed),
        );
        result?;
        Ok(())
    }

    fn serve_connection(
        mut stream: TcpStream,
        peer_ip: IpAddr,
        identity: &Identity,
        config: &std::path::Path,
        control: &ControlHandle,
    ) -> Result<(), Box<dyn Error>> {
        if std::env::var_os("SEAMLESSCONTROL_TEST_OBSERVE").is_some() {
            if !cfg!(debug_assertions) {
                return Err("input observation requires a debug build".into());
            }
            if !peer_ip.is_loopback() {
                return Err("input observation is restricted to loopback".into());
            }
        }
        let peers = config.join("peers");
        let pinned = load_peer_key(&peers, peer_ip)?;
        let known = if pinned.is_none() {
            list_peer_keys(&peers)?
        } else {
            Vec::new()
        };
        let mut previous_ip = None;
        let mut reapproved_revoked = false;
        stream.set_nodelay(true)?;
        stream.set_read_timeout(Some(PAIRING_SOCKET_TIMEOUT))?;
        stream.set_write_timeout(Some(PAIRING_SOCKET_TIMEOUT))?;
        let explicit_pair = negotiate_pairing(&mut stream, Role::Responder, false)?;
        let (mut channel, peer) = SecureChannel::connect(
            stream,
            Role::Responder,
            identity,
            if explicit_pair { None } else { pinned.as_ref() },
            |peer| {
                if is_revoked(&peers, &peer.public_key).unwrap_or(true) {
                    if !explicit_pair {
                        return false;
                    }
                    reapproved_revoked = control.confirm_pair(peer);
                    return reapproved_revoked;
                }
                if let Some((old, _)) = known.iter().find(|(_, key)| *key == peer.public_key) {
                    previous_ip = Some(*old);
                    !explicit_pair || control.confirm_pair(peer)
                } else {
                    control.confirm_pair(peer)
                }
            },
        )
        .map_err(|error| {
            std::io::Error::other(format!(
                "secure handshake (explicit pairing: {explicit_pair}): {error}"
            ))
        })?;
        if is_revoked(&peers, &peer.public_key)? && !reapproved_revoked {
            return Err("peer identity has been revoked".into());
        }
        channel
            .stream_mut()
            .set_read_timeout(Some(Duration::from_secs(120)))?;
        channel
            .stream_mut()
            .set_write_timeout(Some(Duration::from_secs(120)))?;
        let greeting = Frame::read_from(&mut channel)?;
        if greeting.kind != Kind::Hello || greeting.payload != AGENT_PROTOCOL {
            return Err("incompatible peer protocol".into());
        }
        Frame {
            kind: Kind::Hello,
            epoch: 0,
            sequence: 0,
            payload: AGENT_PROTOCOL.to_vec(),
        }
        .write_to(&mut channel)?;
        if let Some(old) = previous_ip {
            relocate_peer_key(&peers, old, peer_ip, &peer.public_key)?;
        } else if reapproved_revoked {
            reapprove_peer_key(&peers, peer_ip, &peer.public_key)?;
        } else {
            remember_peer_key(&peers, peer_ip, &peer.public_key)?;
        }
        let mut first = Frame::read_from(&mut channel)?;
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
        if first.kind == Kind::Control && first.payload == b"PING" {
            channel
                .stream_mut()
                .set_read_timeout(Some(Duration::from_secs(5)))?;
            channel
                .stream_mut()
                .set_write_timeout(Some(Duration::from_secs(5)))?;
            let mut request = first;
            for sample in 1..=LATENCY_SAMPLES {
                if request.kind != Kind::Control
                    || request.epoch != 0
                    || request.sequence != sample
                    || request.payload != b"PING"
                {
                    return Err("invalid latency probe sequence".into());
                }
                Frame {
                    kind: Kind::Control,
                    epoch: 0,
                    sequence: sample,
                    payload: b"PONG".to_vec(),
                }
                .write_to(&mut channel)?;
                if sample < LATENCY_SAMPLES {
                    request = Frame::read_from(&mut channel)?;
                }
            }
            return Ok(());
        }
        let mesh_source = first.payload == b"CLAIM-MESH";
        if first.kind != Kind::Control
            || first.epoch != 0
            || first.sequence != 0
            || (first.payload != b"CLAIM" && !mesh_source)
        {
            return Err("capture claim required before remote input".into());
        }
        let Some(_lease) = control.claim_receiver(&peer_ip.to_string()) else {
            Frame {
                kind: Kind::Control,
                epoch: 0,
                sequence: 0,
                payload: b"BUSY".to_vec(),
            }
            .write_to(&mut channel)?;
            return Err("another peer already owns remote input".into());
        };
        Frame {
            kind: Kind::Control,
            epoch: 0,
            sequence: 0,
            payload: b"READY".to_vec(),
        }
        .write_to(&mut channel)?;
        channel
            .stream_mut()
            .set_read_timeout(Some(PAIRING_SOCKET_TIMEOUT))?;
        first = Frame::read_from(&mut channel)?;
        channel
            .stream_mut()
            .set_read_timeout(Some(Duration::from_secs(if mesh_source { 15 } else { 5 })))?;
        channel
            .stream_mut()
            .set_write_timeout(Some(Duration::from_secs(5)))?;
        println!("Conexión autenticada con {peer_ip}. Esperando control...");
        if std::env::var_os("SEAMLESSCONTROL_TEST_OBSERVE").is_some() {
            #[cfg(debug_assertions)]
            return observe_connection(channel, first, peer.public_key);
        }
        let topology = load_topology(&config.join("topology")).ok();
        let edge_targets: Vec<(LogicalEdge, IpAddr)> = if mesh_source {
            topology
                .as_ref()
                .map(|layout| {
                    [
                        LogicalEdge::Left,
                        LogicalEdge::Right,
                        LogicalEdge::Top,
                        LogicalEdge::Bottom,
                    ]
                    .into_iter()
                    .filter_map(|edge| match layout.neighbor(Machine::Local, edge) {
                        Some(Machine::Peer(target)) => Some((edge, target)),
                        _ => None,
                    })
                    .collect()
                })
                .unwrap_or_default()
        } else {
            topology
                .as_ref()
                .and_then(|layout| layout.edge_to(peer_ip).ok())
                .map(|edge| vec![(edge, peer_ip)])
                .unwrap_or_default()
        };
        let hypr = HyprIpc::from_env().ok_or("Hyprland IPC is required to guard remote input")?;
        if hypr.session_lock_state()? != SessionLockState::Unlocked {
            return Err("session is locked or lock state is undetermined".into());
        }
        // Direct sources provide the destination entry edge in BEGIN. It
        // defines the return edge even without a saved receiver topology.
        let entry_return_edge = Arc::new(Mutex::new(None));
        let watcher_entry_edge = Arc::clone(&entry_return_edge);
        let edge_hypr = Some(hypr.clone());
        let motion_generation = Arc::new(AtomicU64::new(0));
        let input = VirtualInput::connect()?;
        let (mut reader, mut writer) = channel.into_tcp_halves()?;
        let clipboard_sync = Arc::new(Mutex::new(ClipboardSync::new(
            identity.public,
            peer.public_key,
        )));
        let (clipboard_apply, clipboard_apply_worker) =
            spawn_apply_worker(Arc::clone(&clipboard_sync));
        let (clipboard_tx, clipboard_rx) = std::sync::mpsc::sync_channel(8);
        let (release_tx, release_rx) = std::sync::mpsc::sync_channel(8);
        let clipboard_watch = match ClipboardWatch::start(clipboard_sync, move |event| {
            let _ = clipboard_tx.send(event);
        }) {
            Ok(watch) => Some(watch),
            Err(error) => {
                eprintln!("SeamlessControl: observación del portapapeles no disponible: {error}");
                None
            }
        };
        let injector = OmarchyInjector {
            input,
            started: Instant::now(),
            control: control.clone(),
            motion_generation: Arc::clone(&motion_generation),
            clipboard_apply: clipboard_apply.clone(),
            clipboard_remote_id: peer.public_key,
            lock_ipc: hypr.clone(),
            acknowledge_release: release_tx,
            entry_return_edge,
        };
        let watcher_running = Arc::new(AtomicBool::new(true));
        let watcher_flag = Arc::clone(&watcher_running);
        let watcher_control = control.clone();
        let watcher = thread::spawn(move || {
            let mut sequence = 0_u64;
            let mut observed_epoch = None;
            let mut sent_epoch = None;
            let mut observed_motion = 0_u64;
            let mut remote_motion_seen = false;
            let mut detectors: Vec<(LogicalEdge, IpAddr, EdgeReturnDetector)> = Vec::new();
            let mut session_targets = edge_targets.clone();
            let mut monitor_regions: Option<Vec<Rect>> = None;
            let mut last_geometry_refresh = Instant::now();
            while watcher_flag.load(Ordering::Relaxed) {
                if watcher_control.disconnect_requested() {
                    let _ = writer.stream_mut().shutdown(Shutdown::Both);
                    break;
                }
                if hypr
                    .session_lock_state()
                    .unwrap_or(SessionLockState::Undetermined)
                    != SessionLockState::Unlocked
                {
                    eprintln!(
                        "SeamlessControl: sesión bloqueada o estado desconocido; se corta la entrada remota."
                    );
                    let _ = writer.stream_mut().shutdown(Shutdown::Both);
                    break;
                }
                let active_epoch = watcher_control.active_epoch();
                if active_epoch != observed_epoch {
                    observed_epoch = active_epoch;
                    sent_epoch = None;
                    observed_motion = motion_generation.load(Ordering::Relaxed);
                    remote_motion_seen = false;
                    session_targets = if !mesh_source {
                        watcher_entry_edge
                            .lock()
                            .ok()
                            .and_then(|edge| *edge)
                            .map(|edge| vec![(edge, peer_ip)])
                            .unwrap_or_else(|| edge_targets.clone())
                    } else {
                        edge_targets.clone()
                    };
                    last_geometry_refresh = Instant::now();
                    monitor_regions = if active_epoch.is_some() {
                        edge_hypr.as_ref().and_then(|ipc| ipc.monitor_rects().ok())
                    } else {
                        None
                    };
                    detectors = monitor_regions
                        .as_ref()
                        .map(|regions| {
                            session_targets
                                .iter()
                                .map(|(edge, target)| {
                                    (*edge, *target, EdgeReturnDetector::new(regions, *edge))
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                } else if active_epoch.is_some()
                    && sent_epoch.is_none()
                    && last_geometry_refresh.elapsed() >= Duration::from_secs(5)
                {
                    last_geometry_refresh = Instant::now();
                    if let Some(regions) =
                        edge_hypr.as_ref().and_then(|ipc| ipc.monitor_rects().ok())
                        && monitor_regions.as_ref() != Some(&regions)
                    {
                        detectors = session_targets
                            .iter()
                            .map(|(edge, target)| {
                                (*edge, *target, EdgeReturnDetector::new(&regions, *edge))
                            })
                            .collect();
                        monitor_regions = Some(regions);
                    }
                }
                let manual_return = watcher_control
                    .take_return_request()
                    .map(|epoch| (epoch, b"RETURN".to_vec()));
                let motion = motion_generation.load(Ordering::Relaxed);
                if motion != observed_motion {
                    observed_motion = motion;
                    remote_motion_seen = true;
                }
                // Virtual input is asynchronous: Hyprland can move the cursor
                // after the last injected motion event. Keep sampling until
                // the epoch ends so a release at the edge is not missed.
                let automatic_return = if sent_epoch.is_none() && remote_motion_seen {
                    match (active_epoch, edge_hypr.as_ref()) {
                        (Some(epoch), Some(ipc)) => {
                            ipc.cursor_position().ok().and_then(|(x, y)| {
                                detectors.iter_mut().find_map(|(edge, target, detector)| {
                                    if !detector.sample(x, y) {
                                        return None;
                                    }
                                    let payload = if *target == peer_ip {
                                        ReturnRequest {
                                            exit_edge: *edge,
                                            fraction: monitor_regions
                                                .as_ref()
                                                .and_then(|regions| {
                                                    edge_fraction(regions, *edge, x, y)
                                                })
                                                .unwrap_or(u16::MAX / 2),
                                        }
                                        .encode()
                                    } else {
                                        SwitchRequest {
                                            target: *target,
                                            exit_edge: *edge,
                                            fraction: monitor_regions
                                                .as_ref()
                                                .and_then(|regions| {
                                                    edge_fraction(regions, *edge, x, y)
                                                })
                                                .unwrap_or(u16::MAX / 2),
                                        }
                                        .encode()
                                    };
                                    Some((epoch, payload))
                                })
                            })
                        }
                        _ => None,
                    }
                } else {
                    None
                };
                if let Some((active_epoch, payload)) = manual_return.or(automatic_return)
                    && sent_epoch != Some(active_epoch)
                {
                    let Some(next) = sequence.checked_add(1) else {
                        break;
                    };
                    sequence = next;
                    if (Frame {
                        kind: Kind::Control,
                        epoch: active_epoch,
                        sequence,
                        payload,
                    })
                    .write_to(&mut writer)
                    .is_err()
                    {
                        let _ = writer.stream_mut().shutdown(Shutdown::Both);
                        break;
                    }
                    sent_epoch = Some(active_epoch);
                }
                while let Ok(ended_epoch) = release_rx.try_recv() {
                    if send_frame(
                        &mut writer,
                        Kind::Control,
                        ended_epoch,
                        &mut sequence,
                        b"ENDED".to_vec(),
                    )
                    .is_err()
                    {
                        let _ = writer.stream_mut().shutdown(Shutdown::Both);
                        return;
                    }
                }
                while let Ok(event) = clipboard_rx.try_recv() {
                    if send_frame(
                        &mut writer,
                        Kind::Clipboard,
                        0,
                        &mut sequence,
                        event.encode(),
                    )
                    .is_err()
                    {
                        let _ = writer.stream_mut().shutdown(Shutdown::Both);
                        return;
                    }
                }
                thread::sleep(Duration::from_millis(40));
            }
        });
        let result = run_receiver_with_first_until(&mut reader, injector, Some(first), || {
            control.revoked_active() || control.disconnect_requested()
        })
        .map(|_| ());
        watcher_running.store(false, Ordering::Relaxed);
        let _ = watcher.join();
        if let Some(watch) = clipboard_watch {
            watch.stop();
        }
        drop(clipboard_apply);
        let _ = clipboard_apply_worker.join();
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
        mode: ConnectionMode,
        identity: &Identity,
        config: &std::path::Path,
        control: &ControlHandle,
    ) -> Result<(), AttemptError> {
        let peers = config.join("peers");
        let pinned =
            load_peer_key(&peers, address.ip()).map_err(|e| AttemptError::Stop(Box::new(e)))?;
        let known = if pinned.is_none() {
            list_peer_keys(&peers).map_err(|e| AttemptError::Stop(Box::new(e)))?
        } else {
            Vec::new()
        };
        let mut previous_ip = None;
        let mut reapproved_revoked = false;
        let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(5))
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
        let explicit_pair = matches!(mode, ConnectionMode::Pair);
        negotiate_pairing(&mut stream, Role::Initiator, explicit_pair)
            .map_err(|e| AttemptError::Stop(Box::new(e)))?;
        let (mut channel, peer) = SecureChannel::connect(
            stream,
            Role::Initiator,
            identity,
            if explicit_pair { None } else { pinned.as_ref() },
            |peer| {
                if is_revoked(&peers, &peer.public_key).unwrap_or(true) {
                    if !explicit_pair {
                        return false;
                    }
                    reapproved_revoked = control.confirm_pair(peer);
                    return reapproved_revoked;
                }
                if let Some((old, _)) = known.iter().find(|(_, key)| *key == peer.public_key) {
                    previous_ip = Some(*old);
                    !explicit_pair || control.confirm_pair(peer)
                } else {
                    control.confirm_pair(peer)
                }
            },
        )
        .map_err(|error| match error {
            SecureError::Io(_) => AttemptError::Retry(Box::new(error)),
            _ => AttemptError::Stop(Box::new(error)),
        })?;
        if is_revoked(&peers, &peer.public_key).map_err(|e| AttemptError::Stop(Box::new(e)))?
            && !reapproved_revoked
        {
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
            payload: AGENT_PROTOCOL.to_vec(),
        }
        .write_to(&mut channel)
        .map_err(frame_attempt)?;
        let greeting = Frame::read_from(&mut channel).map_err(frame_attempt)?;
        if greeting.kind != Kind::Hello || greeting.payload != AGENT_PROTOCOL {
            return Err(AttemptError::Stop("incompatible peer protocol".into()));
        }
        if let Some(old) = previous_ip {
            relocate_peer_key(&peers, old, address.ip(), &peer.public_key)
                .map_err(|e| AttemptError::Stop(Box::new(e)))?;
        } else if reapproved_revoked {
            reapprove_peer_key(&peers, address.ip(), &peer.public_key)
                .map_err(|e| AttemptError::Stop(Box::new(e)))?;
        } else {
            remember_peer_key(&peers, address.ip(), &peer.public_key)
                .map_err(|e| AttemptError::Stop(Box::new(e)))?;
        }
        channel
            .stream_mut()
            .set_write_timeout(Some(Duration::from_secs(5)))
            .map_err(|e| AttemptError::Retry(Box::new(e)))?;
        match mode {
            ConnectionMode::Capture(edge) | ConnectionMode::CaptureMapped(edge) => {
                Frame {
                    kind: Kind::Control,
                    epoch: 0,
                    sequence: 0,
                    payload: b"CLAIM".to_vec(),
                }
                .write_to(&mut channel)
                .map_err(frame_attempt)?;
                let response = Frame::read_from(&mut channel).map_err(frame_attempt)?;
                if response.kind == Kind::Control && response.payload == b"BUSY" {
                    return Err(AttemptError::Retry("remote input is busy".into()));
                }
                if response.kind != Kind::Control
                    || response.epoch != 0
                    || response.sequence != 0
                    || response.payload != b"READY"
                {
                    return Err(AttemptError::Stop("invalid capture claim reply".into()));
                }
                println!("Conexión autenticada con {address}.");
                let topology_path = config.join("topology");
                capture_loop(
                    channel,
                    edge,
                    control.clone(),
                    identity.public,
                    peer.public_key,
                    matches!(mode, ConnectionMode::CaptureMapped(_))
                        .then_some((topology_path.as_path(), address.ip())),
                )
                .await
                .map_err(|error| {
                    if error
                        .downcast_ref::<FrameError>()
                        .is_some_and(|frame| matches!(frame, FrameError::Io(_)))
                        || error.downcast_ref::<TopologyChanged>().is_some()
                    {
                        AttemptError::Retry(error)
                    } else {
                        AttemptError::Stop(error)
                    }
                })
            }
            ConnectionMode::Pair => {
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
            ConnectionMode::Latency => {
                let mut samples = Vec::with_capacity(LATENCY_SAMPLES as usize);
                for sequence in 1..=LATENCY_SAMPLES {
                    let start = Instant::now();
                    Frame {
                        kind: Kind::Control,
                        epoch: 0,
                        sequence,
                        payload: b"PING".to_vec(),
                    }
                    .write_to(&mut channel)
                    .map_err(frame_attempt)?;
                    let reply = Frame::read_from(&mut channel).map_err(frame_attempt)?;
                    if reply.kind != Kind::Control
                        || reply.epoch != 0
                        || reply.sequence != sequence
                        || reply.payload != b"PONG"
                    {
                        return Err(AttemptError::Stop("invalid latency probe reply".into()));
                    }
                    samples.push(start.elapsed().as_micros());
                }
                samples.sort_unstable();
                println!(
                    "LATENCY\t{}\t{}\t{}\t{}\t{}",
                    LATENCY_SAMPLES, samples[0], samples[9], samples[18], samples[19]
                );
                Ok(())
            }
        }
    }

    enum ReceiverMode {
        Manual(SocketAddr),
        Automatic,
    }

    async fn serve_receiver<P, S>(
        mode: ReceiverMode,
        mut probe: P,
        refresh: Duration,
        identity: Identity,
        config: PathBuf,
        control: ControlHandle,
        stop: S,
    ) -> Result<(), Box<dyn Error>>
    where
        P: FnMut() -> io::Result<SocketAddr>,
        S: Future<Output = ()>,
    {
        let automatic = matches!(mode, ReceiverMode::Automatic);
        let mut address = match mode {
            ReceiverMode::Manual(address) => Some(address),
            ReceiverMode::Automatic => None,
        };
        let mut stop = std::pin::pin!(stop);
        loop {
            let selected = match address.take().map(Ok).unwrap_or_else(&mut probe) {
                Ok(value) => value,
                Err(error) if automatic => {
                    control.set_phase("reconnecting");
                    eprintln!("SeamlessControl: esperando dirección LAN: {error}");
                    tokio::select! {
                        _ = &mut stop => return Ok(()),
                        _ = tokio::time::sleep(refresh) => continue,
                    }
                }
                Err(error) => return Err(error.into()),
            };
            let listener = match TcpListener::bind(selected) {
                Ok(listener) => listener,
                Err(error) if automatic && error.kind() == io::ErrorKind::AddrNotAvailable => {
                    control.set_phase("reconnecting");
                    eprintln!("SeamlessControl: esperando IP local {selected}: {error}");
                    tokio::select! {
                        _ = &mut stop => return Ok(()),
                        _ = tokio::time::sleep(refresh) => continue,
                    }
                }
                Err(error) => return Err(error.into()),
            };
            listener.set_nonblocking(true)?;
            let listener = tokio::net::TcpListener::from_std(listener)?;
            let advertisement = match ServiceAdvertisement::publish(selected, &identity) {
                Ok(value) => value,
                Err(error) => {
                    eprintln!("SeamlessControl: anuncio mDNS no disponible: {error}");
                    None
                }
            };
            control.set_phase("listening");
            println!("SeamlessControl escucha en {selected}");
            let connections = Arc::new(Mutex::new(HashMap::<u64, TcpStream>::new()));
            let limit = Arc::new(tokio::sync::Semaphore::new(MAX_INBOUND_CONNECTIONS));
            let mut workers = tokio::task::JoinSet::new();
            let mut next_connection_id = 0_u64;
            let mut check = tokio::time::interval(refresh);
            check.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            check.tick().await;
            let mut stopping = false;
            loop {
                tokio::select! {
                    _ = &mut stop => {
                        stopping = true;
                        break;
                    }
                    _ = check.tick(), if automatic => {
                        match probe() {
                            Ok(current) if current == selected => {}
                            Ok(current) => {
                                eprintln!("SeamlessControl: IP LAN cambió: {selected} → {current}");
                                address = Some(current);
                                break;
                            }
                            Err(error) => {
                                eprintln!("SeamlessControl: dirección LAN no disponible: {error}");
                                address = None;
                                break;
                            }
                        }
                    }
                    completed = workers.join_next(), if !workers.is_empty() => {
                        if let Some(Err(error)) = completed {
                            eprintln!("Error en un trabajador de red: {error}");
                        }
                    }
                    incoming = listener.accept() => {
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
                        let permit = match Arc::clone(&limit).try_acquire_owned() {
                            Ok(permit) => permit,
                            Err(_) => {
                                eprintln!("Demasiadas conexiones entrantes; se rechazó {ip}");
                                continue;
                            }
                        };
                        let stream = stream.into_std()?;
                        stream.set_nonblocking(false)?;
                        let shutdown = stream.try_clone()?;
                        next_connection_id = next_connection_id
                            .checked_add(1)
                            .ok_or("connection identifier exhausted")?;
                        let id = next_connection_id;
                        connections.lock().expect("connection registry lock").insert(id, shutdown);
                        let registry = Arc::clone(&connections);
                        let identity = identity.clone();
                        let config = config.clone();
                        let connection_control = control.clone();
                        workers.spawn_blocking(move || {
                            let _permit = permit;
                            let result = serve_connection(stream, ip, &identity, &config, &connection_control);
                            registry.lock().expect("connection registry lock").remove(&id);
                            if let Err(error) = result {
                                eprintln!("Conexión con {ip} terminada: {error}");
                            }
                        });
                    }
                }
            }
            control.cancel_pair();
            if let Ok(open) = connections.lock() {
                for stream in open.values() {
                    let _ = stream.shutdown(Shutdown::Both);
                }
            }
            while let Some(result) = workers.join_next().await {
                if let Err(error) = result {
                    eprintln!("Error al cerrar un trabajador de red: {error}");
                }
            }
            drop(advertisement);
            drop(listener);
            control.set_peer("");
            if stopping {
                return Ok(());
            }
            control.set_phase("reconnecting");
        }
    }

    pub async fn run() -> Result<(), Box<dyn Error>> {
        let args: Vec<String> = std::env::args().collect();
        if args.len() == 3 && matches!(args[1].as_str(), "choose-file" | "choose-folder") {
            if args[2] != "en" && args[2] != "es" {
                return Err("picker language must be en or es".into());
            }
            return choose_local_path(args[1] == "choose-folder", args[2] == "es").await;
        }
        if args.len() == 2 && args[1] == "clipboard-helper" {
            clipboard_omarchy::emit_watched_event()?;
            return Ok(());
        }
        if args.len() == 2 && args[1] == "clipboard-file-watch" {
            use std::os::unix::process::CommandExt;
            let executable = std::env::current_exe()?;
            let error = std::process::Command::new("wl-paste")
                .arg("--watch")
                .arg(executable)
                .arg("clipboard-file-event")
                .exec();
            return Err(error.into());
        }
        if args.len() == 2
            && matches!(
                args[1].as_str(),
                "clipboard-file-current" | "clipboard-file-event"
            )
        {
            if args[1] == "clipboard-file-event"
                && std::env::var("CLIPBOARD_STATE").as_deref() == Ok("sensitive")
            {
                println!("null");
                return Ok(());
            }
            let staging = clipboard_staging_dir()?;
            let path = clipboard_omarchy::copied_file(file_session::configured_limit()?, &staging)?
                .and_then(|path| path.to_str().map(str::to_owned));
            if path.is_some() || args[1] == "clipboard-file-event" {
                println!("{}", serde_json::to_string(&path)?);
            }
            return Ok(());
        }
        if args.len() == 3 && args[1] == "receive-file-clipboard-ui" {
            let port: u16 = args[2].parse()?;
            let address = discovery::auto_lan_address(port)?;
            let staging = clipboard_staging_dir()?;
            let session = clipboard_file::staging_session_dir(&staging)?;
            let config = config_dir()?;
            let identity = load_or_create_identity(&config.join("identity"))?;
            let limit = file_session::configured_limit()?;
            let result = file_session::receive_once_with_progress(
                address,
                &session,
                &identity,
                &config.join("peers"),
                limit,
                |offer, peer| {
                    if !clipboard_file::staging_can_fit(&staging, offer.size, limit)? {
                        println!("STAGING_FULL");
                        io::stdout().flush()?;
                        return Ok(false);
                    }
                    file_session::panel_approval(offer, peer)
                },
                |bound| {
                    println!("LISTENING\t{bound}");
                    io::stdout().flush()
                },
                |percent| {
                    println!("PROGRESS\t{percent}");
                    let _ = io::stdout().flush();
                },
            )?;
            if let Some(path) = result {
                clipboard_omarchy::publish_file(&path)?;
                println!(
                    "FILE_READY\t{}",
                    serde_json::to_string(&path.to_string_lossy().to_string())?
                );
            } else {
                println!("FILE_DECLINED");
            }
            return Ok(());
        }
        if args.len() == 2 && args[1] == "diagnose" {
            let ipc = HyprIpc::from_env().ok_or("Hyprland IPC environment is unavailable")?;
            let (x, y) = ipc.cursor_position()?;
            println!("CURSOR\t{x}\t{y}");
            for rect in ipc.monitor_rects()? {
                println!(
                    "MONITOR\t{}\t{}\t{}\t{}",
                    rect.x, rect.y, rect.width, rect.height
                );
            }
            let lock = match ipc.session_lock_state()? {
                SessionLockState::Locked => "locked",
                SessionLockState::Unlocked => "unlocked",
                SessionLockState::Undetermined => "undetermined",
            };
            println!("LOCK\t{lock}");
            return Ok(());
        }
        if (args.len() == 2 && args[1] == "discover")
            || (args.len() == 3 && args[1] == "discover" && args[2] == "--include-local")
        {
            for server in discovery::browse(args.len() == 3).await? {
                println!(
                    "FOUND\t{}\t{}\t{}\t{}",
                    server.name,
                    server.address.ip(),
                    server.address.port(),
                    server.fingerprint
                );
            }
            return Ok(());
        }
        if args.len() == 3 && args[1] == "local-address" {
            let port: u16 = args[2].parse()?;
            println!("{}", discovery::auto_lan_address(port)?);
            return Ok(());
        }
        if args.len() == 4
            && matches!(
                args[1].as_str(),
                "send-file"
                    | "receive-file"
                    | "receive-file-ui"
                    | "receive-file-auto"
                    | "receive-file-auto-ui"
            )
        {
            let address: SocketAddr = if args[1].starts_with("receive-file-auto") {
                discovery::auto_lan_address(args[2].parse()?)?
            } else {
                args[2].parse()?
            };
            if !local_address(address.ip()) {
                return Err(
                    "file transfer address must be loopback, link-local or private LAN".into(),
                );
            }
            let config = config_dir()?;
            let identity = load_or_create_identity(&config.join("identity"))?;
            let limit = file_session::configured_limit()?;
            if args[1] == "send-file" {
                file_session::send_once_with_progress(
                    address,
                    std::path::Path::new(&args[3]),
                    &identity,
                    &config.join("peers"),
                    limit,
                    |percent| {
                        println!("PROGRESS\t{percent}");
                        let _ = io::stdout().flush();
                    },
                )?;
                println!("Archivo entregado y verificado por el destino.");
            } else {
                let panel_mode =
                    matches!(args[1].as_str(), "receive-file-ui" | "receive-file-auto-ui");
                let result = file_session::receive_once(
                    address,
                    std::path::Path::new(&args[3]),
                    &identity,
                    &config.join("peers"),
                    limit,
                    if panel_mode {
                        file_session::panel_approval
                    } else {
                        file_session::terminal_approval
                    },
                    |bound| {
                        if panel_mode {
                            println!("LISTENING\t{bound}");
                            io::stdout().flush()?;
                        }
                        Ok(())
                    },
                )?;
                match result {
                    Some(path) => println!("Archivo guardado en {}", path.display()),
                    None => println!("Archivo rechazado o cancelado."),
                }
            }
            return Ok(());
        }
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
                [_, command, action, who] if command == "topology" && action == "route" => {
                    let address: IpAddr = who.parse()?;
                    for (index, hop) in layout.route_to(address)?.into_iter().enumerate() {
                        let Machine::Peer(peer) = hop.machine else {
                            return Err("invalid route through local machine".into());
                        };
                        if load_peer_key(&config.join("peers"), peer)?.is_none() {
                            return Err("route contains an unpaired peer".into());
                        }
                        println!("HOP\t{}\t{peer}\t{}", index + 1, hop.edge.as_str());
                    }
                    return Ok(());
                }
                _ => {
                    return Err(
                        "usage: topology [set <local|IP> <column> <row> | remove <IP> | route <IP>]".into(),
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
        if args.len() == 2
            && matches!(
                args[1].as_str(),
                "status" | "pause" | "resume" | "reject" | "return" | "emergency-stop" | "stop"
            )
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
        if args.len() == 2 && args[1] == "rotate-key" {
            match control::request("status") {
                Ok(_) => return Err("stop the local agent before rotating its identity".into()),
                Err(error)
                    if matches!(
                        error.kind(),
                        io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused
                    ) => {}
                Err(error) => return Err(error.into()),
            }
            let config = config_dir()?;
            let identity = rotate_identity(&config.join("identity"))?;
            println!(
                "Nueva identidad local: {}",
                key_fingerprint(&identity.public)
            );
            println!("Revoque la identidad anterior en cada equipo remoto y empareje de nuevo.");
            return Ok(());
        }
        if args.len() == 3 && args[1] == "mesh" {
            let port: u16 = args[2].parse()?;
            if port == 0 {
                return Err("mesh port must be nonzero".into());
            }
            let config = config_dir()?;
            let identity = load_or_create_identity(&config.join("identity"))?;
            let local_control = ControlServer::start("connect", &config)?;
            let control = local_control.handle();
            let mut delay = Duration::from_secs(1);
            loop {
                match mesh_loop(port, &identity, &config, control.clone()).await? {
                    MeshSession::Stopped => return Ok(()),
                    MeshSession::Retry(error, uptime) => {
                        if uptime >= Duration::from_secs(30) {
                            delay = Duration::from_secs(1);
                        }
                        eprintln!(
                            "Malla interrumpida: {error}. Reintentando en {} s.",
                            delay.as_secs()
                        );
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
        if !(args.len() == 3
            && matches!(
                args[1].as_str(),
                "serve" | "serve-auto" | "pair" | "connect" | "latency"
            )
            || args.len() == 4 && args[1] == "connect")
        {
            eprintln!(
                "Uso: seamlesscontrold serve <IP-LAN:PUERTO>\n     seamlesscontrold serve-auto <PUERTO>\n     seamlesscontrold discover\n     seamlesscontrold pair <IP-LAN:PUERTO>\n     seamlesscontrold connect <IP-LAN:PUERTO> [left|right|top|bottom]\n     seamlesscontrold mesh <PUERTO>\n     seamlesscontrold stop            # detiene el receptor normalmente\n     seamlesscontrold emergency-stop  # en el receptor; corta y pausa\n     seamlesscontrold resume          # reanuda el receptor\n     seamlesscontrold latency <IP-LAN:PUERTO>\n     seamlesscontrold choose-file en|es\n     seamlesscontrold choose-folder en|es\n     seamlesscontrold receive-file <IP-LAN:PUERTO> <directorio>\n     seamlesscontrold receive-file-auto <PUERTO> <directorio>\n     seamlesscontrold send-file <IP-LAN:PUERTO> <archivo>\n     seamlesscontrold local-address <PUERTO>\n     seamlesscontrold rotate-key"
            );
            return Err("invalid arguments".into());
        }
        let address: SocketAddr = if args[1] == "serve-auto" {
            let port: u16 = args[2].parse()?;
            if port == 0 {
                return Err("serve-auto port must be nonzero".into());
            }
            SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port)
        } else {
            args[2].parse()?
        };
        if args[1] != "serve-auto" && !local_address(address.ip()) {
            return Err("listen address must be loopback, link-local or private LAN".into());
        }
        let config = config_dir()?;
        let identity = load_or_create_identity(&config.join("identity"))?;
        println!("Identidad local: {}", key_fingerprint(&identity.public));
        if args[1] == "latency" && load_peer_key(&config.join("peers"), address.ip())?.is_none() {
            return Err("pair this peer before measuring latency".into());
        }
        if matches!(args[1].as_str(), "connect" | "pair" | "latency") {
            let _local_control = ControlServer::start("connect", &config)?;
            let control = _local_control.handle();
            let mut mode = if args[1] == "connect" && args.len() == 4 {
                ConnectionMode::Capture(Edge::parse(&args[3])?)
            } else if args[1] == "connect" {
                let topology = load_topology(&config.join("topology"))?;
                ConnectionMode::CaptureMapped(Edge::from_logical(topology.edge_to(address.ip())?))
            } else if args[1] == "latency" {
                ConnectionMode::Latency
            } else {
                ConnectionMode::Pair
            };
            let mut delay = Duration::from_secs(1);
            loop {
                if matches!(mode, ConnectionMode::CaptureMapped(_)) {
                    let topology = load_topology(&config.join("topology"))?;
                    mode = ConnectionMode::CaptureMapped(Edge::from_logical(
                        topology.edge_to(address.ip())?,
                    ));
                }
                match connect_once(address, mode, &identity, &config, &control).await {
                    Ok(()) => return Ok(()),
                    Err(AttemptError::Stop(error)) => return Err(error),
                    Err(AttemptError::Retry(error))
                        if !matches!(
                            mode,
                            ConnectionMode::Capture(_) | ConnectionMode::CaptureMapped(_)
                        ) =>
                    {
                        return Err(error);
                    }
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
        let stop_control = control.clone();
        let stop = async move {
            loop {
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => break,
                    _ = tokio::time::sleep(Duration::from_millis(100)) => {
                        if stop_control.shutdown_requested() { break; }
                    }
                }
            }
        };
        if args[1] == "serve-auto" {
            let port = address.port();
            serve_receiver(
                ReceiverMode::Automatic,
                move || discovery::auto_lan_address(port),
                Duration::from_secs(3),
                identity,
                config,
                control,
                stop,
            )
            .await
        } else {
            serve_receiver(
                ReceiverMode::Manual(address),
                move || Ok(address),
                Duration::from_secs(3),
                identity,
                config,
                control,
                stop,
            )
            .await
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::io::Read;
        use std::net::Ipv4Addr;

        #[test]
        fn portal_selection_accepts_only_local_file_paths() {
            assert_eq!(
                portal_file_path("file:///home/user/My%20file%25.txt")
                    .unwrap()
                    .to_str(),
                Some("/home/user/My file%.txt")
            );
            assert!(portal_file_path("https://example.com/file").is_err());
            assert!(portal_file_path("file://remote-host/home/user/file").is_err());
            assert!(portal_file_path("file:///home/user/%00file").is_err());
            assert!(portal_file_path("file:///home/user/%GG").is_err());
        }

        fn test_mesh_link(remote: Identity) -> (MeshLink, thread::JoinHandle<io::Result<usize>>) {
            let local = Identity::generate().unwrap();
            let remote_key = remote.public;
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let server = thread::spawn(move || {
                let (stream, _) = listener.accept()?;
                stream.set_read_timeout(Some(Duration::from_secs(3)))?;
                let (mut channel, _) = SecureChannel::connect(
                    stream,
                    Role::Responder,
                    &remote,
                    Some(&local.public),
                    |_| false,
                )
                .map_err(io::Error::other)?;
                let mut byte = [0];
                channel.read(&mut byte)
            });
            let stream = TcpStream::connect(address).unwrap();
            let (channel, _) =
                SecureChannel::connect(stream, Role::Initiator, &local, Some(&remote_key), |_| {
                    false
                })
                .unwrap();
            let (_, writer) = channel.into_tcp_halves().unwrap();
            (
                MeshLink {
                    writer,
                    peer_key: remote_key,
                    sequence: 0,
                    reader: None,
                },
                server,
            )
        }

        #[test]
        fn revoking_mesh_peers_closes_links_and_releases_active_control() {
            let scratch = std::env::temp_dir().join(format!(
                "seamlesscontrol-mesh-revoke-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            let peers = scratch.join("peers");
            let a = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2));
            let b = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 3));
            let a_identity = Identity::generate().unwrap();
            let b_identity = Identity::generate().unwrap();
            remember_peer_key(&peers, a, &a_identity.public).unwrap();
            remember_peer_key(&peers, b, &b_identity.public).unwrap();
            let (a_link, a_server) = test_mesh_link(a_identity);
            let (b_link, b_server) = test_mesh_link(b_identity);
            let mut links = BTreeMap::from([(a, a_link), (b, b_link)]);
            let coordinator = HandoffCoordinator::new(Default::default());
            reconcile_mesh_links(&mut links, &peers, &coordinator).unwrap();
            assert_eq!(links.len(), 2);

            revoke_peer_key(&peers, a).unwrap();
            reconcile_mesh_links(&mut links, &peers, &coordinator).unwrap();
            assert!(!links.contains_key(&a));
            assert_eq!(a_server.join().unwrap().unwrap(), 0);
            links
                .get_mut(&b)
                .unwrap()
                .send(Kind::Heartbeat, 0, Vec::new())
                .unwrap();
            assert_eq!(b_server.join().unwrap().unwrap(), 1);

            let mut topology = seamlesscontrol_core::topology::Topology::new();
            topology
                .place(Machine::Peer(b), Slot::new(1, 0).unwrap())
                .unwrap();
            let mut active = HandoffCoordinator::new(topology);
            active
                .activate_local_edge(LogicalEdge::Right, b, 1)
                .unwrap();
            revoke_peer_key(&peers, b).unwrap();
            assert!(reconcile_mesh_links(&mut links, &peers, &active).is_err());
            assert!(links.is_empty());
            std::fs::remove_dir_all(scratch).unwrap();
        }

        #[test]
        fn input_epoch_remains_monotonic_when_clock_is_behind() {
            assert_eq!(next_epoch(u64::MAX - 1).unwrap(), u64::MAX);
            assert!(next_epoch(u64::MAX).is_err());
            assert!(next_epoch(0).unwrap() > 0);
        }

        async fn wait_for_listener(address: SocketAddr, open: bool) {
            for _ in 0..100 {
                if TcpStream::connect(address).is_ok() == open {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
            panic!("listener at {address} did not become {open}");
        }

        #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
        async fn automatic_receiver_rebinds_after_address_loss_and_change() {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let scratch = std::env::temp_dir().join(format!(
                "seamlesscontrol-rebind-{}-{unique}",
                std::process::id()
            ));
            let config = scratch.join("config");
            let control_server =
                ControlServer::start_at(scratch.join("run/control.sock"), "serve", &config)
                    .unwrap();
            let port = TcpListener::bind("127.0.0.1:0")
                .unwrap()
                .local_addr()
                .unwrap()
                .port();
            let first: SocketAddr = (Ipv4Addr::new(127, 0, 0, 1), port).into();
            let second: SocketAddr = (Ipv4Addr::new(127, 0, 0, 2), port).into();
            TcpListener::bind(second).unwrap();
            let current = Arc::new(Mutex::new(None));
            let read_current = Arc::clone(&current);
            let control = control_server.handle();
            let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();
            let server = serve_receiver(
                ReceiverMode::Automatic,
                move || {
                    read_current
                        .lock()
                        .unwrap()
                        .ok_or_else(|| io::Error::from(io::ErrorKind::AddrNotAvailable))
                },
                Duration::from_millis(40),
                Identity::generate().unwrap(),
                config,
                control.clone(),
                async {
                    let _ = stop_rx.await;
                },
            );
            let driver = async {
                tokio::time::sleep(Duration::from_millis(80)).await;
                assert_eq!(control.phase(), "reconnecting");
                *current.lock().unwrap() = Some(first);
                wait_for_listener(first, true).await;
                *current.lock().unwrap() = None;
                wait_for_listener(first, false).await;
                *current.lock().unwrap() = Some(second);
                wait_for_listener(second, true).await;
                wait_for_listener(first, false).await;
                stop_tx.send(()).unwrap();
            };
            let (result, ()) = tokio::time::timeout(Duration::from_secs(5), async {
                tokio::join!(server, driver)
            })
            .await
            .unwrap();
            result.unwrap();
            wait_for_listener(second, false).await;
            drop(control_server);
            std::fs::remove_dir_all(scratch).unwrap();
        }

        #[test]
        fn authenticated_input_claim_is_exclusive_and_released_on_disconnect() {
            let scratch =
                std::env::temp_dir().join(format!("seamlesscontrol-claim-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&scratch);
            let config = scratch.join("server");
            let server_identity = Identity::generate().unwrap();
            let server_public = server_identity.public;
            let client_identity = Identity::generate().unwrap();
            let loopback = IpAddr::V4(Ipv4Addr::LOCALHOST);
            remember_peer_key(&config.join("peers"), loopback, &client_identity.public).unwrap();
            let control_server =
                ControlServer::start_at(scratch.join("run/control.sock"), "serve", &config)
                    .unwrap();
            let control = control_server.handle();
            let lease = control.claim_receiver("127.0.0.2").unwrap();
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let worker = thread::spawn(move || {
                let mut errors = Vec::new();
                for _ in 0..2 {
                    let (stream, _) = listener.accept().unwrap();
                    errors.push(
                        serve_connection(stream, loopback, &server_identity, &config, &control)
                            .unwrap_err()
                            .to_string(),
                    );
                }
                errors
            });
            let stream = TcpStream::connect(address).unwrap();
            let (mut channel, _) = SecureChannel::connect(
                stream,
                Role::Initiator,
                &client_identity,
                Some(&server_public),
                |_| false,
            )
            .unwrap();
            Frame {
                kind: Kind::Hello,
                epoch: 0,
                sequence: 0,
                payload: AGENT_PROTOCOL.to_vec(),
            }
            .write_to(&mut channel)
            .unwrap();
            assert_eq!(
                Frame::read_from(&mut channel).unwrap().payload,
                AGENT_PROTOCOL
            );
            Frame {
                kind: Kind::Control,
                epoch: 0,
                sequence: 0,
                payload: b"CLAIM".to_vec(),
            }
            .write_to(&mut channel)
            .unwrap();
            assert_eq!(Frame::read_from(&mut channel).unwrap().payload, b"BUSY");
            drop(channel);
            drop(lease);
            let stream = TcpStream::connect(address).unwrap();
            let (mut channel, _) = SecureChannel::connect(
                stream,
                Role::Initiator,
                &client_identity,
                Some(&server_public),
                |_| false,
            )
            .unwrap();
            Frame {
                kind: Kind::Hello,
                epoch: 0,
                sequence: 0,
                payload: AGENT_PROTOCOL.to_vec(),
            }
            .write_to(&mut channel)
            .unwrap();
            assert_eq!(
                Frame::read_from(&mut channel).unwrap().payload,
                AGENT_PROTOCOL
            );
            Frame {
                kind: Kind::Control,
                epoch: 0,
                sequence: 0,
                payload: b"CLAIM-MESH".to_vec(),
            }
            .write_to(&mut channel)
            .unwrap();
            assert_eq!(Frame::read_from(&mut channel).unwrap().payload, b"READY");
            drop(channel);
            let errors = worker.join().unwrap();
            assert!(errors[0].contains("already owns remote input"));
            assert!(errors[1].contains("I/O error"));
            assert!(
                control_server
                    .handle()
                    .claim_receiver("127.0.0.3")
                    .is_some()
            );
            drop(control_server);
            std::fs::remove_dir_all(scratch).unwrap();
        }
    }
}

#[cfg(target_os = "linux")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    linux::run().await
}

#[cfg(target_os = "windows")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    seamlesscontrol_core::windows_agent::run()
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
fn main() {
    eprintln!("SeamlessControl supports Omarchy and Windows desktops.");
}
