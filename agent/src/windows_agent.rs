//! Windows console agent. It shares the Omarchy wire protocol and trust store.
//! Input is sent to the currently signed-in, unlocked desktop session.

use crate::clipboard::{ClipboardPacket, ClipboardSync};
use crate::file_session;
use crate::protocol::{AGENT_PROTOCOL, EntryPosition, Frame, Kind, ReturnRequest};
use crate::receiver::run_receiver_with_first;
use crate::secure::{Identity, PeerInfo, Role, SecureChannel, SecureWriter};
use crate::storage::{
    is_revoked, key_fingerprint, list_peer_keys, load_or_create_identity, load_peer_key,
    relocate_peer_key, remember_peer_key, revoke_peer_key,
};
use crate::topology::Edge;
use crate::windows_capture::{CaptureEvent, CaptureHandle};
use crate::windows_clipboard::WindowsClipboard;
use crate::windows_input::{WindowsInjector, interactive_desktop, set_dpi_awareness};
use std::error::Error;
use std::io::{self, BufRead, Write};
use std::net::{IpAddr, Shutdown, SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{RecvTimeoutError, sync_channel};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const PAIRING_TIMEOUT: Duration = Duration::from_secs(330);
const DEFAULT_CONTROL_BIND: &str = "0.0.0.0:47832";
const DEFAULT_FILE_BIND: &str = "0.0.0.0:47833";

fn config_dir() -> Result<PathBuf, Box<dyn Error>> {
    let local = std::env::var_os("LOCALAPPDATA").ok_or("LOCALAPPDATA is not set")?;
    Ok(PathBuf::from(local).join("SeamlessControl"))
}

fn local_address(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(value) => value.is_private() || value.is_loopback() || value.is_link_local(),
        IpAddr::V6(value) => {
            value.is_loopback()
                || value.is_unicast_link_local()
                || value.segments()[0] & 0xfe00 == 0xfc00
        }
    }
}

fn confirm_pair(peer: &PeerInfo) -> bool {
    eprintln!("\nPAIRING CODE: {}", peer.sas);
    eprintln!("Compare it with the code on the Omarchy computer.");
    eprint!("If both match, type the six digits here and press Enter: ");
    if io::stderr().flush().is_err() {
        return false;
    }
    let mut answer = String::new();
    io::stdin().lock().read_line(&mut answer).is_ok() && answer.trim() == peer.sas
}

fn hello(channel: &mut SecureChannel<TcpStream>, role: Role) -> Result<(), Box<dyn Error>> {
    let greeting = Frame {
        kind: Kind::Hello,
        epoch: 0,
        sequence: 0,
        payload: AGENT_PROTOCOL.to_vec(),
    };
    match role {
        Role::Initiator => {
            greeting.write_to(channel)?;
            let reply = Frame::read_from(channel)?;
            if reply.kind != Kind::Hello || reply.payload != AGENT_PROTOCOL {
                return Err("the Omarchy agent uses an incompatible protocol".into());
            }
        }
        Role::Responder => {
            let request = Frame::read_from(channel)?;
            if request.kind != Kind::Hello || request.payload != AGENT_PROTOCOL {
                return Err("the source agent uses an incompatible protocol".into());
            }
            greeting.write_to(channel)?;
        }
    }
    Ok(())
}

fn pin_after_handshake(
    config: &Path,
    ip: IpAddr,
    peer: &PeerInfo,
    previous_ip: Option<IpAddr>,
) -> Result<(), Box<dyn Error>> {
    let peers = config.join("peers");
    if is_revoked(&peers, &peer.public_key)? {
        return Err("this peer identity was revoked".into());
    }
    if let Some(old) = previous_ip {
        relocate_peer_key(&peers, old, ip, &peer.public_key)?;
    } else {
        remember_peer_key(&peers, ip, &peer.public_key)?;
    }
    Ok(())
}

fn secure_connection(
    stream: TcpStream,
    role: Role,
    identity: &Identity,
    config: &Path,
    ip: IpAddr,
    prompt: &Mutex<()>,
) -> Result<SecureChannel<TcpStream>, Box<dyn Error>> {
    let peers = config.join("peers");
    let pinned = load_peer_key(&peers, ip)?;
    let known = if pinned.is_none() {
        list_peer_keys(&peers)?
    } else {
        Vec::new()
    };
    let mut previous_ip = None;
    stream.set_nodelay(true)?;
    stream.set_read_timeout(Some(PAIRING_TIMEOUT))?;
    stream.set_write_timeout(Some(PAIRING_TIMEOUT))?;
    let (mut channel, peer) =
        SecureChannel::connect(stream, role, identity, pinned.as_ref(), |candidate| {
            if is_revoked(&peers, &candidate.public_key).unwrap_or(true) {
                return false;
            }
            if let Some((old, _)) = known.iter().find(|(_, key)| *key == candidate.public_key) {
                previous_ip = Some(*old);
                return true;
            }
            let Ok(_prompt_guard) = prompt.lock() else {
                return false;
            };
            confirm_pair(candidate)
        })?;
    channel
        .stream_mut()
        .set_read_timeout(Some(Duration::from_secs(120)))?;
    channel
        .stream_mut()
        .set_write_timeout(Some(Duration::from_secs(120)))?;
    hello(&mut channel, role)?;
    pin_after_handshake(config, ip, &peer, previous_ip)?;
    Ok(channel)
}

struct ReceiverLease(Arc<AtomicBool>);

impl ReceiverLease {
    fn claim(occupied: &Arc<AtomicBool>) -> Option<Self> {
        occupied
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .ok()
            .map(|_| Self(Arc::clone(occupied)))
    }
}

impl Drop for ReceiverLease {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

fn serve_connection(
    stream: TcpStream,
    ip: IpAddr,
    identity: &Identity,
    config: &Path,
    occupied: &Arc<AtomicBool>,
    prompt: &Mutex<()>,
) -> Result<(), Box<dyn Error>> {
    if !local_address(ip) {
        return Err("remote control is restricted to LAN peers".into());
    }
    let mut channel = secure_connection(stream, Role::Responder, identity, config, ip, prompt)?;
    let first = Frame::read_from(&mut channel)?;
    if first.kind == Kind::Control
        && first.epoch == 0
        && first.sequence == 0
        && first.payload == b"PAIR"
    {
        Frame {
            kind: Kind::Control,
            epoch: 0,
            sequence: 0,
            payload: b"PAIRED".to_vec(),
        }
        .write_to(&mut channel)?;
        println!("Paired with {ip}. Input control has not started.");
        return Ok(());
    }
    if first.kind == Kind::Control && first.payload == b"PING" {
        let mut request = first;
        for sequence in 1..=20 {
            if request.kind != Kind::Control
                || request.epoch != 0
                || request.sequence != sequence
                || request.payload != b"PING"
            {
                return Err("invalid latency probe".into());
            }
            Frame {
                kind: Kind::Control,
                epoch: 0,
                sequence,
                payload: b"PONG".to_vec(),
            }
            .write_to(&mut channel)?;
            if sequence < 20 {
                request = Frame::read_from(&mut channel)?;
            }
        }
        return Ok(());
    }
    if first.kind != Kind::Control
        || first.epoch != 0
        || first.sequence != 0
        || (first.payload != b"CLAIM" && first.payload != b"CLAIM-MESH")
    {
        return Err("input requires a capture claim".into());
    }
    let Some(_lease) = ReceiverLease::claim(occupied) else {
        Frame {
            kind: Kind::Control,
            epoch: 0,
            sequence: 0,
            payload: b"BUSY".to_vec(),
        }
        .write_to(&mut channel)?;
        return Ok(());
    };
    if !interactive_desktop()? {
        return Err("Windows desktop is locked; remote input was not enabled".into());
    }
    Frame {
        kind: Kind::Control,
        epoch: 0,
        sequence: 0,
        payload: b"READY".to_vec(),
    }
    .write_to(&mut channel)?;
    channel
        .stream_mut()
        .set_read_timeout(Some(PAIRING_TIMEOUT))?;
    let first_input = Frame::read_from(&mut channel)?;
    channel
        .stream_mut()
        .set_read_timeout(Some(Duration::from_secs(5)))?;
    let (mut reader, writer) = channel.into_tcp_halves()?;
    let pinned = load_peer_key(&config.join("peers"), ip)?.ok_or("peer was removed")?;
    let injector = WindowsInjector::new(writer, identity.public, pinned)?;
    println!("Input session from {ip} is ready.");
    run_receiver_with_first(&mut reader, injector, Some(first_input))?;
    println!("Input session from {ip} ended; held keys and buttons were released.");
    Ok(())
}

fn serve(bind: SocketAddr, identity: Identity, config: PathBuf) -> Result<(), Box<dyn Error>> {
    set_dpi_awareness();
    let listener = TcpListener::bind(bind)?;
    let occupied = Arc::new(AtomicBool::new(false));
    let prompt = Arc::new(Mutex::new(()));
    println!(
        "SeamlessControl Windows receiver listening on {}",
        listener.local_addr()?
    );
    println!("Allow this port on your Private Windows network if the firewall prompts.");
    for connection in listener.incoming() {
        let stream = connection?;
        let address = stream.peer_addr()?;
        let identity = identity.clone();
        let config = config.clone();
        let occupied = Arc::clone(&occupied);
        let prompt = Arc::clone(&prompt);
        thread::spawn(move || {
            if let Err(error) =
                serve_connection(stream, address.ip(), &identity, &config, &occupied, &prompt)
            {
                eprintln!("SeamlessControl connection from {address}: {error}");
            }
        });
    }
    Ok(())
}

fn pair(address: SocketAddr, identity: &Identity, config: &Path) -> Result<(), Box<dyn Error>> {
    let stream = TcpStream::connect_timeout(&address, Duration::from_secs(10))?;
    let prompt = Mutex::new(());
    let mut channel = secure_connection(
        stream,
        Role::Initiator,
        identity,
        config,
        address.ip(),
        &prompt,
    )?;
    Frame {
        kind: Kind::Control,
        epoch: 0,
        sequence: 0,
        payload: b"PAIR".to_vec(),
    }
    .write_to(&mut channel)?;
    let reply = Frame::read_from(&mut channel)?;
    if reply.kind != Kind::Control || reply.payload != b"PAIRED" {
        return Err("the peer did not acknowledge pairing".into());
    }
    println!("Paired with {address}.");
    Ok(())
}

fn send_source_frame(
    writer: &mut SecureWriter<TcpStream>,
    sequence: &mut u64,
    kind: Kind,
    epoch: u64,
    payload: Vec<u8>,
) -> Result<(), Box<dyn Error>> {
    *sequence = sequence.checked_add(1).ok_or("source sequence exhausted")?;
    Frame {
        kind,
        epoch,
        sequence: *sequence,
        payload,
    }
    .write_to(writer)?;
    Ok(())
}

fn next_epoch(previous: u64) -> Result<u64, Box<dyn Error>> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos() as u64;
    Ok(now
        .max(previous.checked_add(1).ok_or("input epoch exhausted")?)
        .max(1))
}

fn connect_source(
    address: SocketAddr,
    edge: Edge,
    identity: &Identity,
    config: &Path,
) -> Result<(), Box<dyn Error>> {
    if !local_address(address.ip()) {
        return Err("remote control is restricted to LAN peers".into());
    }
    if !interactive_desktop()? {
        return Err("Windows desktop must be unlocked to capture input".into());
    }
    set_dpi_awareness();
    let pinned = load_peer_key(&config.join("peers"), address.ip())?
        .ok_or("pair with this Omarchy receiver before connecting")?;
    let stream = TcpStream::connect_timeout(&address, Duration::from_secs(8))?;
    stream.set_nodelay(true)?;
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    let (mut channel, peer) =
        SecureChannel::connect(stream, Role::Initiator, identity, Some(&pinned), |_| false)?;
    hello(&mut channel, Role::Initiator)?;
    Frame {
        kind: Kind::Control,
        epoch: 0,
        sequence: 0,
        payload: b"CLAIM".to_vec(),
    }
    .write_to(&mut channel)?;
    let answer = Frame::read_from(&mut channel)?;
    if answer.kind == Kind::Control && answer.payload == b"BUSY" {
        return Err(
            "Omarchy is already controlling or receiving input; end that session first".into(),
        );
    }
    if answer.kind != Kind::Control
        || answer.epoch != 0
        || answer.sequence != 0
        || answer.payload != b"READY"
    {
        return Err("Omarchy did not accept the input claim".into());
    }
    channel.stream_mut().set_read_timeout(None)?;
    let (capture_tx, capture_rx) = sync_channel(4096);
    let capture = CaptureHandle::start(edge, capture_tx)?;
    let (mut reader, mut writer) = channel.into_tcp_halves()?;
    let mut sequence = 0;
    send_source_frame(&mut writer, &mut sequence, Kind::Heartbeat, 0, Vec::new())?;
    let (feedback_tx, feedback_rx) = sync_channel(32);
    let feedback_reader = thread::spawn(move || {
        loop {
            let frame = Frame::read_from(&mut reader);
            let done = frame.is_err();
            if feedback_tx.send(frame).is_err() || done {
                break;
            }
        }
    });
    let mut clipboard = WindowsClipboard::new();
    let mut clipboard_sync = ClipboardSync::new(identity.public, peer.public_key);
    let mut heartbeat = Instant::now();
    let mut clipboard_tick = Instant::now();
    let mut desktop_tick = Instant::now();
    let mut epoch = None;
    let mut previous_epoch = 0;
    println!("Ready to control {address}. Move the mouse across the {edge:?} outer screen edge.");
    println!("Move back across the entry edge on Omarchy, or press Escape on Windows, to return.");
    let result = (|| -> Result<(), Box<dyn Error>> {
        loop {
            if capture.failed() {
                return Err("Windows input capture stopped or its event queue overflowed".into());
            }
            if desktop_tick.elapsed() >= Duration::from_millis(300) {
                desktop_tick = Instant::now();
                if !interactive_desktop()? {
                    return Err("Windows desktop locked; remote input stopped".into());
                }
            }
            while let Ok(frame) = feedback_rx.try_recv() {
                let frame = frame?;
                match frame.kind {
                    Kind::Control if frame.payload == b"ENDED" => {}
                    Kind::Control
                        if frame.payload == b"RETURN" || frame.payload.starts_with(b"RETURN\t") =>
                    {
                        if epoch == Some(frame.epoch) {
                            if frame.payload != b"RETURN" {
                                let request = ReturnRequest::parse(&frame.payload)?;
                                if request.exit_edge != edge.opposite() {
                                    return Err("remote return edge does not match the Windows capture edge".into());
                                }
                            }
                            send_source_frame(
                                &mut writer,
                                &mut sequence,
                                Kind::Control,
                                frame.epoch,
                                b"END".to_vec(),
                            )?;
                            epoch = None;
                            capture.release();
                            println!("Control returned to Windows.");
                        }
                    }
                    Kind::Clipboard => {
                        let packet = ClipboardPacket::decode(&frame.payload)?;
                        if clipboard_sync.remote_needs_apply(&packet)? {
                            if let Err(error) = clipboard.apply(&packet.event) {
                                eprintln!("SeamlessControl: clipboard apply failed: {error}");
                            } else {
                                clipboard_sync.remote_applied(&packet);
                            }
                        }
                    }
                    _ => return Err("unexpected feedback from Omarchy".into()),
                }
            }
            match capture_rx.recv_timeout(Duration::from_millis(20)) {
                Ok(CaptureEvent::Begin(fraction)) if epoch.is_none() => {
                    previous_epoch = next_epoch(previous_epoch)?;
                    send_source_frame(
                        &mut writer,
                        &mut sequence,
                        Kind::Control,
                        previous_epoch,
                        EntryPosition {
                            edge: edge.opposite(),
                            fraction,
                        }
                        .begin_payload(),
                    )?;
                    epoch = Some(previous_epoch);
                    println!("Controlling Omarchy. Escape returns to Windows.");
                }
                Ok(CaptureEvent::Input(event)) => {
                    if let Some(active) = epoch {
                        send_source_frame(
                            &mut writer,
                            &mut sequence,
                            Kind::Input,
                            active,
                            event.encode(),
                        )?;
                    }
                }
                Ok(CaptureEvent::Release) => {
                    if let Some(active) = epoch.take() {
                        send_source_frame(
                            &mut writer,
                            &mut sequence,
                            Kind::Control,
                            active,
                            b"END".to_vec(),
                        )?;
                        println!("Control returned to Windows.");
                    }
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {
                    return Err("Windows capture thread stopped".into());
                }
                _ => {}
            }
            if heartbeat.elapsed() >= Duration::from_secs(2) {
                send_source_frame(
                    &mut writer,
                    &mut sequence,
                    Kind::Heartbeat,
                    epoch.unwrap_or(0),
                    Vec::new(),
                )?;
                heartbeat = Instant::now();
            }
            if clipboard_tick.elapsed() >= Duration::from_millis(500) {
                clipboard_tick = Instant::now();
                match clipboard.changed() {
                    Ok(Some(event)) => {
                        if let Some(packet) = clipboard_sync.local_changed(&event) {
                            send_source_frame(
                                &mut writer,
                                &mut sequence,
                                Kind::Clipboard,
                                0,
                                packet.encode(),
                            )?;
                        }
                    }
                    Ok(None) => {}
                    Err(error) => eprintln!("SeamlessControl: clipboard read failed: {error}"),
                }
            }
        }
    })();
    drop(capture);
    let _ = writer.stream_mut().shutdown(Shutdown::Both);
    drop(feedback_rx);
    let _ = feedback_reader.join();
    result
}

fn usage() {
    eprintln!(
        "SeamlessControl for Windows (alpha)\n\
         seamlesscontrold.exe serve [BIND_IP:PORT]         Receive Omarchy input (default {DEFAULT_CONTROL_BIND})\n\
         seamlesscontrold.exe pair <OMARCHY_IP:PORT>      Pair with an Omarchy receiver\n\
         seamlesscontrold.exe connect <OMARCHY_IP:PORT> <left|right|top|bottom>\n\
         seamlesscontrold.exe receive-file [BIND_IP:PORT] <DIRECTORY>\n\
         seamlesscontrold.exe send-file <PEER_IP:PORT> <FILE>\n\
         seamlesscontrold.exe identity                    Show this computer's identity\n\
         seamlesscontrold.exe peers                       List paired computers\n\
         seamlesscontrold.exe revoke <PEER_IP>            Revoke a computer"
    );
}

pub fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    let config = config_dir()?;
    let identity = load_or_create_identity(&config.join("identity"))?;
    let peers = config.join("peers");
    match args.as_slice() {
        [_, command] if command == "identity" => {
            println!("Local identity: {}", key_fingerprint(&identity.public));
        }
        [_, command] if command == "peers" => {
            for (ip, key) in list_peer_keys(&peers)? {
                println!("PEER\t{ip}\t{}", key_fingerprint(&key));
            }
        }
        [_, command, ip] if command == "revoke" => {
            let ip: IpAddr = ip.parse()?;
            revoke_peer_key(&peers, ip)?;
            println!("Revoked {ip}. It must be paired again with a new identity.");
        }
        [_, command] if command == "serve" => {
            serve(DEFAULT_CONTROL_BIND.parse()?, identity, config)?;
        }
        [_, command, bind] if command == "serve" => {
            serve(bind.parse()?, identity, config)?;
        }
        [_, command, address] if command == "pair" => {
            pair(address.parse()?, &identity, &config)?;
        }
        [_, command, address, edge] if command == "connect" => {
            let edge = Edge::parse(edge).ok_or("edge must be left, right, top or bottom")?;
            connect_source(address.parse()?, edge, &identity, &config)?;
        }
        [_, command, address, source] if command == "send-file" => {
            file_session::send_once(
                address.parse()?,
                Path::new(source),
                &identity,
                &peers,
                file_session::configured_limit()?,
            )?;
            println!("File delivered and verified.");
        }
        [_, command, directory] if command == "receive-file" => {
            receive_file(
                DEFAULT_FILE_BIND.parse()?,
                Path::new(directory),
                &identity,
                &peers,
            )?;
        }
        [_, command, bind, directory] if command == "receive-file" => {
            receive_file(bind.parse()?, Path::new(directory), &identity, &peers)?;
        }
        _ => {
            usage();
            return Err("invalid Windows agent command".into());
        }
    }
    Ok(())
}

fn receive_file(
    address: SocketAddr,
    directory: &Path,
    identity: &Identity,
    peers: &Path,
) -> Result<(), Box<dyn Error>> {
    let saved = file_session::receive_once(
        address,
        directory,
        identity,
        peers,
        file_session::configured_limit()?,
        file_session::terminal_approval,
        |listening| {
            println!("Waiting for one file on {listening}");
            Ok(())
        },
    )?;
    if let Some(path) = saved {
        println!("Saved verified file: {}", path.display());
    } else {
        println!("File was declined.");
    }
    Ok(())
}
