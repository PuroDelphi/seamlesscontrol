//! Bounded, read-only Hyprland IPC queries for destination cursor geometry.

use crate::topology::Rect;
use serde_json::Value;
use std::io::{self, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::Duration;

const MAX_REPLY: u64 = 128 * 1024;

#[derive(Clone)]
pub struct HyprIpc {
    socket: PathBuf,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionLockState {
    Locked,
    Unlocked,
    Undetermined,
}

impl HyprIpc {
    pub fn from_env() -> Option<Self> {
        let runtime = std::env::var_os("XDG_RUNTIME_DIR")?;
        let signature = std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE")?;
        let signature = signature.to_str()?;
        if signature.is_empty() || signature.contains('/') || signature.contains('\\') {
            return None;
        }
        Some(Self {
            socket: PathBuf::from(runtime)
                .join("hypr")
                .join(signature)
                .join(".socket.sock"),
        })
    }

    fn query(&self, command: &[u8]) -> io::Result<Vec<u8>> {
        let mut stream = UnixStream::connect(&self.socket)?;
        stream.set_read_timeout(Some(Duration::from_millis(250)))?;
        stream.set_write_timeout(Some(Duration::from_millis(250)))?;
        stream.write_all(command)?;
        stream.shutdown(std::net::Shutdown::Write)?;
        let mut reply = Vec::new();
        stream.take(MAX_REPLY + 1).read_to_end(&mut reply)?;
        if reply.len() as u64 > MAX_REPLY {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Hyprland reply too large",
            ));
        }
        Ok(reply)
    }

    pub fn cursor_position(&self) -> io::Result<(i32, i32)> {
        parse_cursor(&self.query(b"j/cursorpos")?)
    }

    pub fn monitor_rects(&self) -> io::Result<Vec<Rect>> {
        parse_monitors(&self.query(b"j/monitors")?)
    }

    pub fn session_lock_state(&self) -> io::Result<SessionLockState> {
        parse_session_lock(&self.query(b"j/monitors")?)
    }

    pub fn active_window_fullscreen(&self) -> io::Result<bool> {
        let value: Value =
            serde_json::from_slice(&self.query(b"j/activewindow")?).map_err(io::Error::other)?;
        Ok(value
            .get("fullscreen")
            .and_then(|value| value.as_i64().or_else(|| value.as_bool().map(i64::from)))
            .is_some_and(|mode| mode > 0))
    }
}

/// Mirror Omarchy's ext-session-lock probe. A monitor without a workspace
/// cannot establish that the session is unlocked.
pub fn parse_session_lock(raw: &[u8]) -> io::Result<SessionLockState> {
    let value: Value = serde_json::from_slice(raw).map_err(io::Error::other)?;
    let monitors = value
        .as_array()
        .ok_or_else(|| invalid_data("invalid monitor list"))?;
    if monitors.is_empty() || monitors.len() > 64 {
        return Ok(SessionLockState::Undetermined);
    }
    let mut readable = false;
    let mut unknown = false;
    for monitor in monitors {
        let blockers = match monitor.get("solitaryBlockedBy") {
            Some(Value::Array(blockers)) => blockers.as_slice(),
            // Hyprland reports null when nothing blocks the active workspace.
            // It is not an unknown lock state in this case.
            Some(Value::Null)
                if monitor
                    .get("activeWorkspace")
                    .and_then(|workspace| workspace.get("id"))
                    .and_then(Value::as_i64)
                    .is_some_and(|id| id > 0) =>
            {
                &[]
            }
            _ => {
                unknown = true;
                continue;
            }
        };
        if blockers.iter().any(|value| value.as_str() == Some("LOCK")) {
            return Ok(SessionLockState::Locked);
        }
        if !blockers
            .iter()
            .any(|value| value.as_str() == Some("WORKSPACE"))
        {
            readable = true;
        }
    }
    Ok(if readable && !unknown {
        SessionLockState::Unlocked
    } else {
        SessionLockState::Undetermined
    })
}

fn invalid_data(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn finite_i32(value: &Value) -> Option<i32> {
    let number = value.as_f64()?;
    if !number.is_finite() || number < f64::from(i32::MIN) || number > f64::from(i32::MAX) {
        return None;
    }
    Some(number.round() as i32)
}

pub fn parse_cursor(raw: &[u8]) -> io::Result<(i32, i32)> {
    let value: Value = serde_json::from_slice(raw).map_err(io::Error::other)?;
    let x = value
        .get("x")
        .and_then(finite_i32)
        .ok_or_else(|| invalid_data("invalid cursor x"))?;
    let y = value
        .get("y")
        .and_then(finite_i32)
        .ok_or_else(|| invalid_data("invalid cursor y"))?;
    Ok((x, y))
}

pub fn parse_monitors(raw: &[u8]) -> io::Result<Vec<Rect>> {
    let value: Value = serde_json::from_slice(raw).map_err(io::Error::other)?;
    let monitors = value
        .as_array()
        .ok_or_else(|| invalid_data("invalid monitor list"))?;
    if monitors.is_empty() || monitors.len() > 64 {
        return Err(invalid_data("invalid monitor count"));
    }
    monitors
        .iter()
        .map(|monitor| {
            let x = monitor
                .get("x")
                .and_then(finite_i32)
                .ok_or_else(|| invalid_data("invalid monitor x"))?;
            let y = monitor
                .get("y")
                .and_then(finite_i32)
                .ok_or_else(|| invalid_data("invalid monitor y"))?;
            let width = monitor
                .get("width")
                .and_then(Value::as_f64)
                .ok_or_else(|| invalid_data("invalid monitor width"))?;
            let height = monitor
                .get("height")
                .and_then(Value::as_f64)
                .ok_or_else(|| invalid_data("invalid monitor height"))?;
            let scale = monitor
                .get("scale")
                .and_then(Value::as_f64)
                .ok_or_else(|| invalid_data("invalid monitor scale"))?;
            let transform = monitor
                .get("transform")
                .and_then(Value::as_i64)
                .unwrap_or(0);
            // Rotated output geometry needs a physical compositor test before it
            // can safely trigger an automatic return.
            if !width.is_finite()
                || !height.is_finite()
                || !scale.is_finite()
                || width <= 0.0
                || height <= 0.0
                || !(0.5..=8.0).contains(&scale)
                || !matches!(transform, 0 | 2 | 4 | 6)
            {
                return Err(invalid_data("unsupported monitor geometry"));
            }
            let width = finite_i32(&Value::from((width / scale).round()))
                .ok_or_else(|| invalid_data("invalid logical width"))?;
            let height = finite_i32(&Value::from((height / scale).round()))
                .ok_or_else(|| invalid_data("invalid logical height"))?;
            Rect::new(x, y, width, height)
                .ok_or_else(|| invalid_data("invalid logical monitor rectangle"))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::net::UnixListener;
    use std::thread;

    #[test]
    fn cursor_and_scaled_monitors_parse_with_bounds() {
        assert_eq!(parse_cursor(br#"{"x":123.4,"y":-5}"#).unwrap(), (123, -5));
        assert_eq!(
            parse_monitors(
                br#"[{"x":-1280,"y":0,"width":2560,"height":1440,"scale":2,"transform":0}]"#
            )
            .unwrap(),
            vec![Rect::new(-1280, 0, 1280, 720).unwrap()]
        );
        assert!(
            parse_monitors(
                br#"[{"x":0,"y":0,"width":1920,"height":1080,"scale":1,"transform":1}]"#
            )
            .is_err()
        );
        assert!(parse_cursor(br#"{"x":"bad","y":0}"#).is_err());
    }

    #[test]
    fn lock_state_fails_closed_and_detects_orphaned_session_lock() {
        assert_eq!(
            parse_session_lock(br#"[{"solitaryBlockedBy":["WINDOWED","CANDIDATE"]}]"#).unwrap(),
            SessionLockState::Unlocked
        );
        assert_eq!(
            parse_session_lock(br#"[{"activeWorkspace":{"id":2},"solitaryBlockedBy":null}]"#)
                .unwrap(),
            SessionLockState::Unlocked
        );
        assert_eq!(
            parse_session_lock(br#"[{"activeWorkspace":{"id":0},"solitaryBlockedBy":null}]"#)
                .unwrap(),
            SessionLockState::Undetermined
        );
        assert_eq!(
            parse_session_lock(br#"[{"solitaryBlockedBy":["LOCK"]}]"#).unwrap(),
            SessionLockState::Locked
        );
        assert_eq!(
            parse_session_lock(br#"[{"solitaryBlockedBy":["WORKSPACE"]}]"#).unwrap(),
            SessionLockState::Undetermined
        );
        assert_eq!(
            parse_session_lock(br#"[{"name":"old Hyprland"}]"#).unwrap(),
            SessionLockState::Undetermined
        );
        assert_eq!(
            parse_session_lock(br#"[{"name":"unknown"},{"solitaryBlockedBy":["LOCK"]}]"#).unwrap(),
            SessionLockState::Locked
        );
        assert!(parse_session_lock(b"not json").is_err());
    }

    #[test]
    fn lock_probe_reads_compositor_without_shell_command() {
        let root =
            std::env::temp_dir().join(format!("seamlesscontrol-lock-ipc-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&root);
        let path = root.join("query.sock");
        let _ = std::fs::remove_file(&path);
        let listener = UnixListener::bind(&path).unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = Vec::new();
            stream.read_to_end(&mut request).unwrap();
            assert_eq!(&request, b"j/monitors");
            stream
                .write_all(br#"[{"solitaryBlockedBy":["LOCK"]}]"#)
                .unwrap();
        });
        let ipc = HyprIpc {
            socket: path.clone(),
        };
        assert_eq!(ipc.session_lock_state().unwrap(), SessionLockState::Locked);
        server.join().unwrap();
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(root).unwrap();
    }

    #[test]
    fn read_only_ipc_uses_bounded_unix_request() {
        let root =
            std::env::temp_dir().join(format!("seamlesscontrol-hypr-ipc-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&root);
        let path = root.join("query.sock");
        let _ = std::fs::remove_file(&path);
        let listener = UnixListener::bind(&path).unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = Vec::new();
            stream.read_to_end(&mut request).unwrap();
            assert_eq!(&request, b"j/cursorpos");
            stream.write_all(br#"{"x":7,"y":9}"#).unwrap();
        });
        let ipc = HyprIpc {
            socket: path.clone(),
        };
        assert_eq!(ipc.cursor_position().unwrap(), (7, 9));
        server.join().unwrap();
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(root).unwrap();
    }
}
