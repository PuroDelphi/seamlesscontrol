//! Versioned framing for authenticated transport. Encryption belongs to the
//! transport adapter; untrusted frames are still bounded and validated here.

use std::fmt;
use std::io::{self, Read, Write};

pub const PROTOCOL_VERSION: u16 = 1;
pub const MAX_PAYLOAD: usize = 1024 * 1024;
const HEADER_LEN: usize = 20;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Kind {
    Hello = 1,
    Control = 2,
    Input = 3,
    Clipboard = 4,
    File = 5,
    Heartbeat = 6,
}

impl TryFrom<u8> for Kind {
    type Error = FrameError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Hello),
            2 => Ok(Self::Control),
            3 => Ok(Self::Input),
            4 => Ok(Self::Clipboard),
            5 => Ok(Self::File),
            6 => Ok(Self::Heartbeat),
            other => Err(FrameError::UnknownKind(other)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Frame {
    pub kind: Kind,
    pub epoch: u64,
    pub sequence: u64,
    pub payload: Vec<u8>,
}

#[derive(Debug)]
pub enum FrameError {
    Io(io::Error),
    Oversized(usize),
    Truncated(usize),
    UnsupportedVersion(u16),
    UnknownKind(u8),
    NonzeroReserved(u8),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::Oversized(n) => write!(f, "frame payload too large: {n} bytes"),
            Self::Truncated(n) => write!(f, "frame header too short: {n} bytes"),
            Self::UnsupportedVersion(v) => write!(f, "unsupported protocol version: {v}"),
            Self::UnknownKind(k) => write!(f, "unknown frame kind: {k}"),
            Self::NonzeroReserved(v) => write!(f, "reserved byte must be zero: {v}"),
        }
    }
}

impl std::error::Error for FrameError {}

impl From<io::Error> for FrameError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl Frame {
    pub fn encode(&self) -> Result<Vec<u8>, FrameError> {
        let len = self.payload.len();
        if len > MAX_PAYLOAD {
            return Err(FrameError::Oversized(len));
        }
        let mut out = Vec::with_capacity(4 + HEADER_LEN + len);
        out.extend_from_slice(&((HEADER_LEN + len) as u32).to_be_bytes());
        out.extend_from_slice(&PROTOCOL_VERSION.to_be_bytes());
        out.push(self.kind as u8);
        out.push(0);
        out.extend_from_slice(&self.epoch.to_be_bytes());
        out.extend_from_slice(&self.sequence.to_be_bytes());
        out.extend_from_slice(&self.payload);
        Ok(out)
    }

    pub fn decode_body(body: &[u8]) -> Result<Self, FrameError> {
        if body.len() < HEADER_LEN {
            return Err(FrameError::Truncated(body.len()));
        }
        if body.len() - HEADER_LEN > MAX_PAYLOAD {
            return Err(FrameError::Oversized(body.len() - HEADER_LEN));
        }
        let version = u16::from_be_bytes([body[0], body[1]]);
        if version != PROTOCOL_VERSION {
            return Err(FrameError::UnsupportedVersion(version));
        }
        let kind = Kind::try_from(body[2])?;
        if body[3] != 0 {
            return Err(FrameError::NonzeroReserved(body[3]));
        }
        let epoch = u64::from_be_bytes(body[4..12].try_into().expect("fixed header"));
        let sequence = u64::from_be_bytes(body[12..20].try_into().expect("fixed header"));
        Ok(Self {
            kind,
            epoch,
            sequence,
            payload: body[HEADER_LEN..].to_vec(),
        })
    }

    pub fn read_from(reader: &mut impl Read) -> Result<Self, FrameError> {
        let mut size = [0; 4];
        reader.read_exact(&mut size)?;
        let size = u32::from_be_bytes(size) as usize;
        if size < HEADER_LEN {
            return Err(FrameError::Truncated(size));
        }
        if size - HEADER_LEN > MAX_PAYLOAD {
            return Err(FrameError::Oversized(size - HEADER_LEN));
        }
        let mut body = vec![0; size];
        reader.read_exact(&mut body)?;
        Self::decode_body(&body)
    }

    pub fn write_to(&self, writer: &mut impl Write) -> Result<(), FrameError> {
        writer.write_all(&self.encode()?)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_round_trip() {
        let original = Frame {
            kind: Kind::Input,
            epoch: 42,
            sequence: 7,
            payload: vec![0, 1, 255],
        };
        let mut bytes = original.encode().unwrap().as_slice().to_vec();
        assert_eq!(Frame::read_from(&mut bytes.as_slice()).unwrap(), original);
        bytes[6] = 99;
        assert!(matches!(
            Frame::read_from(&mut bytes.as_slice()),
            Err(FrameError::UnknownKind(99))
        ));
    }

    #[test]
    fn oversized_frame_is_rejected_before_allocation() {
        let declared = (HEADER_LEN + MAX_PAYLOAD + 1) as u32;
        let bytes = declared.to_be_bytes();
        let mut input = bytes.as_slice();
        assert!(matches!(
            Frame::read_from(&mut input),
            Err(FrameError::Oversized(_))
        ));
    }
}
