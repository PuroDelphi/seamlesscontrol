//! One-shot encrypted file transfer. Pair the computers first; each received
//! file requires a fresh local confirmation before any bytes are written.

use crate::file_bundle;
use crate::file_transfer::{
    DEFAULT_MAX_FILE_BYTES, FileMessage, FileOffer, FileReceiver, FileSender,
};
use crate::peer_policy::{Capability, PeerPolicy};
use crate::protocol::{Frame, Kind};
use crate::secure::{Identity, Role, SecureChannel};
use crate::storage::load_peer_key;
use std::error::Error;
use std::io::{self, BufRead, Write};
use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream};
use std::path::Path;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

const GREETING: &[u8] = b"seamlesscontrol-file/1";
const FILE_TIMEOUT: Duration = Duration::from_secs(300);
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);

struct BundleStage(std::path::PathBuf);

impl Drop for BundleStage {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn bundle_stage(directory: &Path) -> io::Result<BundleStage> {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(io::Error::other)?
        .as_nanos();
    let path = directory.join(format!(
        ".seamlesscontrol-group-{}-{nonce}",
        std::process::id()
    ));
    std::fs::create_dir(&path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700))?;
    }
    Ok(BundleStage(path))
}

fn publish_bundle(
    bundle: &Path,
    directory: &Path,
    stage: &BundleStage,
    count: usize,
    limit: u64,
) -> io::Result<std::path::PathBuf> {
    let unpacked = file_bundle::extract_bundle(bundle, &stage.0, count, limit)?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(io::Error::other)?
        .as_nanos();
    let target = directory.join(format!("SeamlessControl received group {nonce}"));
    std::fs::rename(unpacked, &target)?;
    Ok(target)
}

fn send_frame(
    channel: &mut SecureChannel<TcpStream>,
    sequence: &mut u64,
    message: FileMessage,
) -> Result<(), Box<dyn Error>> {
    *sequence = sequence.checked_add(1).ok_or("file sequence exhausted")?;
    Frame {
        kind: Kind::File,
        epoch: 0,
        sequence: *sequence,
        payload: message.encode(),
    }
    .write_to(channel)?;
    Ok(())
}

fn receive_frame(
    channel: &mut SecureChannel<TcpStream>,
    sequence: &mut u64,
) -> Result<FileMessage, Box<dyn Error>> {
    let frame = Frame::read_from(channel)?;
    let expected = sequence.checked_add(1).ok_or("file sequence exhausted")?;
    if frame.kind != Kind::File || frame.epoch != 0 || frame.sequence != expected {
        return Err("unexpected file frame".into());
    }
    *sequence = expected;
    Ok(FileMessage::decode(&frame.payload)?)
}

fn exchange_greeting(
    channel: &mut SecureChannel<TcpStream>,
    role: Role,
) -> Result<(), Box<dyn Error>> {
    let greeting = Frame {
        kind: Kind::Hello,
        epoch: 0,
        sequence: 0,
        payload: GREETING.to_vec(),
    };
    match role {
        Role::Initiator => {
            greeting.write_to(channel)?;
            if Frame::read_from(channel)? != greeting {
                return Err("incompatible file peer".into());
            }
        }
        Role::Responder => {
            if Frame::read_from(channel)? != greeting {
                return Err("incompatible file peer".into());
            }
            greeting.write_to(channel)?;
        }
    }
    Ok(())
}

fn pinned_key(peers: &Path, ip: IpAddr) -> Result<[u8; 32], Box<dyn Error>> {
    load_peer_key(peers, ip)?.ok_or_else(|| "pair with this IP before transferring files".into())
}

pub fn send_once(
    address: SocketAddr,
    source: &Path,
    identity: &Identity,
    peers: &Path,
    limit: u64,
) -> Result<(), Box<dyn Error>> {
    send_once_with_progress(address, source, identity, peers, limit, |_| {})
}

pub fn send_once_with_progress(
    address: SocketAddr,
    source: &Path,
    identity: &Identity,
    peers: &Path,
    limit: u64,
    mut progress: impl FnMut(u8),
) -> Result<(), Box<dyn Error>> {
    let pinned = pinned_key(peers, address.ip())?;
    if !PeerPolicy::load(
        peers.parent().ok_or("peer directory has no parent")?,
        &pinned,
    )?
    .permits(Capability::Files)
    {
        return Err("file transfers are disabled for this paired computer".into());
    }
    let mut sender = FileSender::open(source, limit)?;
    let total = sender.offer.size;
    let stream = TcpStream::connect_timeout(&address, Duration::from_secs(10))?;
    stream.set_read_timeout(Some(FILE_TIMEOUT))?;
    stream.set_write_timeout(Some(FILE_TIMEOUT))?;
    stream.set_nodelay(true)?;
    let (mut channel, _) =
        SecureChannel::connect(stream, Role::Initiator, identity, Some(&pinned), |_| false)?;
    exchange_greeting(&mut channel, Role::Initiator)?;
    let mut sent = 0;
    let mut received = 0;
    send_frame(
        &mut channel,
        &mut sent,
        FileMessage::Offer(sender.offer.clone()),
    )?;
    match receive_frame(&mut channel, &mut received)? {
        FileMessage::Accept => {}
        FileMessage::Reject => return Err("destination rejected the file".into()),
        _ => return Err("destination did not decide on the file".into()),
    }
    progress(0);
    let mut transferred = 0u64;
    let mut last_percent = 0u8;
    loop {
        match sender.next_chunk() {
            Ok(Some(chunk)) => {
                transferred += chunk.len() as u64;
                send_frame(&mut channel, &mut sent, FileMessage::Chunk(chunk))?;
                let percent = ((transferred as u128 * 100) / total.max(1) as u128) as u8;
                if percent >= last_percent.saturating_add(5) && percent < 100 {
                    progress(percent);
                    last_percent = percent;
                }
            }
            Ok(None) => break,
            Err(error) => {
                let _ = send_frame(&mut channel, &mut sent, FileMessage::Cancel);
                return Err(error.into());
            }
        }
    }
    send_frame(&mut channel, &mut sent, FileMessage::End)?;
    if receive_frame(&mut channel, &mut received)? != FileMessage::Complete {
        return Err("destination did not confirm the saved file".into());
    }
    progress(100);
    Ok(())
}

pub fn receive_once(
    address: SocketAddr,
    directory: &Path,
    identity: &Identity,
    peers: &Path,
    limit: u64,
    approve: impl FnMut(&FileOffer, IpAddr) -> io::Result<bool>,
    ready: impl FnOnce(SocketAddr) -> io::Result<()>,
) -> Result<Option<std::path::PathBuf>, Box<dyn Error>> {
    receive_once_with_progress(
        address,
        directory,
        identity,
        peers,
        limit,
        approve,
        ready,
        |_| {},
    )
}

// The three callbacks separate approval, listener readiness, and UI progress.
#[allow(clippy::too_many_arguments)]
pub fn receive_once_with_progress(
    address: SocketAddr,
    directory: &Path,
    identity: &Identity,
    peers: &Path,
    limit: u64,
    mut approve: impl FnMut(&FileOffer, IpAddr) -> io::Result<bool>,
    ready: impl FnOnce(SocketAddr) -> io::Result<()>,
    mut progress: impl FnMut(u8),
) -> Result<Option<std::path::PathBuf>, Box<dyn Error>> {
    if !directory.is_dir() {
        return Err("destination directory does not exist".into());
    }
    let listener = TcpListener::bind(address)?;
    ready(listener.local_addr()?)?;
    receive_with_listener_progress(
        listener,
        directory,
        identity,
        peers,
        limit,
        &mut approve,
        &mut progress,
    )
}

#[cfg(test)]
fn receive_with_listener(
    listener: TcpListener,
    directory: &Path,
    identity: &Identity,
    peers: &Path,
    limit: u64,
    approve: &mut impl FnMut(&FileOffer, IpAddr) -> io::Result<bool>,
) -> Result<Option<std::path::PathBuf>, Box<dyn Error>> {
    receive_with_listener_progress(
        listener,
        directory,
        identity,
        peers,
        limit,
        approve,
        &mut |_| {},
    )
}

fn receive_with_listener_progress(
    listener: TcpListener,
    directory: &Path,
    identity: &Identity,
    peers: &Path,
    limit: u64,
    approve: &mut impl FnMut(&FileOffer, IpAddr) -> io::Result<bool>,
    progress: &mut impl FnMut(u8),
) -> Result<Option<std::path::PathBuf>, Box<dyn Error>> {
    let (mut channel, peer_address, files_allowed) = loop {
        let (stream, peer_address) = listener.accept()?;
        let Some(pinned) = load_peer_key(peers, peer_address.ip())? else {
            eprintln!(
                "SeamlessControl: se ignoró una conexión de archivo no emparejada desde {peer_address}"
            );
            continue;
        };
        stream.set_read_timeout(Some(HANDSHAKE_TIMEOUT))?;
        stream.set_write_timeout(Some(HANDSHAKE_TIMEOUT))?;
        stream.set_nodelay(true)?;
        let Ok((mut channel, _)) =
            SecureChannel::connect(stream, Role::Responder, identity, Some(&pinned), |_| false)
        else {
            eprintln!("SeamlessControl: falló la autenticación del archivo desde {peer_address}");
            continue;
        };
        if exchange_greeting(&mut channel, Role::Responder).is_err() {
            eprintln!("SeamlessControl: protocolo de archivo incompatible desde {peer_address}");
            continue;
        }
        channel.stream_mut().set_read_timeout(Some(FILE_TIMEOUT))?;
        channel.stream_mut().set_write_timeout(Some(FILE_TIMEOUT))?;
        let files_allowed = PeerPolicy::load(
            peers.parent().ok_or("peer directory has no parent")?,
            &pinned,
        )?
        .permits(Capability::Files);
        break (channel, peer_address, files_allowed);
    };
    let mut sent = 0;
    let mut received = 0;
    let FileMessage::Offer(offer) = receive_frame(&mut channel, &mut received)? else {
        return Err("expected a file offer".into());
    };
    offer.validate(limit)?;
    let group_count = file_bundle::is_bundle_name(&offer.name);
    if offer.name.ends_with(".scbundle") && group_count.is_none() {
        return Err("invalid bundle offer name".into());
    }
    if !files_allowed {
        send_frame(&mut channel, &mut sent, FileMessage::Reject)?;
        return Ok(None);
    }
    if !approve(&offer, peer_address.ip())? {
        send_frame(&mut channel, &mut sent, FileMessage::Reject)?;
        return Ok(None);
    }
    let total = offer.size;
    let stage = group_count.map(|_| bundle_stage(directory)).transpose()?;
    let receive_directory = stage.as_ref().map_or(directory, |stage| stage.0.as_path());
    let mut receiver = FileReceiver::accept(offer, receive_directory, limit)?;
    send_frame(&mut channel, &mut sent, FileMessage::Accept)?;
    progress(0);
    let mut transferred = 0u64;
    let mut last_percent = 0u8;
    loop {
        match receive_frame(&mut channel, &mut received)? {
            FileMessage::Chunk(data) => {
                receiver.write_chunk(&data)?;
                transferred += data.len() as u64;
                let percent = ((transferred as u128 * 100) / total.max(1) as u128) as u8;
                if percent >= last_percent.saturating_add(5) && percent < 100 {
                    progress(percent);
                    last_percent = percent;
                }
            }
            FileMessage::End => {
                let saved = receiver.finish()?;
                let saved = if let (Some(count), Some(stage)) = (group_count, stage.as_ref()) {
                    publish_bundle(&saved, directory, stage, count, limit)?
                } else {
                    saved
                };
                send_frame(&mut channel, &mut sent, FileMessage::Complete)?;
                progress(100);
                return Ok(Some(saved));
            }
            FileMessage::Cancel => return Ok(None),
            _ => return Err("unexpected message during file transfer".into()),
        }
    }
}

pub fn terminal_approval(offer: &FileOffer, peer: IpAddr) -> io::Result<bool> {
    eprintln!(
        "Archivo de {peer}: {} ({} bytes, SHA-256 {:02x?})",
        offer.name, offer.size, offer.sha256
    );
    eprint!("¿Aceptar este archivo? Escriba SI: ");
    io::stderr().flush()?;
    let mut answer = String::new();
    io::stdin().lock().read_line(&mut answer)?;
    Ok(answer.trim() == "SI")
}

pub fn panel_approval(offer: &FileOffer, peer: IpAddr) -> io::Result<bool> {
    panel_approval_with_io(
        offer,
        peer,
        &mut io::stdin().lock(),
        &mut io::stdout().lock(),
    )
}

fn panel_approval_with_io(
    offer: &FileOffer,
    peer: IpAddr,
    input: &mut impl BufRead,
    output: &mut impl Write,
) -> io::Result<bool> {
    let hash: String = offer
        .sha256
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    writeln!(
        output,
        "OFFER\t{peer}\t{}\t{}\t{hash}",
        offer.name, offer.size
    )?;
    output.flush()?;
    let mut answer = String::new();
    input.read_line(&mut answer)?;
    Ok(answer.trim() == "SI")
}

pub fn configured_limit() -> Result<u64, Box<dyn Error>> {
    match std::env::var("SEAMLESSCONTROL_MAX_FILE_BYTES") {
        Ok(value) => {
            let limit: u64 = value.parse()?;
            if limit == 0 {
                return Err("SEAMLESSCONTROL_MAX_FILE_BYTES must be positive".into());
            }
            Ok(limit)
        }
        Err(std::env::VarError::NotPresent) => Ok(DEFAULT_MAX_FILE_BYTES),
        Err(error) => Err(error.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::remember_peer_key;
    use std::fs;
    use std::thread;

    #[test]
    fn panel_offer_is_machine_readable_and_acceptance_is_explicit() {
        let offer = FileOffer {
            name: "informe.txt".to_owned(),
            size: 8,
            sha256: [0xab; 32],
        };
        let peer: IpAddr = "192.168.50.10".parse().unwrap();
        let mut output = Vec::new();
        assert!(panel_approval_with_io(&offer, peer, &mut "SI\n".as_bytes(), &mut output).unwrap());
        let line = String::from_utf8(output).unwrap();
        let fields: Vec<_> = line.trim_end().split('\t').collect();
        assert_eq!(
            fields,
            [
                "OFFER",
                "192.168.50.10",
                "informe.txt",
                "8",
                &"ab".repeat(32)
            ]
        );
        assert!(
            !panel_approval_with_io(&offer, peer, &mut "NO\n".as_bytes(), &mut Vec::new()).unwrap()
        );
        assert!(
            !panel_approval_with_io(&offer, peer, &mut "".as_bytes(), &mut Vec::new()).unwrap()
        );
    }

    #[test]
    fn paired_transfer_requires_consent_and_confirms_saved_bytes() {
        let dir = std::env::temp_dir().join(format!(
            "seamlesscontrol-file-session-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        let sender_peers = dir.join("sender-peers");
        let receiver_peers = dir.join("receiver-peers");
        let downloads = dir.join("downloads");
        fs::create_dir_all(&downloads).unwrap();
        let source = dir.join("example.txt");
        fs::write(&source, "contenido cifrado 🙂").unwrap();
        let sender_id = Identity::generate().unwrap();
        let receiver_id = Identity::generate().unwrap();
        let ip: IpAddr = "127.0.0.1".parse().unwrap();
        remember_peer_key(&sender_peers, ip, &receiver_id.public).unwrap();
        remember_peer_key(&receiver_peers, ip, &sender_id.public).unwrap();
        for accepted in [false, true] {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let destination = downloads.clone();
            let peers = receiver_peers.clone();
            let identity = receiver_id.clone();
            let worker = thread::spawn(move || {
                let mut progress = Vec::new();
                let result = receive_with_listener_progress(
                    listener,
                    &destination,
                    &identity,
                    &peers,
                    DEFAULT_MAX_FILE_BYTES,
                    &mut |offer, peer| {
                        assert_eq!(offer.name, "example.txt");
                        assert_eq!(peer, ip);
                        Ok(accepted)
                    },
                    &mut |percent| progress.push(percent),
                );
                (result.map_err(|error| error.to_string()), progress)
            });
            let mut send_progress = Vec::new();
            let result = send_once_with_progress(
                address,
                &source,
                &sender_id,
                &sender_peers,
                DEFAULT_MAX_FILE_BYTES,
                |percent| send_progress.push(percent),
            );
            if accepted {
                result.unwrap();
                let (received, receive_progress) = worker.join().unwrap();
                let saved = received.unwrap().unwrap();
                assert_eq!(fs::read(saved).unwrap(), fs::read(&source).unwrap());
                assert_eq!(send_progress, [0, 100]);
                assert_eq!(receive_progress, [0, 100]);
            } else {
                assert!(result.is_err());
                let (received, receive_progress) = worker.join().unwrap();
                assert!(received.unwrap().is_none());
                assert!(send_progress.is_empty());
                assert!(receive_progress.is_empty());
                assert!(!downloads.join("example.txt").exists());
            }
        }
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn unauthenticated_connection_does_not_consume_file_receiver() {
        let dir = std::env::temp_dir().join(format!(
            "seamlesscontrol-file-unauthenticated-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        let downloads = dir.join("downloads");
        fs::create_dir_all(&downloads).unwrap();
        let source = dir.join("example.txt");
        fs::write(&source, b"authenticated contents").unwrap();
        let sender_id = Identity::generate().unwrap();
        let impostor_id = Identity::generate().unwrap();
        let receiver_id = Identity::generate().unwrap();
        let ip: IpAddr = "127.0.0.1".parse().unwrap();
        let sender_peers = dir.join("sender-peers");
        let receiver_peers = dir.join("receiver-peers");
        let impostor_peers = dir.join("impostor-peers");
        remember_peer_key(&sender_peers, ip, &receiver_id.public).unwrap();
        remember_peer_key(&impostor_peers, ip, &receiver_id.public).unwrap();
        remember_peer_key(&receiver_peers, ip, &sender_id.public).unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let worker = thread::spawn(move || {
            receive_with_listener(
                listener,
                &downloads,
                &receiver_id,
                &receiver_peers,
                DEFAULT_MAX_FILE_BYTES,
                &mut |_, _| Ok(true),
            )
            .map_err(|error| error.to_string())
        });
        drop(TcpStream::connect(address).unwrap());
        assert!(
            send_once(
                address,
                &source,
                &impostor_id,
                &impostor_peers,
                DEFAULT_MAX_FILE_BYTES,
            )
            .is_err()
        );
        send_once(
            address,
            &source,
            &sender_id,
            &sender_peers,
            DEFAULT_MAX_FILE_BYTES,
        )
        .unwrap();
        let saved = worker.join().unwrap().unwrap().unwrap();
        assert_eq!(fs::read(saved).unwrap(), fs::read(&source).unwrap());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn growing_source_cancels_encrypted_transfer_without_publishing() {
        let dir = std::env::temp_dir().join(format!(
            "seamlesscontrol-growing-session-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        let downloads = dir.join("downloads");
        fs::create_dir_all(&downloads).unwrap();
        let source = dir.join("growing.txt");
        fs::write(&source, b"initial").unwrap();
        let sender_id = Identity::generate().unwrap();
        let receiver_id = Identity::generate().unwrap();
        let ip: IpAddr = "127.0.0.1".parse().unwrap();
        let sender_peers = dir.join("sender-peers");
        let receiver_peers = dir.join("receiver-peers");
        remember_peer_key(&sender_peers, ip, &receiver_id.public).unwrap();
        remember_peer_key(&receiver_peers, ip, &sender_id.public).unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let worker = thread::spawn({
            let source = source.clone();
            let downloads = downloads.clone();
            move || {
                receive_with_listener(
                    listener,
                    &downloads,
                    &receiver_id,
                    &receiver_peers,
                    DEFAULT_MAX_FILE_BYTES,
                    &mut |_offer, _peer| {
                        let mut file = fs::OpenOptions::new().append(true).open(&source)?;
                        file.write_all(b" new data")?;
                        Ok(true)
                    },
                )
                .map_err(|error| error.to_string())
            }
        });
        assert!(
            send_once(
                address,
                &source,
                &sender_id,
                &sender_peers,
                DEFAULT_MAX_FILE_BYTES,
            )
            .is_err()
        );
        assert!(worker.join().unwrap().unwrap().is_none());
        assert!(!downloads.join("growing.txt").exists());
        assert_eq!(fs::read_dir(&downloads).unwrap().count(), 0);
        fs::remove_dir_all(dir).unwrap();
    }
}
