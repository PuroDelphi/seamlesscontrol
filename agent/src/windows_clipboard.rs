//! Windows text/PNG clipboard adapter and local file clipboard bridge.

use crate::clipboard::{ClipboardEvent, MAX_IMAGE_BYTES, MAX_TEXT_BYTES, valid_png_header};
use crate::clipboard_file::local_regular_file;
use std::io;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};
use std::ptr;
use std::thread;
use std::time::Duration;
use windows_sys::Win32::Foundation::GlobalFree;
use windows_sys::Win32::System::DataExchange::{
    CloseClipboard, CountClipboardFormats, EmptyClipboard, GetClipboardData,
    GetClipboardSequenceNumber, IsClipboardFormatAvailable, OpenClipboard,
    RegisterClipboardFormatW, SetClipboardData,
};
use windows_sys::Win32::System::Memory::{
    GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock,
};
use windows_sys::Win32::UI::Shell::{DROPFILES, DragQueryFileW};
use windows_sys::Win32::UI::WindowsAndMessaging::{CreateWindowExW, DestroyWindow};

const CF_UNICODETEXT: u32 = 13;
const CF_HDROP: u32 = 15;
const CF_DIB: u32 = 8;
const CF_DIBV5: u32 = 17;
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
        let png_format = registered_png_format();
        let event = if unsafe { IsClipboardFormatAvailable(CF_HDROP) } != 0 {
            ClipboardEvent::Ignore
        } else if png_format != 0 && unsafe { IsClipboardFormatAvailable(png_format) } != 0 {
            match read_global_data(png_format, MAX_IMAGE_BYTES)? {
                Some(bytes) if valid_png_header(&bytes) => ClipboardEvent::ImagePng(bytes),
                Some(_) => ClipboardEvent::Ignore,
                None => return Ok(None),
            }
        } else if unsafe { IsClipboardFormatAvailable(CF_DIBV5) } != 0
            || unsafe { IsClipboardFormatAvailable(CF_DIB) } != 0
        {
            let format = if unsafe { IsClipboardFormatAvailable(CF_DIBV5) } != 0 {
                CF_DIBV5
            } else {
                CF_DIB
            };
            match read_global_data(format, MAX_IMAGE_BYTES * 8)? {
                Some(dib) => {
                    dib_to_png(&dib).map_or(ClipboardEvent::Ignore, ClipboardEvent::ImagePng)
                }
                None => return Ok(None),
            }
        } else if unsafe { IsClipboardFormatAvailable(CF_UNICODETEXT) } != 0 {
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

    /// None means unchanged; Some(None) means a changed clipboard without local paths.
    pub fn copied_paths(
        &mut self,
        limit: u64,
        staging: &Path,
    ) -> io::Result<Option<Option<Vec<PathBuf>>>> {
        let sequence = unsafe { GetClipboardSequenceNumber() };
        if sequence == 0 || sequence == self.last_sequence {
            return Ok(None);
        }
        let Some(_open) = OpenedClipboard::open_for_read()? else {
            return Ok(None);
        };
        if unsafe { IsClipboardFormatAvailable(CF_HDROP) } == 0 {
            self.last_sequence = sequence;
            return Ok(Some(None));
        }
        let clipboard_files = unsafe { GetClipboardData(CF_HDROP) };
        if clipboard_files.is_null() {
            return Ok(None);
        }
        let count = unsafe { DragQueryFileW(clipboard_files, u32::MAX, ptr::null_mut(), 0) };
        if count == 0 || count > 256 {
            self.last_sequence = sequence;
            return Ok(Some(None));
        }
        let mut paths = Vec::with_capacity(count as usize);
        for index in 0..count {
            let length =
                unsafe { DragQueryFileW(clipboard_files, index, ptr::null_mut(), 0) } as usize;
            if length == 0 || length > 32767 {
                self.last_sequence = sequence;
                return Ok(Some(None));
            }
            let mut wide = vec![0u16; length + 1];
            if unsafe {
                DragQueryFileW(clipboard_files, index, wide.as_mut_ptr(), wide.len() as u32)
            } as usize
                != length
            {
                return Ok(None);
            }
            paths.push(PathBuf::from(std::ffi::OsString::from_wide(
                &wide[..length],
            )));
        }
        drop(_open);
        self.last_sequence = sequence;
        if paths.iter().any(|path| {
            path.to_string_lossy()
                .to_lowercase()
                .starts_with(&staging.to_string_lossy().to_lowercase())
        }) {
            return Ok(Some(None));
        }
        if paths.len() == 1
            && paths[0].is_file()
            && !local_regular_file(&paths[0], limit).unwrap_or(false)
        {
            return Ok(Some(None));
        }
        Ok(Some(Some(paths)))
    }

    pub fn publish_file(&mut self, path: &Path) -> io::Result<()> {
        let metadata = std::fs::symlink_metadata(path)?;
        if !path.is_absolute()
            || metadata.file_type().is_symlink()
            || !(metadata.is_dir() || local_regular_file(path, u64::MAX)?)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid local file",
            ));
        }
        let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
        wide.extend_from_slice(&[0, 0]);
        let offset = size_of::<DROPFILES>();
        let bytes = offset + wide.len() * size_of::<u16>();
        let handle = unsafe { GlobalAlloc(GMEM_MOVEABLE, bytes) };
        if handle.is_null() {
            return Err(last_error("GlobalAlloc failed"));
        }
        let pointer = unsafe { GlobalLock(handle) };
        if pointer.is_null() {
            unsafe { GlobalFree(handle) };
            return Err(last_error("GlobalLock failed"));
        }
        let header = DROPFILES {
            pFiles: offset as u32,
            pt: windows_sys::Win32::Foundation::POINT { x: 0, y: 0 },
            fNC: 0,
            fWide: 1,
        };
        unsafe {
            ptr::write_unaligned(pointer.cast::<DROPFILES>(), header);
            ptr::copy_nonoverlapping(
                wide.as_ptr(),
                pointer.cast::<u8>().add(offset).cast::<u16>(),
                wide.len(),
            );
            GlobalUnlock(handle);
        }
        let _open = match OpenedClipboard::open_for_write() {
            Ok(open) => open,
            Err(error) => {
                unsafe { GlobalFree(handle) };
                return Err(error);
            }
        };
        if unsafe { EmptyClipboard() } == 0 {
            unsafe { GlobalFree(handle) };
            return Err(last_error("EmptyClipboard failed"));
        }
        if unsafe { SetClipboardData(CF_HDROP, handle) }.is_null() {
            unsafe { GlobalFree(handle) };
            return Err(last_error("SetClipboardData failed"));
        }
        self.last_sequence = unsafe { GetClipboardSequenceNumber() };
        Ok(())
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
            ClipboardEvent::ImagePng(bytes) => {
                if !valid_png_header(bytes) {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "invalid clipboard PNG",
                    ));
                }
                None
            }
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
        let image_handle = if let ClipboardEvent::ImagePng(bytes) = event {
            let handle = unsafe { GlobalAlloc(GMEM_MOVEABLE, bytes.len()) };
            if handle.is_null() {
                return Err(last_error("image GlobalAlloc failed"));
            }
            let pointer = unsafe { GlobalLock(handle) };
            if pointer.is_null() {
                unsafe { GlobalFree(handle) };
                return Err(last_error("image GlobalLock failed"));
            }
            unsafe {
                ptr::copy_nonoverlapping(bytes.as_ptr(), pointer.cast::<u8>(), bytes.len());
                GlobalUnlock(handle);
            }
            Some(handle)
        } else {
            None
        };
        let dib_handle = if let ClipboardEvent::ImagePng(bytes) = event {
            match png_to_dibv5(bytes) {
                Some(dib) => {
                    let handle = unsafe { GlobalAlloc(GMEM_MOVEABLE, dib.len()) };
                    if handle.is_null() {
                        if let Some(image_handle) = image_handle {
                            unsafe { GlobalFree(image_handle) };
                        }
                        return Err(last_error("DIB GlobalAlloc failed"));
                    }
                    let pointer = unsafe { GlobalLock(handle) };
                    if pointer.is_null() {
                        unsafe { GlobalFree(handle) };
                        if let Some(image_handle) = image_handle {
                            unsafe { GlobalFree(image_handle) };
                        }
                        return Err(last_error("DIB GlobalLock failed"));
                    }
                    unsafe {
                        ptr::copy_nonoverlapping(dib.as_ptr(), pointer.cast::<u8>(), dib.len());
                        GlobalUnlock(handle);
                    }
                    Some(handle)
                }
                None => None,
            }
        } else {
            None
        };
        let _open = match OpenedClipboard::open_for_write() {
            Ok(open) => open,
            Err(error) => {
                if let Some(handle) = handle {
                    unsafe { GlobalFree(handle) };
                }
                if let Some(handle) = image_handle {
                    unsafe { GlobalFree(handle) };
                }
                if let Some(handle) = dib_handle {
                    unsafe { GlobalFree(handle) };
                }
                return Err(error);
            }
        };
        if unsafe { EmptyClipboard() } == 0 {
            if let Some(handle) = handle {
                unsafe { GlobalFree(handle) };
            }
            if let Some(handle) = image_handle {
                unsafe { GlobalFree(handle) };
            }
            if let Some(handle) = dib_handle {
                unsafe { GlobalFree(handle) };
            }
            return Err(last_error("EmptyClipboard failed"));
        }
        if let Some(image_handle) = image_handle {
            let format = registered_png_format();
            if format == 0 || unsafe { SetClipboardData(format, image_handle) }.is_null() {
                unsafe { GlobalFree(image_handle) };
                if let Some(dib_handle) = dib_handle {
                    unsafe { GlobalFree(dib_handle) };
                }
                return Err(last_error("SetClipboardData PNG failed"));
            }
        }
        if let Some(dib_handle) = dib_handle
            && unsafe { SetClipboardData(CF_DIBV5, dib_handle) }.is_null()
        {
            unsafe { GlobalFree(dib_handle) };
            return Err(last_error("SetClipboardData DIBV5 failed"));
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

fn registered_png_format() -> u32 {
    let name: Vec<u16> = "PNG\0".encode_utf16().collect();
    unsafe { RegisterClipboardFormatW(name.as_ptr()) }
}

fn read_global_data(format: u32, limit: usize) -> io::Result<Option<Vec<u8>>> {
    let handle = unsafe { GetClipboardData(format) };
    if handle.is_null() {
        return Ok(None);
    }
    let size = unsafe { GlobalSize(handle) };
    if size == 0 || size > limit {
        return Ok(Some(Vec::new()));
    }
    let pointer = unsafe { GlobalLock(handle) };
    if pointer.is_null() {
        return Ok(None);
    }
    let bytes = unsafe { std::slice::from_raw_parts(pointer.cast::<u8>(), size).to_vec() };
    unsafe { GlobalUnlock(handle) };
    Ok(Some(bytes))
}

fn dib_to_png(dib: &[u8]) -> Option<Vec<u8>> {
    let le_u16 = |start: usize| {
        Some(u16::from_le_bytes(
            dib.get(start..start + 2)?.try_into().ok()?,
        ))
    };
    let le_u32 = |start: usize| {
        Some(u32::from_le_bytes(
            dib.get(start..start + 4)?.try_into().ok()?,
        ))
    };
    let header = le_u32(0)? as usize;
    if header < 40 || header > dib.len() || le_u16(12)? != 1 {
        return None;
    }
    let width = i32::from_le_bytes(dib.get(4..8)?.try_into().ok()?);
    let height = i32::from_le_bytes(dib.get(8..12)?.try_into().ok()?);
    let bpp = le_u16(14)?;
    let compression = le_u32(16)?;
    if width <= 0
        || height == 0
        || height == i32::MIN
        || !(bpp == 24 || bpp == 32)
        || !(compression == 0 || (compression == 3 && bpp == 32 && header >= 56))
    {
        return None;
    }
    let width = width as usize;
    let rows = height.unsigned_abs() as usize;
    if width > 16384 || rows > 16384 || width.checked_mul(rows)? > 64_000_000 {
        return None;
    }
    if compression == 3
        && (le_u32(40)?, le_u32(44)?, le_u32(48)?) != (0x00ff0000, 0x0000ff00, 0x000000ff)
    {
        return None;
    }
    let pixel_offset = header.checked_add(if compression == 3 && header == 40 {
        12
    } else {
        0
    })?;
    let stride = width
        .checked_mul(bpp as usize)?
        .checked_add(31)?
        .checked_div(32)?
        .checked_mul(4)?;
    let pixels = dib.get(pixel_offset..pixel_offset.checked_add(stride.checked_mul(rows)?)?)?;
    let mut rgba = vec![0u8; width.checked_mul(rows)?.checked_mul(4)?];
    let has_alpha = bpp == 32
        && (0..rows).any(|y| {
            pixels[y * stride + 3..y * stride + width * 4]
                .chunks_exact(4)
                .any(|p| p[0] != 0)
        });
    for y in 0..rows {
        let src_y = if height > 0 { rows - 1 - y } else { y };
        for x in 0..width {
            let source = src_y * stride + x * (bpp as usize / 8);
            let dest = (y * width + x) * 4;
            rgba[dest..dest + 4].copy_from_slice(&[
                pixels[source + 2],
                pixels[source + 1],
                pixels[source],
                if has_alpha { pixels[source + 3] } else { 255 },
            ]);
        }
    }
    let mut output = Vec::new();
    let mut encoder = png::Encoder::new(&mut output, width as u32, rows as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header().ok()?.write_image_data(&rgba).ok()?;
    valid_png_header(&output).then_some(output)
}

fn png_to_dibv5(bytes: &[u8]) -> Option<Vec<u8>> {
    if !valid_png_header(bytes) {
        return None;
    }
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().ok()?;
    let size = reader.output_buffer_size()?;
    if size > 64 * 1024 * 1024 {
        return None;
    }
    let mut decoded = vec![0; size];
    let info = reader.next_frame(&mut decoded).ok()?;
    let width = info.width as usize;
    let height = info.height as usize;
    let pixels = width.checked_mul(height)?;
    if pixels == 0 || pixels > 16_000_000 {
        return None;
    }
    let channels = match info.color_type {
        png::ColorType::Grayscale => 1,
        png::ColorType::GrayscaleAlpha => 2,
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        png::ColorType::Indexed => return None,
    };
    let source = decoded.get(..pixels.checked_mul(channels)?)?;
    let mut dib = vec![0u8; 124 + pixels.checked_mul(4)?];
    dib[0..4].copy_from_slice(&124u32.to_le_bytes());
    dib[4..8].copy_from_slice(&info.width.to_le_bytes());
    dib[8..12].copy_from_slice(&(-(info.height as i32)).to_le_bytes());
    dib[12..14].copy_from_slice(&1u16.to_le_bytes());
    dib[14..16].copy_from_slice(&32u16.to_le_bytes());
    dib[16..20].copy_from_slice(&3u32.to_le_bytes());
    dib[20..24].copy_from_slice(&((pixels * 4) as u32).to_le_bytes());
    for (start, mask) in [
        (40, 0x00ff0000u32),
        (44, 0x0000ff00),
        (48, 0x000000ff),
        (52, 0xff000000),
    ] {
        dib[start..start + 4].copy_from_slice(&mask.to_le_bytes());
    }
    dib[56..60].copy_from_slice(&0x73524742u32.to_le_bytes()); // sRGB
    for (index, pixel) in source.chunks_exact(channels).enumerate() {
        let (r, g, b, a) = match pixel {
            [gray] => (*gray, *gray, *gray, 255),
            [gray, alpha] => (*gray, *gray, *gray, *alpha),
            [red, green, blue] => (*red, *green, *blue, 255),
            [red, green, blue, alpha] => (*red, *green, *blue, *alpha),
            _ => return None,
        };
        dib[124 + index * 4..124 + index * 4 + 4].copy_from_slice(&[b, g, r, a]);
    }
    Some(dib)
}

impl Default for WindowsClipboard {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod image_tests {
    use super::*;

    #[test]
    fn windows_bitmap_converts_to_png_and_back_without_swapping_channels() {
        let mut dib = vec![0u8; 48];
        dib[0..4].copy_from_slice(&40u32.to_le_bytes());
        dib[4..8].copy_from_slice(&2i32.to_le_bytes());
        dib[8..12].copy_from_slice(&1i32.to_le_bytes());
        dib[12..14].copy_from_slice(&1u16.to_le_bytes());
        dib[14..16].copy_from_slice(&24u16.to_le_bytes());
        dib[40..48].copy_from_slice(&[10, 20, 30, 40, 50, 60, 0, 0]);
        let png = dib_to_png(&dib).expect("supported DIB");
        assert!(valid_png_header(&png));
        let round_trip = png_to_dibv5(&png).expect("supported PNG");
        assert_eq!(&round_trip[8..12], &(-1i32).to_le_bytes());
        assert_eq!(&round_trip[124..132], &[10, 20, 30, 255, 40, 50, 60, 255]);
    }
}
