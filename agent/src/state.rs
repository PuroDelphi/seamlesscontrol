//! Input ownership and held-input ledger. A receiver must release all held
//! keys/buttons whenever a session changes or its authenticated peer drops.

use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InputEvent {
    KeyDown(u32),
    KeyUp(u32),
    ButtonDown(u32),
    ButtonUp(u32),
    Motion { dx: i32, dy: i32 },
    Scroll { horizontal: i32, vertical: i32 },
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
}
