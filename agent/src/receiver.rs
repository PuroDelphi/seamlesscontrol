//! Authenticated input receiver. This module contains no platform calls, so
//! disconnect and malformed-frame behavior can be tested without a desktop.

use crate::clipboard::ClipboardPacket;
use crate::protocol::{EntryPosition, Frame, FrameError, Kind};
use crate::state::{ApplyResult, EventError, InputEvent, Receiver};
use std::fmt;
use std::io::{self, Read};

pub trait Injector {
    fn inject(&mut self, event: &InputEvent) -> io::Result<()>;

    fn place_cursor(&mut self, _entry: EntryPosition) -> io::Result<()> {
        Ok(())
    }

    fn clipboard_received(&mut self, _packet: ClipboardPacket) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "clipboard receiver unavailable",
        ))
    }

    fn ownership_changed(&mut self, _controlling: bool) {}

    fn active_epoch_changed(&mut self, _epoch: Option<u64>) {}

    fn control_released(&mut self, _epoch: u64) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "release acknowledgement unavailable",
        ))
    }
}

#[derive(Debug)]
pub enum ReceiverError {
    Frame(FrameError),
    Event(EventError),
    Clipboard(io::Error),
    Injection(io::Error),
    UnexpectedFrame,
}

impl fmt::Display for ReceiverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Frame(e) => write!(f, "frame error: {e}"),
            Self::Event(e) => write!(f, "input event error: {e}"),
            Self::Clipboard(e) => write!(f, "clipboard error: {e}"),
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
    active_epoch: Option<u64>,
    last_epoch: Option<u64>,
}

impl<I: Injector> InputReceiver<I> {
    pub fn new(injector: I) -> Self {
        Self {
            injector,
            ledger: Receiver::new(),
            controlling: false,
            active_epoch: None,
            last_epoch: None,
        }
    }

    pub fn handle(&mut self, frame: &Frame) -> Result<(), ReceiverError> {
        match frame.kind {
            Kind::Control if frame.payload == b"BEGIN" || frame.payload.starts_with(b"BEGIN\t") => {
                if frame.epoch == 0 || self.last_epoch.is_some_and(|last| frame.epoch <= last) {
                    return Ok(());
                }
                let entry = EntryPosition::parse_begin(&frame.payload)
                    .map_err(|_| ReceiverError::UnexpectedFrame)?;
                self.release()?;
                if let Some(entry) = entry {
                    self.injector
                        .place_cursor(entry)
                        .map_err(ReceiverError::Injection)?;
                }
                for event in self.ledger.begin(frame.epoch) {
                    self.injector
                        .inject(&event)
                        .map_err(ReceiverError::Injection)?;
                }
                self.controlling = true;
                self.active_epoch = Some(frame.epoch);
                self.last_epoch = Some(frame.epoch);
                self.injector.active_epoch_changed(Some(frame.epoch));
                self.injector.ownership_changed(true);
                Ok(())
            }
            Kind::Control if frame.payload == b"END" => {
                if self.active_epoch == Some(frame.epoch) {
                    self.release()
                } else {
                    Ok(())
                }
            }
            Kind::Control if frame.payload == b"RELEASE" => {
                if self.active_epoch != Some(frame.epoch) {
                    return Err(ReceiverError::UnexpectedFrame);
                }
                self.release()?;
                self.injector
                    .control_released(frame.epoch)
                    .map_err(ReceiverError::Injection)
            }
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
            Kind::Clipboard => {
                let packet =
                    ClipboardPacket::decode(&frame.payload).map_err(ReceiverError::Clipboard)?;
                self.injector
                    .clipboard_received(packet)
                    .map_err(ReceiverError::Clipboard)
            }
            _ => Err(ReceiverError::UnexpectedFrame),
        }
    }

    /// Always call after EOF, error, timeout, pause, or lock.
    pub fn release(&mut self) -> Result<(), ReceiverError> {
        self.active_epoch = None;
        if std::mem::take(&mut self.controlling) {
            self.injector.active_epoch_changed(None);
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
    use crate::clipboard::{ClipboardEvent, ClipboardSync};
    use crate::handoff::HandoffCoordinator;
    use crate::protocol::SwitchRequest;
    use crate::secure::{Identity, Role, SecureChannel};
    use crate::topology::{Edge, Machine, Slot, Topology};
    use std::net::{IpAddr, TcpListener, TcpStream};
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
    struct PositionedInjector {
        placements: Vec<EntryPosition>,
        input: Vec<InputEvent>,
    }

    impl Injector for PositionedInjector {
        fn inject(&mut self, event: &InputEvent) -> io::Result<()> {
            self.input.push(event.clone());
            Ok(())
        }

        fn place_cursor(&mut self, entry: EntryPosition) -> io::Result<()> {
            self.placements.push(entry);
            Ok(())
        }
    }

    #[derive(Debug, Eq, PartialEq)]
    enum ReleaseAction {
        Input(InputEvent),
        Acknowledged(u64),
    }

    #[derive(Default)]
    struct ReleaseInjector(Vec<ReleaseAction>);

    impl Injector for ReleaseInjector {
        fn inject(&mut self, event: &InputEvent) -> io::Result<()> {
            self.0.push(ReleaseAction::Input(event.clone()));
            Ok(())
        }

        fn control_released(&mut self, epoch: u64) -> io::Result<()> {
            self.0.push(ReleaseAction::Acknowledged(epoch));
            Ok(())
        }
    }

    #[test]
    fn explicit_release_acknowledges_only_after_held_input_is_lifted() {
        let mut receiver = InputReceiver::new(ReleaseInjector::default());
        let frame = |kind, epoch, sequence, payload: Vec<u8>| Frame {
            kind,
            epoch,
            sequence,
            payload,
        };
        receiver
            .handle(&frame(Kind::Control, 17, 1, b"BEGIN".to_vec()))
            .unwrap();
        receiver
            .handle(&frame(Kind::Input, 17, 2, InputEvent::KeyDown(42).encode()))
            .unwrap();
        receiver
            .handle(&frame(
                Kind::Input,
                17,
                3,
                InputEvent::ButtonDown(272).encode(),
            ))
            .unwrap();
        assert!(matches!(
            receiver.handle(&frame(Kind::Control, 16, 4, b"RELEASE".to_vec())),
            Err(ReceiverError::UnexpectedFrame)
        ));
        receiver
            .handle(&frame(Kind::Control, 17, 5, b"RELEASE".to_vec()))
            .unwrap();
        assert_eq!(
            receiver.into_injector().0,
            vec![
                ReleaseAction::Input(InputEvent::KeyDown(42)),
                ReleaseAction::Input(InputEvent::ButtonDown(272)),
                ReleaseAction::Input(InputEvent::KeyUp(42)),
                ReleaseAction::Input(InputEvent::ButtonUp(272)),
                ReleaseAction::Acknowledged(17),
            ]
        );
    }

    #[test]
    fn stale_control_frames_cannot_release_current_input() {
        let mut receiver = InputReceiver::new(RecordingInjector::default());
        let frame = |kind, epoch, sequence, payload: Vec<u8>| Frame {
            kind,
            epoch,
            sequence,
            payload,
        };
        receiver
            .handle(&frame(Kind::Control, 12, 1, b"BEGIN".to_vec()))
            .unwrap();
        receiver
            .handle(&frame(Kind::Input, 12, 2, InputEvent::KeyDown(42).encode()))
            .unwrap();
        receiver
            .handle(&frame(Kind::Control, 11, 3, b"BEGIN".to_vec()))
            .unwrap();
        receiver
            .handle(&frame(Kind::Control, 12, 4, b"BEGIN".to_vec()))
            .unwrap();
        receiver
            .handle(&frame(Kind::Control, 11, 5, b"END".to_vec()))
            .unwrap();
        assert_eq!(receiver.injector.0, vec![InputEvent::KeyDown(42)]);
        receiver
            .handle(&frame(Kind::Control, 12, 6, b"END".to_vec()))
            .unwrap();
        receiver
            .handle(&frame(Kind::Control, 12, 7, b"BEGIN".to_vec()))
            .unwrap();
        receiver
            .handle(&frame(Kind::Control, 13, 8, b"BEGIN".to_vec()))
            .unwrap();
        receiver
            .handle(&frame(Kind::Input, 13, 9, InputEvent::KeyDown(30).encode()))
            .unwrap();
        receiver
            .handle(&frame(Kind::Control, 13, 10, b"END".to_vec()))
            .unwrap();
        assert_eq!(
            receiver.into_injector().0,
            vec![
                InputEvent::KeyDown(42),
                InputEvent::KeyUp(42),
                InputEvent::KeyDown(30),
                InputEvent::KeyUp(30),
            ]
        );
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

    #[derive(Default)]
    struct ClipboardInjector(Vec<ClipboardPacket>);
    impl Injector for ClipboardInjector {
        fn inject(&mut self, _event: &InputEvent) -> io::Result<()> {
            Ok(())
        }

        fn clipboard_received(&mut self, packet: ClipboardPacket) -> io::Result<()> {
            self.0.push(packet);
            Ok(())
        }
    }

    #[test]
    fn clipboard_is_bounded_utf8_and_independent_of_input_capture() {
        let mut receiver = InputReceiver::new(ClipboardInjector::default());
        let text = ClipboardPacket {
            revision: 1,
            origin: [2; 32],
            event: ClipboardEvent::Text("hola 🙂".as_bytes().to_vec()),
        };
        receiver
            .handle(&Frame {
                kind: Kind::Clipboard,
                epoch: 0,
                sequence: 1,
                payload: text.encode(),
            })
            .unwrap();
        assert!(matches!(
            receiver.handle(&Frame {
                kind: Kind::Clipboard,
                epoch: 0,
                sequence: 2,
                payload: vec![1, 0xff],
            }),
            Err(ReceiverError::Clipboard(_))
        ));
        assert_eq!(receiver.into_injector().0, vec![text]);
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
                epoch: 17,
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
    fn encrypted_three_machine_handoff_releases_before_next_begin() {
        let b_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let c_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let b_address = b_listener.local_addr().unwrap();
        let c_address = c_listener.local_addr().unwrap();
        let a_id = Identity::generate().unwrap();
        let b_id = Identity::generate().unwrap();
        let c_id = Identity::generate().unwrap();
        let a_public = a_id.public;
        let b_public = b_id.public;
        let c_public = c_id.public;
        let b_ip = IpAddr::from([192, 168, 1, 2]);
        let c_ip = IpAddr::from([192, 168, 1, 3]);

        let b_server = thread::spawn(move || {
            let (stream, _) = b_listener.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(3)))
                .unwrap();
            let (mut channel, _) =
                SecureChannel::connect(stream, Role::Responder, &b_id, Some(&a_public), |_| false)
                    .unwrap();
            let mut receiver = InputReceiver::new(ReleaseInjector::default());
            for _ in 0..3 {
                receiver
                    .handle(&Frame::read_from(&mut channel).unwrap())
                    .unwrap();
            }
            Frame {
                kind: Kind::Control,
                epoch: 41,
                sequence: 1,
                payload: SwitchRequest {
                    target: c_ip,
                    exit_edge: Edge::Bottom,
                    fraction: 16_384,
                }
                .encode(),
            }
            .write_to(&mut channel)
            .unwrap();
            receiver
                .handle(&Frame::read_from(&mut channel).unwrap())
                .unwrap();
            let actions = receiver.into_injector().0;
            assert_eq!(
                actions,
                vec![
                    ReleaseAction::Input(InputEvent::KeyDown(42)),
                    ReleaseAction::Input(InputEvent::ButtonDown(272)),
                    ReleaseAction::Input(InputEvent::KeyUp(42)),
                    ReleaseAction::Input(InputEvent::ButtonUp(272)),
                    ReleaseAction::Acknowledged(41),
                ]
            );
            Frame {
                kind: Kind::Control,
                epoch: 41,
                sequence: 2,
                payload: b"ENDED".to_vec(),
            }
            .write_to(&mut channel)
            .unwrap();
        });
        let c_server = thread::spawn(move || {
            let (stream, _) = c_listener.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(3)))
                .unwrap();
            let (mut channel, _) =
                SecureChannel::connect(stream, Role::Responder, &c_id, Some(&a_public), |_| false)
                    .unwrap();
            let mut receiver = InputReceiver::new(PositionedInjector::default());
            for _ in 0..4 {
                receiver
                    .handle(&Frame::read_from(&mut channel).unwrap())
                    .unwrap();
            }
            receiver.into_injector()
        });

        let b_stream = TcpStream::connect(b_address).unwrap();
        b_stream
            .set_read_timeout(Some(std::time::Duration::from_secs(3)))
            .unwrap();
        let (mut b_channel, _) =
            SecureChannel::connect(b_stream, Role::Initiator, &a_id, Some(&b_public), |_| false)
                .unwrap();
        let c_stream = TcpStream::connect(c_address).unwrap();
        c_stream
            .set_read_timeout(Some(std::time::Duration::from_secs(3)))
            .unwrap();
        let (mut c_channel, _) =
            SecureChannel::connect(c_stream, Role::Initiator, &a_id, Some(&c_public), |_| false)
                .unwrap();

        let mut topology = Topology::new();
        topology
            .place(Machine::Peer(b_ip), Slot::new(1, 0).unwrap())
            .unwrap();
        topology
            .place(Machine::Peer(c_ip), Slot::new(1, 1).unwrap())
            .unwrap();
        let mut handoff = HandoffCoordinator::new(topology);
        handoff.activate_local_edge(Edge::Right, b_ip, 41).unwrap();
        for (sequence, kind, payload) in [
            (1, Kind::Control, b"BEGIN".to_vec()),
            (2, Kind::Input, InputEvent::KeyDown(42).encode()),
            (3, Kind::Input, InputEvent::ButtonDown(272).encode()),
        ] {
            Frame {
                kind,
                epoch: 41,
                sequence,
                payload,
            }
            .write_to(&mut b_channel)
            .unwrap();
        }
        let request = Frame::read_from(&mut b_channel).unwrap();
        assert_eq!(
            SwitchRequest::parse(&request.payload),
            Ok(SwitchRequest {
                target: c_ip,
                exit_edge: Edge::Bottom,
                fraction: 16_384,
            })
        );
        handoff
            .request(b_ip, request.epoch, Machine::Peer(c_ip))
            .unwrap();
        assert_eq!(handoff.owner(), Machine::Peer(b_ip));
        Frame {
            kind: Kind::Control,
            epoch: 41,
            sequence: 4,
            payload: b"RELEASE".to_vec(),
        }
        .write_to(&mut b_channel)
        .unwrap();
        let ended = Frame::read_from(&mut b_channel).unwrap();
        assert_eq!(
            (ended.kind, ended.epoch, ended.payload.as_slice()),
            (Kind::Control, 41, &b"ENDED"[..])
        );
        assert_eq!(
            handoff.acknowledge(b_ip, ended.epoch, 42),
            Ok(Machine::Peer(c_ip))
        );
        for (sequence, kind, payload) in [
            (
                1,
                Kind::Control,
                EntryPosition {
                    edge: Edge::Top,
                    fraction: 16_384,
                }
                .begin_payload(),
            ),
            (2, Kind::Input, InputEvent::KeyDown(42).encode()),
            (3, Kind::Input, InputEvent::ButtonDown(272).encode()),
            (4, Kind::Control, b"END".to_vec()),
        ] {
            Frame {
                kind,
                epoch: 42,
                sequence,
                payload,
            }
            .write_to(&mut c_channel)
            .unwrap();
        }
        b_server.join().unwrap();
        let c_injector = c_server.join().unwrap();
        assert_eq!(
            c_injector.placements,
            vec![EntryPosition {
                edge: Edge::Top,
                fraction: 16_384,
            }]
        );
        assert_eq!(
            c_injector.input,
            vec![
                InputEvent::KeyDown(42),
                InputEvent::ButtonDown(272),
                InputEvent::KeyUp(42),
                InputEvent::ButtonUp(272),
            ]
        );
    }

    #[test]
    fn encrypted_clipboard_exchange_remains_text_only_and_echo_free() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let initiator = Identity::generate().unwrap();
        let responder = Identity::generate().unwrap();
        let initiator_key = initiator.public;
        let responder_key = responder.public;
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let (channel, _) =
                SecureChannel::connect(stream, Role::Responder, &responder, None, |_| true)
                    .unwrap();
            let (mut reader, mut writer) = channel.into_tcp_halves().unwrap();
            let mut receiver = InputReceiver::new(ClipboardInjector::default());
            receiver
                .handle(&Frame::read_from(&mut reader).unwrap())
                .unwrap();
            Frame {
                kind: Kind::Clipboard,
                epoch: 0,
                sequence: 1,
                payload: ClipboardPacket {
                    revision: 2,
                    origin: responder_key,
                    event: ClipboardEvent::Text(b"destino".to_vec()),
                }
                .encode(),
            }
            .write_to(&mut writer)
            .unwrap();
            receiver.into_injector().0
        });
        let stream = TcpStream::connect(address).unwrap();
        let (channel, _) =
            SecureChannel::connect(stream, Role::Initiator, &initiator, None, |_| true).unwrap();
        let (mut reader, mut writer) = channel.into_tcp_halves().unwrap();
        let local = ClipboardPacket {
            revision: 1,
            origin: initiator_key,
            event: ClipboardEvent::Text("origen 🙂".as_bytes().to_vec()),
        };
        Frame {
            kind: Kind::Clipboard,
            epoch: 0,
            sequence: 1,
            payload: local.encode(),
        }
        .write_to(&mut writer)
        .unwrap();
        let remote =
            ClipboardPacket::decode(&Frame::read_from(&mut reader).unwrap().payload).unwrap();
        let mut sync = ClipboardSync::new(initiator_key, responder_key);
        assert!(sync.remote_needs_apply(&remote).unwrap());
        sync.remote_applied(&remote);
        assert!(sync.local_changed(&remote.event).is_none());
        assert_eq!(server.join().unwrap(), vec![local]);
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
