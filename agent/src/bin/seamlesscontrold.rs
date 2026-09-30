#[cfg(target_os = "linux")]
mod linux {
    use ashpd::desktop::Session;
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
    use seamlesscontrol_core::clipboard_omarchy::{
        self, ClipboardWatch, spawn_apply_events, spawn_apply_worker,
    };
    use seamlesscontrol_core::control::{self, ControlHandle, ControlServer};
    use seamlesscontrol_core::file_session;
    use seamlesscontrol_core::handoff::HandoffCoordinator;
    use seamlesscontrol_core::hypr_ipc::{HyprIpc, SessionLockState};
    use seamlesscontrol_core::omarchy::VirtualInput;
    use seamlesscontrol_core::protocol::{Frame, FrameError, Kind};
    use seamlesscontrol_core::receiver::{Injector, run_receiver_with_first_until};
    use seamlesscontrol_core::secure::{Identity, Role, SecureChannel, SecureError, SecureWriter};
    use seamlesscontrol_core::state::InputEvent;
    use seamlesscontrol_core::storage::{
        is_revoked, key_fingerprint, list_peer_keys, load_or_create_identity, load_peer_key,
        load_topology, remember_peer_key, revoke_peer_key, save_topology,
    };
    use seamlesscontrol_core::topology::{
        Edge as LogicalEdge, EdgeReturnDetector, Machine, Rect, Slot, external_barriers,
    };
    use std::collections::{BTreeMap, BTreeSet, HashMap};
    use std::error::Error;
    use std::io::{self, Write};
    use std::net::{IpAddr, Shutdown, SocketAddr, TcpListener, TcpStream};
    use std::os::unix::net::UnixStream;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    const PAIRING_SOCKET_TIMEOUT: Duration = Duration::from_secs(330);
    const AGENT_PROTOCOL: &[u8] = b"seamlesscontrol/4";
    const LATENCY_SAMPLES: u64 = 20;
    const MAX_INBOUND_CONNECTIONS: usize = 8;

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
        Latency,
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
        feedback: tokio::sync::mpsc::Sender<(IpAddr, Result<Frame, FrameError>)>,
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
        let reader = thread::spawn(move || {
            loop {
                let frame = Frame::read_from(&mut reader);
                let done = frame.is_err();
                if feedback.blocking_send((peer_ip, frame)).is_err() || done {
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
        Ok(now.max(previous.saturating_add(1)).max(1))
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
            tokio::sync::mpsc::channel::<(IpAddr, Result<Frame, FrameError>)>(32);
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
        let mut coordinator = HandoffCoordinator::new(topology.clone());
        let mut heartbeat = tokio::time::interval(Duration::from_secs(2));
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
        let result: Result<(), Box<dyn Error>> = async {
            loop {
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => break,
                    _ = pause_tick.tick() => {
                        if control.revoked_active() { return Err("active mesh peer was revoked".into()); }
                        locked = lock_ipc.session_lock_state().unwrap_or(SessionLockState::Undetermined) != SessionLockState::Unlocked;
                        if pending_since.is_some_and(|since| since.elapsed() > Duration::from_secs(2)) {
                            return Err("mesh handoff acknowledgement timed out".into());
                        }
                        if control.paused() || locked {
                            if let (Machine::Peer(peer), Some(epoch)) = (coordinator.owner(), coordinator.epoch()) {
                                let _ = mesh_link(&mut links, peer)?.send(Kind::Control, epoch, b"END".to_vec());
                                coordinator.reset_local();
                                keys.clear(); buttons.clear(); pending_since = None;
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
                        for link in links.values_mut() { link.send(Kind::Heartbeat, 0, Vec::new())?; }
                    }
                    Some(event) = clipboard_rx.recv() => {
                        if !locked && let Some(packet) = clipboard_mesh.local_changed(event)? {
                            for link in links.values_mut() {
                                link.send(Kind::Clipboard, 0, packet.encode())?;
                            }
                        }
                    }
                    feedback = feedback_rx.recv() => {
                        let (from, frame) = feedback.ok_or("mesh feedback channel closed")?;
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
                            let next = next_epoch(last_epoch)?;
                            let target = coordinator.acknowledge(from, frame.epoch, next)?;
                            pending_since = None;
                            match target {
                                Machine::Peer(peer) => {
                                    last_epoch = next;
                                    let link = mesh_link(&mut links, peer)?;
                                    link.send(Kind::Control, next, b"BEGIN".to_vec())?;
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
                        } else if frame.payload == b"RETURN" {
                            if coordinator.owner() == Machine::Peer(from) && coordinator.epoch() == Some(frame.epoch) {
                                mesh_link(&mut links, from)?.send(Kind::Control, frame.epoch, b"END".to_vec())?;
                                coordinator.reset_local(); pending_since = None; keys.clear(); buttons.clear();
                                let mut options = ReleaseOptions::default().set_activation_id(current_activation.take());
                                if let Some(position) = release_position { options = options.set_cursor_position(position); }
                                portal.release(&session, options).await?;
                                portal_active = false;
                                control.set_peer(""); control.set_phase("ready");
                            }
                        } else if let Some(target) = frame.payload.strip_prefix(b"SWITCH\t") {
                            let target: IpAddr = std::str::from_utf8(target)?.parse()?;
                            if coordinator.owner() != Machine::Peer(from) || coordinator.epoch() != Some(frame.epoch) {
                                return Err("stale mesh switch request".into());
                            }
                            coordinator.request(from, frame.epoch, Machine::Peer(target))?;
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
                        mesh_link(&mut links, peer)?.send(Kind::Control, last_epoch, b"BEGIN".to_vec())?;
                        control.set_peer(&peer.to_string()); control.set_phase("controlling");
                        println!("Control remoto activo en {peer}.");
                    }
                    signal = deactivated.next() => {
                        if signal.is_none() { return Err("capture deactivation stream closed".into()); }
                        if let (Machine::Peer(peer), Some(epoch)) = (coordinator.owner(), coordinator.epoch()) {
                            mesh_link(&mut links, peer)?.send(Kind::Control, epoch, b"END".to_vec())?;
                        }
                        coordinator.reset_local(); pending_since = None; keys.clear(); buttons.clear(); current_activation = None;
                        portal_active = false;
                        control.set_peer(""); control.set_phase("ready");
                    }
                    signal = zones_changed.next() => {
                        let signal = signal.ok_or("capture zones stream closed")?;
                        if signal.zone_set().is_some_and(|id| id != zone_set) { continue; }
                        if let (Machine::Peer(peer), Some(epoch)) = (coordinator.owner(), coordinator.epoch()) {
                            mesh_link(&mut links, peer)?.send(Kind::Control, epoch, b"END".to_vec())?;
                        }
                        coordinator.reset_local(); pending_since = None; keys.clear(); buttons.clear(); current_activation = None;
                        portal_active = false;
                        if capture_enabled { portal.disable(&session, Default::default()).await?; capture_enabled = false; }
                        (zone_set, barriers) = install_mesh_barriers(&portal, &session, &topology).await?;
                        if !control.paused() && !locked { portal.enable(&session, Default::default()).await?; capture_enabled = true; }
                        control.set_peer(""); control.set_phase(if locked { "locked" } else if control.paused() { "paused" } else { "ready" });
                    }
                    event = events.next() => {
                        let event = event.ok_or("EIS event stream closed")??;
                        if let EiEvent::SeatAdded(seat) = &event {
                            seat.seat.bind_capabilities(DeviceCapability::Pointer | DeviceCapability::Keyboard | DeviceCapability::Scroll | DeviceCapability::Button);
                            context.flush()?;
                        }
                        if let (Machine::Peer(peer), Some(epoch)) = (coordinator.owner(), coordinator.epoch()) {
                            if matches!(&event, EiEvent::KeyboardKey(key) if key.key == 1 && key.state == ei::keyboard::KeyState::Press) {
                                mesh_link(&mut links, peer)?.send(Kind::Control, epoch, b"END".to_vec())?;
                                coordinator.reset_local(); pending_since = None; keys.clear(); buttons.clear();
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
        let mut pause_tick = tokio::time::interval(Duration::from_millis(100));
        pause_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut capture_enabled = !locked;
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
                            portal.enable(&session, Default::default()).await?;
                            capture_enabled = true;
                            control.set_phase("ready");
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
                        if frame.kind != Kind::Control || frame.payload != b"RETURN" {
                            return Err("unexpected feedback from remote peer".into());
                        }
                        if active && frame.epoch == epoch {
                            send_frame(&mut writer, Kind::Control, epoch, &mut sequence, b"END".to_vec())?;
                            let mut options = ReleaseOptions::default().set_activation_id(current_activation.take());
                            if let Some(position) = release_position { options = options.set_cursor_position(position); }
                            portal.release(&session, options).await?;
                            active = false;
                            control.set_phase(if locked { "locked" } else { "ready" });
                            println!("El equipo remoto devolvió el control local.");
                        }
                    }
                    signal = activated.next() => {
                        let signal = signal.ok_or("capture activation stream closed")?;
                        if control.paused() || locked {
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
                        send_frame(&mut writer, Kind::Control, epoch, &mut sequence, b"BEGIN".to_vec())?;
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
                            active = false;
                            current_activation = None;
                        }
                        if capture_enabled {
                            portal.disable(&session, Default::default()).await?;
                            capture_enabled = false;
                        }
                        current_zone_set = install_barriers(&portal, &session, edge).await?;
                        if !control.paused() && !locked {
                            portal.enable(&session, Default::default()).await?;
                            capture_enabled = true;
                            control.set_phase("ready");
                        } else {
                            control.set_phase(if locked { "locked" } else { "paused" });
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
                                control.set_phase(if locked { "locked" } else { "ready" });
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
    }

    impl Injector for OmarchyInjector {
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
        remember_peer_key(&peers, peer_ip, &peer.public_key)?;
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
        let edge_hypr = (!edge_targets.is_empty()).then(|| hypr.clone());
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
        };
        let watcher_running = Arc::new(AtomicBool::new(true));
        let watcher_flag = Arc::clone(&watcher_running);
        let watcher_control = control.clone();
        let watcher = thread::spawn(move || {
            let mut sequence = 0_u64;
            let mut observed_epoch = None;
            let mut sent_epoch = None;
            let mut observed_motion = 0_u64;
            let mut detectors: Vec<(IpAddr, EdgeReturnDetector)> = Vec::new();
            let mut monitor_regions: Option<Vec<Rect>> = None;
            let mut last_geometry_refresh = Instant::now();
            while watcher_flag.load(Ordering::Relaxed) {
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
                    last_geometry_refresh = Instant::now();
                    monitor_regions = if active_epoch.is_some() {
                        edge_hypr.as_ref().and_then(|ipc| ipc.monitor_rects().ok())
                    } else {
                        None
                    };
                    detectors = monitor_regions
                        .as_ref()
                        .map(|regions| {
                            edge_targets
                                .iter()
                                .map(|(edge, target)| {
                                    (*target, EdgeReturnDetector::new(regions, *edge))
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
                        detectors = edge_targets
                            .iter()
                            .map(|(edge, target)| {
                                (*target, EdgeReturnDetector::new(&regions, *edge))
                            })
                            .collect();
                        monitor_regions = Some(regions);
                    }
                }
                let manual_return = watcher_control
                    .take_return_request()
                    .map(|epoch| (epoch, b"RETURN".to_vec()));
                let motion = motion_generation.load(Ordering::Relaxed);
                let automatic_return = if sent_epoch.is_none() && motion != observed_motion {
                    observed_motion = motion;
                    match (active_epoch, edge_hypr.as_ref()) {
                        (Some(epoch), Some(ipc)) => {
                            ipc.cursor_position().ok().and_then(|(x, y)| {
                                detectors.iter_mut().find_map(|(target, detector)| {
                                    if !detector.sample(x, y) {
                                        return None;
                                    }
                                    let payload = if *target == peer_ip {
                                        b"RETURN".to_vec()
                                    } else {
                                        format!("SWITCH\t{target}").into_bytes()
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
            control.revoked_active()
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
            payload: AGENT_PROTOCOL.to_vec(),
        }
        .write_to(&mut channel)
        .map_err(frame_attempt)?;
        let greeting = Frame::read_from(&mut channel).map_err(frame_attempt)?;
        if greeting.kind != Kind::Hello || greeting.payload != AGENT_PROTOCOL {
            return Err(AttemptError::Stop("incompatible peer protocol".into()));
        }
        remember_peer_key(&peers, address.ip(), &peer.public_key)
            .map_err(|e| AttemptError::Stop(Box::new(e)))?;
        channel
            .stream_mut()
            .set_write_timeout(Some(Duration::from_secs(5)))
            .map_err(|e| AttemptError::Retry(Box::new(e)))?;
        match mode {
            ConnectionMode::Capture(edge) => {
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
                capture_loop(
                    channel,
                    edge,
                    control.clone(),
                    identity.public,
                    peer.public_key,
                )
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

    pub async fn run() -> Result<(), Box<dyn Error>> {
        let args: Vec<String> = std::env::args().collect();
        if args.len() == 2 && args[1] == "clipboard-helper" {
            clipboard_omarchy::emit_watched_event()?;
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
        if args.len() == 4
            && matches!(
                args[1].as_str(),
                "send-file" | "receive-file" | "receive-file-ui"
            )
        {
            let address: SocketAddr = args[2].parse()?;
            if !local_address(address.ip()) {
                return Err(
                    "file transfer address must be loopback, link-local or private LAN".into(),
                );
            }
            let config = config_dir()?;
            let identity = load_or_create_identity(&config.join("identity"))?;
            let limit = file_session::configured_limit()?;
            if args[1] == "send-file" {
                file_session::send_once(
                    address,
                    std::path::Path::new(&args[3]),
                    &identity,
                    &config.join("peers"),
                    limit,
                )?;
                println!("Archivo entregado y verificado por el destino.");
            } else {
                let result = file_session::receive_once(
                    address,
                    std::path::Path::new(&args[3]),
                    &identity,
                    &config.join("peers"),
                    limit,
                    if args[1] == "receive-file-ui" {
                        file_session::panel_approval
                    } else {
                        file_session::terminal_approval
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
                "status" | "pause" | "resume" | "reject" | "return"
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
            && matches!(args[1].as_str(), "serve" | "pair" | "connect" | "latency")
            || args.len() == 4 && args[1] == "connect")
        {
            eprintln!(
                "Uso: seamlesscontrold serve <IP-LAN:PUERTO>\n     seamlesscontrold pair <IP-LAN:PUERTO>\n     seamlesscontrold connect <IP-LAN:PUERTO> [left|right|top|bottom]\n     seamlesscontrold mesh <PUERTO>\n     seamlesscontrold latency <IP-LAN:PUERTO>\n     seamlesscontrold receive-file <IP-LAN:PUERTO> <directorio>\n     seamlesscontrold send-file <IP-LAN:PUERTO> <archivo>"
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
        if args[1] == "latency" && load_peer_key(&config.join("peers"), address.ip())?.is_none() {
            return Err("pair this peer before measuring latency".into());
        }
        if matches!(args[1].as_str(), "connect" | "pair" | "latency") {
            let _local_control = ControlServer::start("connect", &config)?;
            let control = _local_control.handle();
            let mode = if args[1] == "connect" && args.len() == 4 {
                ConnectionMode::Capture(Edge::parse(&args[3])?)
            } else if args[1] == "connect" {
                let topology = load_topology(&config.join("topology"))?;
                ConnectionMode::Capture(Edge::from_logical(topology.edge_to(address.ip())?))
            } else if args[1] == "latency" {
                ConnectionMode::Latency
            } else {
                ConnectionMode::Pair
            };
            let mut delay = Duration::from_secs(1);
            loop {
                match connect_once(address, mode, &identity, &config, &control).await {
                    Ok(()) => return Ok(()),
                    Err(AttemptError::Stop(error)) => return Err(error),
                    Err(AttemptError::Retry(error))
                        if !matches!(mode, ConnectionMode::Capture(_)) =>
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
        let listener = TcpListener::bind(address)?;
        listener.set_nonblocking(true)?;
        let listener = tokio::net::TcpListener::from_std(listener)?;
        println!("SeamlessControl escucha en {address}");
        let stop = tokio::signal::ctrl_c();
        tokio::pin!(stop);
        let connections = Arc::new(Mutex::new(HashMap::<u64, TcpStream>::new()));
        let limit = Arc::new(tokio::sync::Semaphore::new(MAX_INBOUND_CONNECTIONS));
        let mut workers = tokio::task::JoinSet::new();
        let mut next_connection_id = 0_u64;
        loop {
            tokio::select! {
                _ = &mut stop => {
                    control.cancel_pair();
                    if let Ok(open) = connections.lock() {
                        for stream in open.values() {
                            let _ = stream.shutdown(Shutdown::Both);
                        }
                    }
                    break;
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
        while let Some(result) = workers.join_next().await {
            if let Err(error) = result {
                eprintln!("Error al cerrar un trabajador de red: {error}");
            }
        }
        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::net::Ipv4Addr;

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

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("El receptor Omarchy sólo se compila en Linux por ahora.");
}
