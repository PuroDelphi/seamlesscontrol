//! Windows console agent. It shares the Omarchy wire protocol and trust store.
//! Input is sent to the currently signed-in, unlocked desktop session.

use crate::file_session;
use crate::protocol::{AGENT_PROTOCOL, Frame, Kind};
use crate::receiver::run_receiver_with_first;
use crate::secure::{Identity, PeerInfo, Role, SecureChannel};
use crate::storage::{
    is_revoked, key_fingerprint, list_peer_keys, load_or_create_identity, load_peer_key,
    relocate_peer_key, remember_peer_key, revoke_peer_key,
};
use crate::windows_input::{WindowsInjector, set_dpi_awareness};
use std::error::Error;
use std::io::{self, BufRead, Write};
use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

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
    let injector = WindowsInjector::new(writer, pinned)?;
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

fn usage() {
    eprintln!(
        "SeamlessControl for Windows (alpha)\n\
         seamlesscontrold.exe serve [BIND_IP:PORT]         Receive Omarchy input (default {DEFAULT_CONTROL_BIND})\n\
         seamlesscontrold.exe pair <OMARCHY_IP:PORT>      Pair with an Omarchy receiver\n\
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
