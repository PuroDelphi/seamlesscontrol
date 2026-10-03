//! Interactive Windows input adapter for an authenticated Omarchy source.

use crate::clipboard::{ClipboardPacket, ClipboardSync};
use crate::protocol::{EntryPosition, Frame, Kind, ReturnRequest};
use crate::receiver::Injector;
use crate::secure::SecureWriter;
use crate::state::InputEvent;
use crate::topology::{EdgeReturnDetector, Rect, edge_entry_point, edge_fraction};
use crate::windows_clipboard::WindowsClipboard;
use crate::windows_keymap::evdev_to_set1;
use std::io;
use std::net::TcpStream;
use windows_sys::Win32::Foundation::POINT;
use windows_sys::Win32::System::StationsAndDesktops::{
    CloseDesktop, DESKTOP_READOBJECTS, GetUserObjectInformationW, OpenInputDesktop, UOI_NAME,
};
use windows_sys::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY,
    KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE, MOUSEEVENTF_HWHEEL, MOUSEEVENTF_LEFTDOWN,
    MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_MOVE,
    MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_WHEEL, MOUSEEVENTF_XDOWN,
    MOUSEEVENTF_XUP, MOUSEINPUT, SendInput,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
    SM_YVIRTUALSCREEN, SetCursorPos,
};

fn last_error(context: &'static str) -> io::Error {
    io::Error::other(format!("{context}: {}", io::Error::last_os_error()))
}

pub fn set_dpi_awareness() {
    // Call before querying physical screen coordinates. A process manifest may
    // already have selected this context, in which case the call can fail.
    unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
}

/// Fail closed when the input desktop is the lock screen or cannot be read.
pub fn interactive_desktop() -> io::Result<bool> {
    let desktop = unsafe { OpenInputDesktop(0, 0, DESKTOP_READOBJECTS) };
    if desktop.is_null() {
        return Err(last_error("OpenInputDesktop failed"));
    }
    let mut name = [0u16; 64];
    let mut needed = 0u32;
    let result = unsafe {
        GetUserObjectInformationW(
            desktop,
            UOI_NAME,
            name.as_mut_ptr().cast(),
            std::mem::size_of_val(&name) as u32,
            &mut needed,
        )
    };
    unsafe { CloseDesktop(desktop) };
    if result == 0 {
        return Err(last_error("cannot identify input desktop"));
    }
    let end = name
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(name.len());
    Ok(String::from_utf16_lossy(&name[..end]).eq_ignore_ascii_case("Default"))
}

fn screen_rect() -> io::Result<Rect> {
    let x = unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) };
    let y = unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) };
    let width = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) };
    let height = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) };
    Rect::new(x, y, width, height)
        .ok_or_else(|| io::Error::other("Windows virtual screen geometry is invalid"))
}

fn cursor_pos() -> io::Result<(i32, i32)> {
    let mut point = POINT::default();
    if unsafe { GetCursorPos(&mut point) } == 0 {
        return Err(last_error("GetCursorPos failed"));
    }
    Ok((point.x, point.y))
}

fn send(input: INPUT) -> io::Result<()> {
    if unsafe { SendInput(1, &input, size_of::<INPUT>() as i32) } != 1 {
        // UIPI can block injection without a useful last-error code.
        return Err(last_error("SendInput failed or was blocked by Windows"));
    }
    Ok(())
}

fn mouse(flags: u32, data: u32, dx: i32, dy: i32) -> io::Result<()> {
    send(INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx,
                dy,
                mouseData: data,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    })
}

fn key(evdev: u32, pressed: bool) -> io::Result<()> {
    let Some(scan) = evdev_to_set1(evdev) else {
        eprintln!("SeamlessControl: unsupported evdev key {evdev}; ignoring it");
        return Ok(());
    };
    let flags = KEYEVENTF_SCANCODE
        | if scan.extended {
            KEYEVENTF_EXTENDEDKEY
        } else {
            0
        }
        | if pressed { 0 } else { KEYEVENTF_KEYUP };
    send(INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: 0,
                wScan: scan.code,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    })
}

fn button(evdev: u32, pressed: bool) -> io::Result<()> {
    let (flag, data) = match (evdev, pressed) {
        (272, true) => (MOUSEEVENTF_LEFTDOWN, 0),
        (272, false) => (MOUSEEVENTF_LEFTUP, 0),
        (273, true) => (MOUSEEVENTF_RIGHTDOWN, 0),
        (273, false) => (MOUSEEVENTF_RIGHTUP, 0),
        (274, true) => (MOUSEEVENTF_MIDDLEDOWN, 0),
        (274, false) => (MOUSEEVENTF_MIDDLEUP, 0),
        (275, true) => (MOUSEEVENTF_XDOWN, 1),
        (275, false) => (MOUSEEVENTF_XUP, 1),
        (276, true) => (MOUSEEVENTF_XDOWN, 2),
        (276, false) => (MOUSEEVENTF_XUP, 2),
        _ => return Ok(()),
    };
    mouse(flag, data, 0, 0)
}

pub struct WindowsInjector {
    writer: SecureWriter<TcpStream>,
    clipboard: WindowsClipboard,
    clipboard_sync: ClipboardSync,
    epoch: Option<u64>,
    sequence: u64,
    return_sent: bool,
    return_detector: Option<EdgeReturnDetector>,
    return_edge: Option<crate::topology::Edge>,
    screen: Rect,
    motion_remainder: (i64, i64),
    scroll_remainder: (i64, i64),
}

impl WindowsInjector {
    pub fn new(
        writer: SecureWriter<TcpStream>,
        local_id: [u8; 32],
        peer_id: [u8; 32],
    ) -> io::Result<Self> {
        Ok(Self {
            writer,
            clipboard: WindowsClipboard::new(),
            clipboard_sync: ClipboardSync::new(local_id, peer_id),
            epoch: None,
            sequence: 0,
            return_sent: false,
            return_detector: None,
            return_edge: None,
            screen: screen_rect()?,
            motion_remainder: (0, 0),
            scroll_remainder: (0, 0),
        })
    }

    fn send_feedback(&mut self, kind: Kind, epoch: u64, payload: Vec<u8>) -> io::Result<()> {
        self.sequence = self.sequence.checked_add(1).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "feedback sequence exhausted")
        })?;
        Frame {
            kind,
            epoch,
            sequence: self.sequence,
            payload,
        }
        .write_to(&mut self.writer)
        .map_err(io::Error::other)
    }

    fn feedback(&mut self, epoch: u64, payload: Vec<u8>) -> io::Result<()> {
        self.send_feedback(Kind::Control, epoch, payload)
    }

    fn maybe_return(&mut self) -> io::Result<()> {
        let (Some(epoch), Some(edge)) = (self.epoch, self.return_edge) else {
            return Ok(());
        };
        if self.return_sent {
            return Ok(());
        }
        let (x, y) = cursor_pos()?;
        if self
            .return_detector
            .as_mut()
            .is_some_and(|detector| detector.sample(x, y))
        {
            let fraction = edge_fraction(&[self.screen], edge, x, y).unwrap_or(u16::MAX / 2);
            self.feedback(
                epoch,
                ReturnRequest {
                    exit_edge: edge,
                    fraction,
                }
                .encode(),
            )?;
            self.return_sent = true;
        }
        Ok(())
    }
}

impl Injector for WindowsInjector {
    fn place_cursor(&mut self, entry: EntryPosition) -> io::Result<()> {
        if !interactive_desktop()? {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Windows desktop is locked",
            ));
        }
        self.screen = screen_rect()?;
        let (x, y) = edge_entry_point(&[self.screen], entry.edge, entry.fraction)
            .ok_or_else(|| io::Error::other("Windows entry edge is unavailable"))?;
        if unsafe { SetCursorPos(x, y) } == 0 {
            return Err(last_error("SetCursorPos failed"));
        }
        self.return_edge = Some(entry.edge);
        self.return_detector = Some(EdgeReturnDetector::new(&[self.screen], entry.edge));
        self.return_sent = false;
        self.motion_remainder = (0, 0);
        self.scroll_remainder = (0, 0);
        Ok(())
    }

    fn inject(&mut self, event: &InputEvent) -> io::Result<()> {
        if matches!(
            event,
            InputEvent::KeyDown(_) | InputEvent::ButtonDown(_) | InputEvent::Motion { .. }
        ) && !interactive_desktop()?
        {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Windows desktop is locked",
            ));
        }
        match event {
            InputEvent::KeyDown(value) => key(*value, true),
            InputEvent::KeyUp(value) => key(*value, false),
            InputEvent::ButtonDown(value) => button(*value, true),
            InputEvent::ButtonUp(value) => button(*value, false),
            InputEvent::Motion { dx_milli, dy_milli } => {
                let x = self.motion_remainder.0 + i64::from(*dx_milli);
                let y = self.motion_remainder.1 + i64::from(*dy_milli);
                let dx = i32::try_from(x / 1000).map_err(io::Error::other)?;
                let dy = i32::try_from(y / 1000).map_err(io::Error::other)?;
                self.motion_remainder = (x % 1000, y % 1000);
                if dx != 0 || dy != 0 {
                    mouse(MOUSEEVENTF_MOVE, 0, dx, dy)?;
                }
                self.maybe_return()
            }
            InputEvent::Scroll {
                horizontal_milli,
                vertical_milli,
            } => {
                // EIS scroll deltas use logical units. Approximate 15 units per
                // Windows wheel detent and retain fractional steps.
                let x = self.scroll_remainder.0 + i64::from(*horizontal_milli) * 8;
                let y = self.scroll_remainder.1 + i64::from(*vertical_milli) * 8;
                let horizontal = i32::try_from(x / 1000).map_err(io::Error::other)?;
                let vertical = i32::try_from(y / 1000).map_err(io::Error::other)?;
                self.scroll_remainder = (x % 1000, y % 1000);
                if horizontal != 0 {
                    mouse(MOUSEEVENTF_HWHEEL, horizontal as u32, 0, 0)?;
                }
                if vertical != 0 {
                    mouse(MOUSEEVENTF_WHEEL, vertical as u32, 0, 0)?;
                }
                Ok(())
            }
        }
    }

    fn active_epoch_changed(&mut self, epoch: Option<u64>) {
        self.epoch = epoch;
        if epoch.is_some() {
            self.return_sent = false;
        }
    }

    fn clipboard_received(&mut self, packet: ClipboardPacket) -> io::Result<()> {
        if !self.clipboard_sync.remote_needs_apply(&packet)? {
            return Ok(());
        }
        if interactive_desktop().unwrap_or(false) {
            if let Err(error) = self.clipboard.apply(&packet.event) {
                eprintln!("SeamlessControl: Windows clipboard apply failed: {error}");
            } else {
                self.clipboard_sync.remote_applied(&packet);
            }
        }
        Ok(())
    }

    fn heartbeat(&mut self) -> io::Result<()> {
        if !interactive_desktop().unwrap_or(false) {
            return Ok(());
        }
        match self.clipboard.changed() {
            Ok(Some(event)) => {
                if let Some(packet) = self.clipboard_sync.local_changed(&event) {
                    self.send_feedback(Kind::Clipboard, 0, packet.encode())?;
                }
            }
            Ok(None) => {}
            Err(error) => eprintln!("SeamlessControl: Windows clipboard read failed: {error}"),
        }
        Ok(())
    }

    fn control_released(&mut self, epoch: u64) -> io::Result<()> {
        self.feedback(epoch, b"ENDED".to_vec())
    }
}
