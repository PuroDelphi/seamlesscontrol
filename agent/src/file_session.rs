//! One-shot encrypted file transfer. Pair the computers first; each received
//! file requires a fresh local confirmation before any bytes are written.

use crate::file_transfer::{
    DEFAULT_MAX_FILE_BYTES, FileMessage, FileOffer, FileReceiver, FileSender,
};
use crate::protocol::{Frame, Kind};
use crate::secure::{Identity, Role, SecureChannel};
use crate::storage::load_peer_key;
use std::error::Error;
use std::io::{self, BufRead, Write};
use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream};
use std::path::Path;
use std::time::Duration;

const GREETING: &[u8] = b"seamlesscontrol-file/1";
const FILE_TIMEOUT: Duration = Duration::from_secs(300);

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
    let pinned = pinned_key(peers, address.ip())?;
    let mut sender = FileSender::open(source, limit)?;
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
    while let Some(chunk) = sender.next_chunk()? {
        send_frame(&mut channel, &mut sent, FileMessage::Chunk(chunk))?;
    }
    send_frame(&mut channel, &mut sent, FileMessage::End)?;
    if receive_frame(&mut channel, &mut received)? != FileMessage::Complete {
        return Err("destination did not confirm the saved file".into());
    }
    Ok(())
}

pub fn receive_once(
    address: SocketAddr,
    directory: &Path,
    identity: &Identity,
    peers: &Path,
    limit: u64,
    mut approve: impl FnMut(&FileOffer, IpAddr) -> io::Result<bool>,
) -> Result<Option<std::path::PathBuf>, Box<dyn Error>> {
    if !directory.is_dir() {
        return Err("destination directory does not exist".into());
    }
    let listener = TcpListener::bind(address)?;
    receive_with_listener(listener, directory, identity, peers, limit, &mut approve)
}

fn receive_with_listener(
    listener: TcpListener,
    directory: &Path,
    identity: &Identity,
    peers: &Path,
    limit: u64,
    approve: &mut impl FnMut(&FileOffer, IpAddr) -> io::Result<bool>,
) -> Result<Option<std::path::PathBuf>, Box<dyn Error>> {
    let (stream, peer_address) = listener.accept()?;
    let pinned = pinned_key(peers, peer_address.ip())?;
    stream.set_read_timeout(Some(FILE_TIMEOUT))?;
    stream.set_write_timeout(Some(FILE_TIMEOUT))?;
    stream.set_nodelay(true)?;
    let (mut channel, _) =
        SecureChannel::connect(stream, Role::Responder, identity, Some(&pinned), |_| false)?;
    exchange_greeting(&mut channel, Role::Responder)?;
    let mut sent = 0;
    let mut received = 0;
    let FileMessage::Offer(offer) = receive_frame(&mut channel, &mut received)? else {
        return Err("expected a file offer".into());
    };
    offer.validate(limit)?;
    if !approve(&offer, peer_address.ip())? {
        send_frame(&mut channel, &mut sent, FileMessage::Reject)?;
        return Ok(None);
    }
    let mut receiver = FileReceiver::accept(offer, directory, limit)?;
    send_frame(&mut channel, &mut sent, FileMessage::Accept)?;
    loop {
        match receive_frame(&mut channel, &mut received)? {
            FileMessage::Chunk(data) => receiver.write_chunk(&data)?,
            FileMessage::End => {
                let saved = receiver.finish()?;
                send_frame(&mut channel, &mut sent, FileMessage::Complete)?;
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
        let peer: IpAddr = "192.168.1.10".parse().unwrap();
        let mut output = Vec::new();
        assert!(panel_approval_with_io(&offer, peer, &mut "SI\n".as_bytes(), &mut output).unwrap());
        let line = String::from_utf8(output).unwrap();
        let fields: Vec<_> = line.trim_end().split('\t').collect();
        assert_eq!(
            fields,
            [
                "OFFER",
                "192.168.1.10",
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
                receive_with_listener(
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
                )
                .map_err(|error| error.to_string())
            });
            let result = send_once(
                address,
                &source,
                &sender_id,
                &sender_peers,
                DEFAULT_MAX_FILE_BYTES,
            );
            if accepted {
                result.unwrap();
                let saved = worker.join().unwrap().unwrap().unwrap();
                assert_eq!(fs::read(saved).unwrap(), fs::read(&source).unwrap());
            } else {
                assert!(result.is_err());
                assert!(worker.join().unwrap().unwrap().is_none());
                assert!(!downloads.join("example.txt").exists());
            }
        }
        fs::remove_dir_all(dir).unwrap();
    }
}
