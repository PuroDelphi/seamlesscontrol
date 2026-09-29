//! Authenticated input receiver. This module contains no platform calls, so
//! disconnect and malformed-frame behavior can be tested without a desktop.

use crate::protocol::{Frame, FrameError, Kind};
use crate::state::{ApplyResult, EventError, InputEvent, Receiver};
use std::fmt;
use std::io::{self, Read};

pub trait Injector {
    fn inject(&mut self, event: &InputEvent) -> io::Result<()>;

    fn ownership_changed(&mut self, _controlling: bool) {}
}

#[derive(Debug)]
pub enum ReceiverError {
    Frame(FrameError),
    Event(EventError),
    Injection(io::Error),
    UnexpectedFrame,
}

impl fmt::Display for ReceiverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Frame(e) => write!(f, "frame error: {e}"),
            Self::Event(e) => write!(f, "input event error: {e}"),
            Self::Injection(e) => write!(f, "injection error: {e}"),
            Self::UnexpectedFrame => write!(f, "unexpected frame in input receiver"),
        }
    }
}

impl std::error::Error for ReceiverError {}
impl From<FrameError> for ReceiverError {
    fn from(value: FrameError) -> Self {
        Self::Frame(value)
    }
}
impl From<EventError> for ReceiverError {
    fn from(value: EventError) -> Self {
        Self::Event(value)
    }
}

pub struct InputReceiver<I: Injector> {
    injector: I,
    ledger: Receiver,
    controlling: bool,
}

impl<I: Injector> InputReceiver<I> {
    pub fn new(injector: I) -> Self {
        Self {
            injector,
            ledger: Receiver::new(),
            controlling: false,
        }
    }

    pub fn handle(&mut self, frame: &Frame) -> Result<(), ReceiverError> {
        match frame.kind {
            Kind::Control if frame.payload == b"BEGIN" => {
                self.release()?;
                for event in self.ledger.begin(frame.epoch) {
                    self.injector
                        .inject(&event)
                        .map_err(ReceiverError::Injection)?;
                }
                self.controlling = true;
                self.injector.ownership_changed(true);
                Ok(())
            }
            Kind::Control if frame.payload == b"END" => self.release(),
            Kind::Input if self.controlling => {
                let event = InputEvent::decode(&frame.payload)?;
                if self.ledger.apply(frame.epoch, frame.sequence, &event) == ApplyResult::Accepted
                    && let Err(error) = self.injector.inject(&event)
                {
                    let _ = self.release();
                    return Err(ReceiverError::Injection(error));
                }
                Ok(())
            }
            Kind::Heartbeat => Ok(()),
            _ => Err(ReceiverError::UnexpectedFrame),
        }
    }

    /// Always call after EOF, error, timeout, pause, or lock.
    pub fn release(&mut self) -> Result<(), ReceiverError> {
        if std::mem::take(&mut self.controlling) {
            self.injector.ownership_changed(false);
        }
        let mut first_error = None;
        for event in self.ledger.disconnect() {
            if let Err(error) = self.injector.inject(&event)
                && first_error.is_none()
            {
                first_error = Some(error);
            }
        }
        first_error.map_or(Ok(()), |error| Err(ReceiverError::Injection(error)))
    }

    pub fn into_injector(self) -> I {
        self.injector
    }
}

/// Consume a stream of authenticated, decrypted frames. The caller closes
/// the transport after this returns; all held input is released first.
pub fn run_receiver<R: Read, I: Injector>(reader: &mut R, injector: I) -> Result<I, ReceiverError> {
    run_receiver_with_first(reader, injector, None)
}

/// Use when the caller has already read the first frame to switch transport
/// timeouts after a slow, consent-gated connection setup.
pub fn run_receiver_with_first<R: Read, I: Injector>(
    reader: &mut R,
    injector: I,
    first: Option<Frame>,
) -> Result<I, ReceiverError> {
    run_receiver_with_first_until(reader, injector, first, || false)
}

pub fn run_receiver_with_first_until<R: Read, I: Injector>(
    reader: &mut R,
    injector: I,
    first: Option<Frame>,
    mut should_stop: impl FnMut() -> bool,
) -> Result<I, ReceiverError> {
    let mut receiver = InputReceiver::new(injector);
    let mut first = first;
    loop {
        if should_stop() {
            receiver.release()?;
            return Ok(receiver.into_injector());
        }
        match first.take().map_or_else(|| Frame::read_from(reader), Ok) {
            Ok(frame) => {
                if let Err(error) = receiver.handle(&frame) {
                    receiver.release()?;
                    return Err(error);
                }
            }
            Err(FrameError::Io(error)) if error.kind() == io::ErrorKind::UnexpectedEof => {
                receiver.release()?;
                return Ok(receiver.into_injector());
            }
            Err(error) => {
                receiver.release()?;
                return Err(ReceiverError::Frame(error));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secure::{Identity, Role, SecureChannel};
    use std::net::{TcpListener, TcpStream};
    use std::thread;

    #[derive(Default)]
    struct RecordingInjector(Vec<InputEvent>);
    impl Injector for RecordingInjector {
        fn inject(&mut self, event: &InputEvent) -> io::Result<()> {
            self.0.push(event.clone());
            Ok(())
        }
    }

    #[derive(Default)]
    struct OwnershipInjector(Vec<bool>);
    impl Injector for OwnershipInjector {
        fn inject(&mut self, _event: &InputEvent) -> io::Result<()> {
            Ok(())
        }

        fn ownership_changed(&mut self, controlling: bool) {
            self.0.push(controlling);
        }
    }

    #[test]
    fn receiver_reports_remote_ownership_until_end() {
        let mut receiver = InputReceiver::new(OwnershipInjector::default());
        receiver
            .handle(&Frame {
                kind: Kind::Control,
                epoch: 9,
                sequence: 1,
                payload: b"BEGIN".to_vec(),
            })
            .unwrap();
        receiver
            .handle(&Frame {
                kind: Kind::Control,
                epoch: 9,
                sequence: 2,
                payload: b"END".to_vec(),
            })
            .unwrap();
        receiver.release().unwrap();
        assert_eq!(receiver.into_injector().0, vec![true, false]);
    }

    #[test]
    fn eof_releases_pressed_key_and_button() {
        let mut bytes = Vec::new();
        for frame in [
            Frame {
                kind: Kind::Control,
                epoch: 77,
                sequence: 0,
                payload: b"BEGIN".to_vec(),
            },
            Frame {
                kind: Kind::Input,
                epoch: 77,
                sequence: 1,
                payload: InputEvent::KeyDown(42).encode(),
            },
            Frame {
                kind: Kind::Input,
                epoch: 77,
                sequence: 2,
                payload: InputEvent::ButtonDown(272).encode(),
            },
        ] {
            frame.write_to(&mut bytes).unwrap();
        }
        let output = run_receiver(&mut bytes.as_slice(), RecordingInjector::default()).unwrap();
        assert_eq!(
            output.0,
            vec![
                InputEvent::KeyDown(42),
                InputEvent::ButtonDown(272),
                InputEvent::KeyUp(42),
                InputEvent::ButtonUp(272),
            ]
        );
    }

    #[test]
    fn stale_input_is_ignored_and_invalid_frame_releases() {
        let mut receiver = InputReceiver::new(RecordingInjector::default());
        receiver
            .handle(&Frame {
                kind: Kind::Control,
                epoch: 5,
                sequence: 0,
                payload: b"BEGIN".to_vec(),
            })
            .unwrap();
        receiver
            .handle(&Frame {
                kind: Kind::Input,
                epoch: 4,
                sequence: 1,
                payload: InputEvent::KeyDown(30).encode(),
            })
            .unwrap();
        assert!(receiver.injector.0.is_empty());
        receiver
            .handle(&Frame {
                kind: Kind::Input,
                epoch: 5,
                sequence: 2,
                payload: InputEvent::KeyDown(30).encode(),
            })
            .unwrap();
        assert!(matches!(
            receiver.handle(&Frame {
                kind: Kind::File,
                epoch: 5,
                sequence: 3,
                payload: vec![]
            }),
            Err(ReceiverError::UnexpectedFrame)
        ));
        receiver.release().unwrap();
        assert_eq!(
            receiver.injector.0,
            vec![InputEvent::KeyDown(30), InputEvent::KeyUp(30)]
        );
    }

    #[test]
    fn encrypted_loopback_disconnect_releases_remote_key() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let initiator = Identity::generate().unwrap();
        let responder = Identity::generate().unwrap();
        let handle = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let (mut channel, _) =
                SecureChannel::connect(stream, Role::Responder, &responder, None, |_| true)
                    .unwrap();
            run_receiver(&mut channel, RecordingInjector::default())
                .unwrap()
                .0
        });
        let stream = TcpStream::connect(address).unwrap();
        let (mut channel, _) =
            SecureChannel::connect(stream, Role::Initiator, &initiator, None, |_| true).unwrap();
        Frame {
            kind: Kind::Control,
            epoch: 19,
            sequence: 0,
            payload: b"BEGIN".to_vec(),
        }
        .write_to(&mut channel)
        .unwrap();
        Frame {
            kind: Kind::Input,
            epoch: 19,
            sequence: 1,
            payload: InputEvent::KeyDown(42).encode(),
        }
        .write_to(&mut channel)
        .unwrap();
        drop(channel);
        assert_eq!(
            handle.join().unwrap(),
            vec![InputEvent::KeyDown(42), InputEvent::KeyUp(42)]
        );
    }

    #[test]
    fn encrypted_return_feedback_ends_remote_input_epoch() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let initiator = Identity::generate().unwrap();
        let responder = Identity::generate().unwrap();
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let (channel, _) =
                SecureChannel::connect(stream, Role::Responder, &responder, None, |_| true)
                    .unwrap();
            let (mut reader, mut writer) = channel.into_tcp_halves().unwrap();
            let mut receiver = InputReceiver::new(RecordingInjector::default());
            receiver
                .handle(&Frame::read_from(&mut reader).unwrap())
                .unwrap();
            receiver
                .handle(&Frame::read_from(&mut reader).unwrap())
                .unwrap();
            Frame {
                kind: Kind::Control,
                epoch: 0,
                sequence: 1,
                payload: b"RETURN".to_vec(),
            }
            .write_to(&mut writer)
            .unwrap();
            receiver
                .handle(&Frame::read_from(&mut reader).unwrap())
                .unwrap();
            receiver.into_injector().0
        });
        let stream = TcpStream::connect(address).unwrap();
        let (channel, _) =
            SecureChannel::connect(stream, Role::Initiator, &initiator, None, |_| true).unwrap();
        let (mut reader, mut writer) = channel.into_tcp_halves().unwrap();
        for (sequence, kind, payload) in [
            (1, Kind::Control, b"BEGIN".to_vec()),
            (2, Kind::Input, InputEvent::KeyDown(42).encode()),
        ] {
            Frame {
                kind,
                epoch: 17,
                sequence,
                payload,
            }
            .write_to(&mut writer)
            .unwrap();
        }
        let feedback = Frame::read_from(&mut reader).unwrap();
        assert_eq!(
            (feedback.kind, feedback.payload.as_slice()),
            (Kind::Control, &b"RETURN"[..])
        );
        Frame {
            kind: Kind::Control,
            epoch: 17,
            sequence: 3,
            payload: b"END".to_vec(),
        }
        .write_to(&mut writer)
        .unwrap();
        assert_eq!(
            server.join().unwrap(),
            vec![InputEvent::KeyDown(42), InputEvent::KeyUp(42)]
        );
    }

    #[test]
    fn local_revocation_releases_held_input_before_next_frame() {
        let mut bytes = Vec::new();
        for frame in [
            Frame {
                kind: Kind::Control,
                epoch: 11,
                sequence: 0,
                payload: b"BEGIN".to_vec(),
            },
            Frame {
                kind: Kind::Input,
                epoch: 11,
                sequence: 1,
                payload: InputEvent::KeyDown(42).encode(),
            },
            Frame {
                kind: Kind::Input,
                epoch: 11,
                sequence: 2,
                payload: InputEvent::ButtonDown(272).encode(),
            },
        ] {
            frame.write_to(&mut bytes).unwrap();
        }
        let mut checks = 0;
        let output = run_receiver_with_first_until(
            &mut bytes.as_slice(),
            RecordingInjector::default(),
            None,
            || {
                checks += 1;
                checks > 3
            },
        )
        .unwrap();
        assert_eq!(
            output.0,
            vec![
                InputEvent::KeyDown(42),
                InputEvent::ButtonDown(272),
                InputEvent::KeyUp(42),
                InputEvent::ButtonUp(272),
            ]
        );
    }
}
