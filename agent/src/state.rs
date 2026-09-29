//! Input ownership and held-input ledger. A receiver must release all held
//! keys/buttons whenever a session changes or its authenticated peer drops.

use std::collections::BTreeSet;
use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InputEvent {
    KeyDown(u32),
    KeyUp(u32),
    ButtonDown(u32),
    ButtonUp(u32),
    /// Relative distance in thousandths of a logical pixel.
    Motion {
        dx_milli: i32,
        dy_milli: i32,
    },
    /// Scroll distance in thousandths of the input backend's scroll unit.
    Scroll {
        horizontal_milli: i32,
        vertical_milli: i32,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EventError {
    UnknownTag(u8),
    InvalidLength(usize),
}

impl fmt::Display for EventError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownTag(tag) => write!(f, "unknown input event tag: {tag}"),
            Self::InvalidLength(size) => write!(f, "invalid input event length: {size}"),
        }
    }
}

impl std::error::Error for EventError {}

impl InputEvent {
    pub fn encode(&self) -> Vec<u8> {
        let (tag, first, second) = match self {
            Self::KeyDown(value) => (1, *value, None),
            Self::KeyUp(value) => (2, *value, None),
            Self::ButtonDown(value) => (3, *value, None),
            Self::ButtonUp(value) => (4, *value, None),
            Self::Motion { dx_milli, dy_milli } => (5, *dx_milli as u32, Some(*dy_milli as u32)),
            Self::Scroll {
                horizontal_milli,
                vertical_milli,
            } => (6, *horizontal_milli as u32, Some(*vertical_milli as u32)),
        };
        let mut out = Vec::with_capacity(if second.is_some() { 9 } else { 5 });
        out.push(tag);
        out.extend_from_slice(&first.to_be_bytes());
        if let Some(value) = second {
            out.extend_from_slice(&value.to_be_bytes());
        }
        out
    }

    pub fn decode(payload: &[u8]) -> Result<Self, EventError> {
        if payload.is_empty() {
            return Err(EventError::InvalidLength(0));
        }
        let expected = if payload[0] <= 4 { 5 } else { 9 };
        if payload.len() != expected {
            return Err(EventError::InvalidLength(payload.len()));
        }
        let first = u32::from_be_bytes(payload[1..5].try_into().expect("checked length"));
        let second = if expected == 9 {
            u32::from_be_bytes(payload[5..9].try_into().expect("checked length"))
        } else {
            0
        };
        match payload[0] {
            1 => Ok(Self::KeyDown(first)),
            2 => Ok(Self::KeyUp(first)),
            3 => Ok(Self::ButtonDown(first)),
            4 => Ok(Self::ButtonUp(first)),
            5 => Ok(Self::Motion {
                dx_milli: first as i32,
                dy_milli: second as i32,
            }),
            6 => Ok(Self::Scroll {
                horizontal_milli: first as i32,
                vertical_milli: second as i32,
            }),
            tag => Err(EventError::UnknownTag(tag)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ApplyResult {
    Accepted,
    Duplicate,
    Stale,
}

#[derive(Default, Debug)]
pub struct Receiver {
    epoch: Option<u64>,
    last_sequence: Option<u64>,
    held_keys: BTreeSet<u32>,
    held_buttons: BTreeSet<u32>,
}

impl Receiver {
    pub fn new() -> Self {
        Self::default()
    }

    /// Start an authenticated control epoch. The caller sends the returned
    /// releases to the injector before accepting input from the new epoch.
    pub fn begin(&mut self, epoch: u64) -> Vec<InputEvent> {
        let releases = self.release_all();
        self.epoch = Some(epoch);
        self.last_sequence = None;
        releases
    }

    pub fn apply(&mut self, epoch: u64, sequence: u64, event: &InputEvent) -> ApplyResult {
        if self.epoch != Some(epoch) {
            return ApplyResult::Stale;
        }
        if self.last_sequence.is_some_and(|last| sequence <= last) {
            return ApplyResult::Duplicate;
        }
        self.last_sequence = Some(sequence);
        match event {
            InputEvent::KeyDown(key) => {
                self.held_keys.insert(*key);
            }
            InputEvent::KeyUp(key) => {
                self.held_keys.remove(key);
            }
            InputEvent::ButtonDown(button) => {
                self.held_buttons.insert(*button);
            }
            InputEvent::ButtonUp(button) => {
                self.held_buttons.remove(button);
            }
            InputEvent::Motion { .. } | InputEvent::Scroll { .. } => {}
        }
        ApplyResult::Accepted
    }

    /// Disconnect, pause, lock, or watchdog expiry all use this path.
    pub fn disconnect(&mut self) -> Vec<InputEvent> {
        let releases = self.release_all();
        self.epoch = None;
        self.last_sequence = None;
        releases
    }

    fn release_all(&mut self) -> Vec<InputEvent> {
        let mut releases = Vec::with_capacity(self.held_keys.len() + self.held_buttons.len());
        releases.extend(self.held_keys.iter().copied().map(InputEvent::KeyUp));
        releases.extend(self.held_buttons.iter().copied().map(InputEvent::ButtonUp));
        self.held_keys.clear();
        self.held_buttons.clear();
        releases
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disconnect_releases_every_pressed_input() {
        let mut r = Receiver::new();
        assert!(r.begin(8).is_empty());
        assert_eq!(
            r.apply(8, 1, &InputEvent::KeyDown(42)),
            ApplyResult::Accepted
        );
        assert_eq!(
            r.apply(8, 2, &InputEvent::ButtonDown(272)),
            ApplyResult::Accepted
        );
        assert_eq!(
            r.disconnect(),
            vec![InputEvent::KeyUp(42), InputEvent::ButtonUp(272)]
        );
        assert_eq!(r.apply(8, 3, &InputEvent::KeyDown(42)), ApplyResult::Stale);
    }

    #[test]
    fn old_or_duplicate_events_cannot_repress_a_key() {
        let mut r = Receiver::new();
        r.begin(1);
        assert_eq!(
            r.apply(1, 2, &InputEvent::KeyDown(30)),
            ApplyResult::Accepted
        );
        assert_eq!(
            r.apply(1, 2, &InputEvent::KeyDown(30)),
            ApplyResult::Duplicate
        );
        assert_eq!(r.begin(2), vec![InputEvent::KeyUp(30)]);
        assert_eq!(r.apply(1, 3, &InputEvent::KeyDown(30)), ApplyResult::Stale);
        assert!(r.disconnect().is_empty());
    }

    #[test]
    fn input_events_have_stable_binary_encoding() {
        let samples = [
            InputEvent::KeyDown(42),
            InputEvent::ButtonUp(272),
            InputEvent::Motion {
                dx_milli: -1250,
                dy_milli: 500,
            },
            InputEvent::Scroll {
                horizontal_milli: 0,
                vertical_milli: -2000,
            },
        ];
        for event in samples {
            assert_eq!(InputEvent::decode(&event.encode()).unwrap(), event);
        }
        assert_eq!(
            InputEvent::decode(&[5, 1]).unwrap_err(),
            EventError::InvalidLength(2)
        );
        assert_eq!(
            InputEvent::decode(&[9, 0, 0, 0, 0, 0, 0, 0, 0]).unwrap_err(),
            EventError::UnknownTag(9)
        );
    }
}
