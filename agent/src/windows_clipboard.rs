//! Text-only Windows clipboard adapter. Non-text formats are left untouched.

use crate::clipboard::{ClipboardEvent, MAX_TEXT_BYTES};
use std::io;
use std::ptr;
use std::thread;
use std::time::Duration;
use windows_sys::Win32::Foundation::GlobalFree;
use windows_sys::Win32::System::DataExchange::{
    CloseClipboard, CountClipboardFormats, EmptyClipboard, GetClipboardData,
    GetClipboardSequenceNumber, IsClipboardFormatAvailable, OpenClipboard, SetClipboardData,
};
use windows_sys::Win32::System::Memory::{
    GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{CreateWindowExW, DestroyWindow};

const CF_UNICODETEXT: u32 = 13;
const MAX_UTF16_BYTES: usize = MAX_TEXT_BYTES * 2 + 2;

fn last_error(context: &'static str) -> io::Error {
    io::Error::other(format!("{context}: {}", io::Error::last_os_error()))
}

struct OpenedClipboard {
    owner: Option<windows_sys::Win32::Foundation::HWND>,
}

impl OpenedClipboard {
    fn open_for_read() -> io::Result<Option<Self>> {
        if unsafe { OpenClipboard(ptr::null_mut()) } == 0 {
            return Ok(None); // Another application may briefly own the clipboard.
        }
        Ok(Some(Self { owner: None }))
    }

    fn open_for_write() -> io::Result<Self> {
        let class: Vec<u16> = "STATIC\0".encode_utf16().collect();
        let owner = unsafe {
            CreateWindowExW(
                0,
                class.as_ptr(),
                ptr::null(),
                0,
                0,
                0,
                0,
                0,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null(),
            )
        };
        if owner.is_null() {
            return Err(last_error("clipboard owner window could not be created"));
        }
        for _ in 0..5 {
            if unsafe { OpenClipboard(owner) } != 0 {
                return Ok(Self { owner: Some(owner) });
            }
            thread::sleep(Duration::from_millis(30));
        }
        let error = last_error("OpenClipboard failed");
        unsafe { DestroyWindow(owner) };
        Err(error)
    }
}

impl Drop for OpenedClipboard {
    fn drop(&mut self) {
        unsafe { CloseClipboard() };
        if let Some(owner) = self.owner {
            unsafe { DestroyWindow(owner) };
        }
    }
}

pub struct WindowsClipboard {
    last_sequence: u32,
}

impl WindowsClipboard {
    pub fn new() -> Self {
        Self {
            last_sequence: unsafe { GetClipboardSequenceNumber() },
        }
    }

    pub fn changed(&mut self) -> io::Result<Option<ClipboardEvent>> {
        let sequence = unsafe { GetClipboardSequenceNumber() };
        if sequence == 0 || sequence == self.last_sequence {
            return Ok(None);
        }
        let Some(_open) = OpenedClipboard::open_for_read()? else {
            return Ok(None);
        };
        let event = if unsafe { IsClipboardFormatAvailable(CF_UNICODETEXT) } != 0 {
            let handle = unsafe { GetClipboardData(CF_UNICODETEXT) };
            if handle.is_null() {
                return Ok(None); // Delayed rendering may not be ready yet.
            }
            let size = unsafe { GlobalSize(handle) };
            if size == 0 || size > MAX_UTF16_BYTES || size % 2 != 0 {
                ClipboardEvent::Ignore
            } else {
                let pointer = unsafe { GlobalLock(handle) };
                if pointer.is_null() {
                    return Ok(None);
                }
                let units = unsafe { std::slice::from_raw_parts(pointer.cast::<u16>(), size / 2) };
                let end = units
                    .iter()
                    .position(|unit| *unit == 0)
                    .unwrap_or(units.len());
                let event = String::from_utf16(&units[..end])
                    .ok()
                    .map(|text| text.into_bytes())
                    .filter(|bytes| bytes.len() <= MAX_TEXT_BYTES)
                    .map_or(ClipboardEvent::Ignore, ClipboardEvent::Text);
                unsafe { GlobalUnlock(handle) };
                event
            }
        } else if unsafe { CountClipboardFormats() } == 0 {
            ClipboardEvent::Clear
        } else {
            ClipboardEvent::Ignore
        };
        self.last_sequence = sequence;
        Ok(Some(event))
    }

    pub fn apply(&mut self, event: &ClipboardEvent) -> io::Result<()> {
        if matches!(event, ClipboardEvent::Ignore) {
            return Ok(());
        }
        let wide = match event {
            ClipboardEvent::Text(bytes) => {
                if bytes.len() > MAX_TEXT_BYTES {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "clipboard too large",
                    ));
                }
                let text = std::str::from_utf8(bytes).map_err(|_| {
                    io::Error::new(io::ErrorKind::InvalidData, "clipboard is not UTF-8")
                })?;
                let mut wide: Vec<u16> = text.encode_utf16().collect();
                wide.push(0);
                Some(wide)
            }
            ClipboardEvent::Clear => None,
            ClipboardEvent::Ignore => unreachable!(),
        };
        let handle = if let Some(wide) = &wide {
            let size = wide.len() * size_of::<u16>();
            let handle = unsafe { GlobalAlloc(GMEM_MOVEABLE, size) };
            if handle.is_null() {
                return Err(last_error("GlobalAlloc failed"));
            }
            let pointer = unsafe { GlobalLock(handle) };
            if pointer.is_null() {
                unsafe { GlobalFree(handle) };
                return Err(last_error("GlobalLock failed"));
            }
            unsafe { ptr::copy_nonoverlapping(wide.as_ptr(), pointer.cast::<u16>(), wide.len()) };
            unsafe { GlobalUnlock(handle) };
            Some(handle)
        } else {
            None
        };
        let _open = match OpenedClipboard::open_for_write() {
            Ok(open) => open,
            Err(error) => {
                if let Some(handle) = handle {
                    unsafe { GlobalFree(handle) };
                }
                return Err(error);
            }
        };
        if unsafe { EmptyClipboard() } == 0 {
            if let Some(handle) = handle {
                unsafe { GlobalFree(handle) };
            }
            return Err(last_error("EmptyClipboard failed"));
        }
        if let Some(handle) = handle
            && unsafe { SetClipboardData(CF_UNICODETEXT, handle) }.is_null()
        {
            unsafe { GlobalFree(handle) };
            return Err(last_error("SetClipboardData failed"));
        }
        drop(_open);
        self.last_sequence = unsafe { GetClipboardSequenceNumber() };
        Ok(())
    }
}

impl Default for WindowsClipboard {
    fn default() -> Self {
        Self::new()
    }
}
