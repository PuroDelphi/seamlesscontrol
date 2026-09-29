//! Small text-only clipboard messages shared by both platform adapters.

use std::io::{self, Read, Write};

pub const MAX_TEXT_BYTES: usize = 256 * 1024;
const PACKET_MAGIC: &[u8; 4] = b"SCB1";
const PACKET_HEADER: usize = 4 + 8 + 32;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClipboardEvent {
    Ignore,
    Text(Vec<u8>),
    Clear,
}

impl ClipboardEvent {
    pub fn encode(&self) -> Vec<u8> {
        match self {
            Self::Ignore => vec![0],
            Self::Text(bytes) => {
                let mut out = Vec::with_capacity(bytes.len() + 1);
                out.push(1);
                out.extend_from_slice(bytes);
                out
            }
            Self::Clear => vec![2],
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        match payload {
            [0] => Ok(Self::Ignore),
            [1, bytes @ ..] if bytes.len() <= MAX_TEXT_BYTES => {
                std::str::from_utf8(bytes).map_err(|_| {
                    io::Error::new(io::ErrorKind::InvalidData, "clipboard is not UTF-8")
                })?;
                Ok(Self::Text(bytes.to_vec()))
            }
            [2] => Ok(Self::Clear),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid clipboard message",
            )),
        }
    }

    pub fn write_framed(&self, writer: &mut impl Write) -> io::Result<()> {
        let payload = self.encode();
        if payload.len() > MAX_TEXT_BYTES + 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "clipboard too large",
            ));
        }
        writer.write_all(&(payload.len() as u32).to_be_bytes())?;
        writer.write_all(&payload)
    }

    pub fn read_framed(reader: &mut impl Read) -> io::Result<Option<Self>> {
        let mut size = [0; 4];
        if reader.read(&mut size[..1])? == 0 {
            return Ok(None);
        }
        reader.read_exact(&mut size[1..])?;
        let size = u32::from_be_bytes(size) as usize;
        if !(1..=MAX_TEXT_BYTES + 1).contains(&size) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid clipboard length",
            ));
        }
        let mut payload = vec![0; size];
        reader.read_exact(&mut payload)?;
        Self::decode(&payload).map(Some)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClipboardPacket {
    pub revision: u64,
    pub origin: [u8; 32],
    pub event: ClipboardEvent,
}

impl ClipboardPacket {
    pub fn encode(&self) -> Vec<u8> {
        let event = self.event.encode();
        let mut out = Vec::with_capacity(PACKET_HEADER + event.len());
        out.extend_from_slice(PACKET_MAGIC);
        out.extend_from_slice(&self.revision.to_be_bytes());
        out.extend_from_slice(&self.origin);
        out.extend_from_slice(&event);
        out
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        if payload.len() < PACKET_HEADER + 1
            || payload.len() > PACKET_HEADER + MAX_TEXT_BYTES + 1
            || &payload[..4] != PACKET_MAGIC
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid clipboard packet",
            ));
        }
        let revision = u64::from_be_bytes(payload[4..12].try_into().expect("checked length"));
        if revision == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "zero clipboard revision",
            ));
        }
        let origin = payload[12..44].try_into().expect("checked length");
        let event = ClipboardEvent::decode(&payload[44..])?;
        if matches!(event, ClipboardEvent::Ignore) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "ignored event on network",
            ));
        }
        Ok(Self {
            revision,
            origin,
            event,
        })
    }
}

pub struct ClipboardSync {
    local_id: [u8; 32],
    remote_id: [u8; 32],
    clock: u64,
    winner: Option<(u64, [u8; 32])>,
    last_local: Option<ClipboardEvent>,
}

impl ClipboardSync {
    pub fn new(local_id: [u8; 32], remote_id: [u8; 32]) -> Self {
        Self {
            local_id,
            remote_id,
            clock: 0,
            winner: None,
            last_local: None,
        }
    }

    pub fn local_changed(&mut self, event: &ClipboardEvent) -> Option<ClipboardPacket> {
        if matches!(event, ClipboardEvent::Ignore) || self.last_local.as_ref() == Some(event) {
            return None;
        }
        self.clock = self.clock.checked_add(1)?;
        self.last_local = Some(event.clone());
        self.winner = Some((self.clock, self.local_id));
        Some(ClipboardPacket {
            revision: self.clock,
            origin: self.local_id,
            event: event.clone(),
        })
    }

    pub fn remote_needs_apply(&mut self, packet: &ClipboardPacket) -> io::Result<bool> {
        if packet.origin != self.remote_id {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "clipboard origin does not match peer",
            ));
        }
        self.clock = self.clock.max(packet.revision);
        let incoming = (packet.revision, packet.origin);
        if self.winner.is_some_and(|winner| incoming <= winner) {
            return Ok(false);
        }
        if self.last_local.as_ref() == Some(&packet.event) {
            self.winner = Some(incoming);
            return Ok(false);
        }
        Ok(true)
    }

    pub fn remote_applied(&mut self, packet: &ClipboardPacket) {
        self.winner = Some((packet.revision, packet.origin));
        self.last_local = Some(packet.event.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_clear_and_empty_round_trip() {
        for event in [
            ClipboardEvent::Text("á🙂\n".as_bytes().to_vec()),
            ClipboardEvent::Text(Vec::new()),
            ClipboardEvent::Clear,
        ] {
            let mut wire = Vec::new();
            event.write_framed(&mut wire).unwrap();
            assert_eq!(
                ClipboardEvent::read_framed(&mut wire.as_slice()).unwrap(),
                Some(event)
            );
        }
        assert!(ClipboardEvent::decode(&[1, 0xff]).is_err());
        assert!(ClipboardEvent::decode(&[2, 0]).is_err());
        let oversized = ((MAX_TEXT_BYTES + 2) as u32).to_be_bytes();
        assert!(ClipboardEvent::read_framed(&mut oversized.as_slice()).is_err());
    }

    #[test]
    fn applying_remote_text_does_not_echo_to_peer() {
        let mut state = ClipboardSync::new([1; 32], [2; 32]);
        let first = ClipboardEvent::Text(b"origin".to_vec());
        let second = ClipboardEvent::Text(b"destination".to_vec());
        assert!(state.local_changed(&first).is_some());
        assert!(state.local_changed(&first).is_none());
        let packet = ClipboardPacket {
            revision: 1,
            origin: [2; 32],
            event: second.clone(),
        };
        assert_eq!(ClipboardPacket::decode(&packet.encode()).unwrap(), packet);
        assert!(state.remote_needs_apply(&packet).unwrap());
        state.remote_applied(&packet);
        assert!(state.local_changed(&second).is_none());
        assert!(state.local_changed(&ClipboardEvent::Clear).is_some());
        assert!(state.local_changed(&ClipboardEvent::Ignore).is_none());
        assert!(ClipboardPacket::decode(&[0; 44]).is_err());
    }

    #[test]
    fn concurrent_copies_choose_the_same_winner() {
        let mut a = ClipboardSync::new([1; 32], [2; 32]);
        let mut b = ClipboardSync::new([2; 32], [1; 32]);
        let from_a = a
            .local_changed(&ClipboardEvent::Text(b"A".to_vec()))
            .unwrap();
        let from_b = b
            .local_changed(&ClipboardEvent::Text(b"B".to_vec()))
            .unwrap();
        assert!(a.remote_needs_apply(&from_b).unwrap());
        a.remote_applied(&from_b);
        assert!(!b.remote_needs_apply(&from_a).unwrap());
        assert!(a.local_changed(&from_b.event).is_none());
        assert!(b.local_changed(&from_b.event).is_none());
        assert!(a.remote_needs_apply(&from_a).is_err());
    }
}
