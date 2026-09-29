//! Mutually authenticated Noise XX transport. On first pairing, both humans
//! must compare the same six-digit SAS before the channel is usable. Later
//! sessions require the previously pinned static public key.

use snow::{Builder, HandshakeState, TransportState};
use std::fmt;
use std::io::{self, Read, Write};

const PATTERN: &str = "Noise_XX_25519_ChaChaPoly_BLAKE2s";
const RECORD_PLAINTEXT: usize = 16 * 1024;
const RECORD_CIPHERTEXT: usize = RECORD_PLAINTEXT + 16;
const MAX_HANDSHAKE: usize = 4096;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Role {
    Initiator,
    Responder,
}

#[derive(Clone, Eq, PartialEq)]
pub struct Identity {
    pub private: [u8; 32],
    pub public: [u8; 32],
}

impl fmt::Debug for Identity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Identity")
            .field("private", &"[redacted]")
            .field("public", &self.public)
            .finish()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PeerInfo {
    pub public_key: [u8; 32],
    pub sas: String,
}

#[derive(Debug)]
pub enum SecureError {
    Io(io::Error),
    Noise(snow::Error),
    InvalidKey,
    InvalidRecord,
    PeerKeyChanged,
    PairingRejected,
    PeerRejected,
}

impl fmt::Display for SecureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::Noise(e) => write!(f, "Noise protocol error: {e}"),
            Self::InvalidKey => write!(f, "invalid peer key"),
            Self::InvalidRecord => write!(f, "invalid encrypted record"),
            Self::PeerKeyChanged => write!(f, "paired peer changed its identity key"),
            Self::PairingRejected => write!(f, "local pairing confirmation rejected"),
            Self::PeerRejected => write!(f, "remote pairing confirmation rejected"),
        }
    }
}

impl std::error::Error for SecureError {}
impl From<io::Error> for SecureError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}
impl From<snow::Error> for SecureError {
    fn from(e: snow::Error) -> Self {
        Self::Noise(e)
    }
}

impl Identity {
    pub fn generate() -> Result<Self, SecureError> {
        let params = PATTERN.parse().expect("constant Noise pattern");
        let keys = Builder::new(params).generate_keypair()?;
        let private = keys
            .private
            .try_into()
            .map_err(|_| SecureError::InvalidKey)?;
        let public = keys
            .public
            .try_into()
            .map_err(|_| SecureError::InvalidKey)?;
        Ok(Self { private, public })
    }
}

pub struct SecureChannel<S> {
    stream: S,
    cipher: TransportState,
    read_buf: Vec<u8>,
    read_at: usize,
}

impl<S: Read + Write> SecureChannel<S> {
    /// `confirm` must require local human approval of the SAS on first pair.
    /// A known peer is accepted only if its static key matches `pinned_peer`.
    pub fn connect(
        mut stream: S,
        role: Role,
        identity: &Identity,
        pinned_peer: Option<&[u8; 32]>,
        mut confirm: impl FnMut(&PeerInfo) -> bool,
    ) -> Result<(Self, PeerInfo), SecureError> {
        let params = PATTERN.parse().expect("constant Noise pattern");
        let builder = Builder::new(params).local_private_key(&identity.private)?;
        let mut state = match role {
            Role::Initiator => builder.build_initiator()?,
            Role::Responder => builder.build_responder()?,
        };

        match role {
            Role::Initiator => {
                send_handshake(&mut stream, &mut state)?;
                receive_handshake(&mut stream, &mut state)?;
                send_handshake(&mut stream, &mut state)?;
            }
            Role::Responder => {
                receive_handshake(&mut stream, &mut state)?;
                send_handshake(&mut stream, &mut state)?;
                receive_handshake(&mut stream, &mut state)?;
            }
        }

        let remote = state.get_remote_static().ok_or(SecureError::InvalidKey)?;
        let public_key: [u8; 32] = remote.try_into().map_err(|_| SecureError::InvalidKey)?;
        let hash = state.get_handshake_hash();
        let short = u32::from_be_bytes(hash[0..4].try_into().expect("Noise hash")) % 1_000_000;
        let peer = PeerInfo {
            public_key,
            sas: format!("{short:06}"),
        };
        let local_ok = if let Some(pinned) = pinned_peer {
            if pinned != &public_key {
                return Err(SecureError::PeerKeyChanged);
            }
            true
        } else {
            confirm(&peer)
        };
        let mut channel = Self {
            stream,
            cipher: state.into_transport_mode()?,
            read_buf: Vec::new(),
            read_at: 0,
        };

        // Both sides exchange an authenticated acceptance byte before the
        // caller can send input. Responder sends first to avoid a deadlock.
        match role {
            Role::Initiator => {
                channel.send_record(&[u8::from(local_ok)])?;
                let accepted = channel.read_record()?.as_deref() == Some(&[1][..]);
                if !local_ok {
                    return Err(SecureError::PairingRejected);
                }
                if !accepted {
                    return Err(SecureError::PeerRejected);
                }
            }
            Role::Responder => {
                let accepted = channel.read_record()?.as_deref() == Some(&[1][..]);
                channel.send_record(&[u8::from(local_ok)])?;
                if !local_ok {
                    return Err(SecureError::PairingRejected);
                }
                if !accepted {
                    return Err(SecureError::PeerRejected);
                }
            }
        }
        Ok((channel, peer))
    }

    fn send_record(&mut self, plain: &[u8]) -> Result<(), SecureError> {
        if plain.is_empty() || plain.len() > RECORD_PLAINTEXT {
            return Err(SecureError::InvalidRecord);
        }
        let mut ciphertext = vec![0; plain.len() + 16];
        let size = self.cipher.write_message(plain, &mut ciphertext)?;
        let size: u16 = size.try_into().map_err(|_| SecureError::InvalidRecord)?;
        self.stream.write_all(&size.to_be_bytes())?;
        self.stream.write_all(&ciphertext[..usize::from(size)])?;
        self.stream.flush()?;
        Ok(())
    }

    fn read_record(&mut self) -> Result<Option<Vec<u8>>, SecureError> {
        let mut length = [0; 2];
        if self.stream.read(&mut length[..1])? == 0 {
            return Ok(None);
        }
        self.stream.read_exact(&mut length[1..])?;
        let length = usize::from(u16::from_be_bytes(length));
        if !(16..=RECORD_CIPHERTEXT).contains(&length) {
            return Err(SecureError::InvalidRecord);
        }
        let mut ciphertext = vec![0; length];
        self.stream.read_exact(&mut ciphertext)?;
        let mut plain = vec![0; length];
        let size = self.cipher.read_message(&ciphertext, &mut plain)?;
        plain.truncate(size);
        Ok(Some(plain))
    }

    pub fn stream_mut(&mut self) -> &mut S {
        &mut self.stream
    }
}

impl<S: Read + Write> Read for SecureChannel<S> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        if self.read_at >= self.read_buf.len() {
            self.read_buf = match self.read_record() {
                Ok(Some(record)) => record,
                Ok(None) => return Ok(0),
                Err(SecureError::Io(error)) => return Err(error),
                Err(error) => return Err(io::Error::other(error)),
            };
            self.read_at = 0;
            if self.read_buf.is_empty() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "empty encrypted record",
                ));
            }
        }
        let count = buf.len().min(self.read_buf.len() - self.read_at);
        buf[..count].copy_from_slice(&self.read_buf[self.read_at..self.read_at + count]);
        self.read_at += count;
        Ok(count)
    }
}

impl<S: Read + Write> Write for SecureChannel<S> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        let count = buf.len().min(RECORD_PLAINTEXT);
        self.send_record(&buf[..count]).map_err(io::Error::other)?;
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.stream.flush()
    }
}

fn send_handshake(stream: &mut impl Write, state: &mut HandshakeState) -> Result<(), SecureError> {
    let mut message = [0; MAX_HANDSHAKE];
    let size = state.write_message(&[], &mut message)?;
    let size: u16 = size.try_into().map_err(|_| SecureError::InvalidRecord)?;
    stream.write_all(&size.to_be_bytes())?;
    stream.write_all(&message[..usize::from(size)])?;
    stream.flush()?;
    Ok(())
}

fn receive_handshake(
    stream: &mut impl Read,
    state: &mut HandshakeState,
) -> Result<(), SecureError> {
    let mut size = [0; 2];
    stream.read_exact(&mut size)?;
    let size = usize::from(u16::from_be_bytes(size));
    if size == 0 || size > MAX_HANDSHAKE {
        return Err(SecureError::InvalidRecord);
    }
    let mut message = vec![0; size];
    stream.read_exact(&mut message)?;
    state.read_message(&message, &mut [0; MAX_HANDSHAKE])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{TcpListener, TcpStream};
    use std::thread;

    #[test]
    fn paired_transport_round_trip_and_pin() {
        let a = Identity::generate().unwrap();
        let b = Identity::generate().unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let handle = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let (mut channel, peer) =
                SecureChannel::connect(stream, Role::Responder, &b, None, |_| true).unwrap();
            let mut payload = [0; 4];
            channel.read_exact(&mut payload).unwrap();
            assert_eq!(&payload, b"ping");
            channel.write_all(b"pong").unwrap();
            peer
        });
        let stream = TcpStream::connect(address).unwrap();
        let (mut channel, peer) =
            SecureChannel::connect(stream, Role::Initiator, &a, None, |_| true).unwrap();
        channel.write_all(b"ping").unwrap();
        let mut response = [0; 4];
        channel.read_exact(&mut response).unwrap();
        assert_eq!(&response, b"pong");
        let other = handle.join().unwrap();
        assert_eq!(peer.sas, other.sas);
        assert_eq!(other.public_key, a.public);
    }

    #[test]
    fn changed_peer_key_is_rejected() {
        let a = Identity::generate().unwrap();
        let b = Identity::generate().unwrap();
        let wrong = Identity::generate().unwrap().public;
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let handle = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let _ = SecureChannel::connect(stream, Role::Responder, &b, None, |_| true);
        });
        let stream = TcpStream::connect(address).unwrap();
        assert!(matches!(
            SecureChannel::connect(stream, Role::Initiator, &a, Some(&wrong), |_| panic!(
                "must not ask to approve a changed key"
            )),
            Err(SecureError::PeerKeyChanged)
        ));
        handle.join().unwrap();
    }
}
