//! Local crossing preference. The first deliberate crossing is discarded;
//! a second crossing within a short interval transfers control.

use std::io;
use std::path::Path;
use std::time::{Duration, Instant};

const DOUBLE_CROSS_WINDOW: Duration = Duration::from_millis(1600);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum EdgePolicy {
    #[default]
    Fluid,
    Deliberate,
    Fullscreen,
}

impl EdgePolicy {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "fluid" => Some(Self::Fluid),
            "deliberate" => Some(Self::Deliberate),
            "fullscreen" => Some(Self::Fullscreen),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fluid => "fluid",
            Self::Deliberate => "deliberate",
            Self::Fullscreen => "fullscreen",
        }
    }

    pub fn load(config: &Path) -> Self {
        std::fs::read_to_string(config.join("edge-policy"))
            .ok()
            .and_then(|value| Self::parse(value.trim()))
            .unwrap_or_default()
    }

    pub fn save(self, config: &Path) -> io::Result<()> {
        std::fs::create_dir_all(config)?;
        std::fs::write(config.join("edge-policy"), self.as_str())
    }
}

#[derive(Debug, Default)]
pub struct EdgeGate {
    first_crossing: Option<Instant>,
}

impl EdgeGate {
    pub fn allow(&mut self, policy: EdgePolicy, fullscreen: bool, now: Instant) -> bool {
        if policy == EdgePolicy::Fluid || (policy == EdgePolicy::Fullscreen && !fullscreen) {
            self.first_crossing = None;
            return true;
        }
        if self
            .first_crossing
            .is_some_and(|first| now.duration_since(first) <= DOUBLE_CROSS_WINDOW)
        {
            self.first_crossing = None;
            true
        } else {
            self.first_crossing = Some(now);
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deliberate_crossing_needs_two_attempts_within_window() {
        let now = Instant::now();
        let mut gate = EdgeGate::default();
        assert!(!gate.allow(EdgePolicy::Deliberate, false, now));
        assert!(gate.allow(
            EdgePolicy::Deliberate,
            false,
            now + Duration::from_millis(800)
        ));
        assert!(!gate.allow(EdgePolicy::Deliberate, false, now + Duration::from_secs(3)));
        assert!(!gate.allow(EdgePolicy::Deliberate, false, now + Duration::from_secs(5)));
    }

    #[test]
    fn fullscreen_guard_only_requires_two_crossings_during_fullscreen() {
        let now = Instant::now();
        let mut gate = EdgeGate::default();
        assert!(!gate.allow(EdgePolicy::Fullscreen, true, now));
        assert!(gate.allow(
            EdgePolicy::Fullscreen,
            false,
            now + Duration::from_millis(100)
        ));
        assert!(!gate.allow(
            EdgePolicy::Fullscreen,
            true,
            now + Duration::from_millis(200)
        ));
    }
}
