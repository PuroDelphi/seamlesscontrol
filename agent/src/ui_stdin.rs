//! Bounded stdin requests for UI operations that must not expose paths in argv.

use std::io::{self, BufRead, Read};

fn read_json_line(max_bytes: u64) -> io::Result<serde_json::Value> {
    let stdin = io::stdin();
    let mut input = Vec::new();
    stdin
        .lock()
        .take(max_bytes + 1)
        .read_until(b'\n', &mut input)?;
    if input.len() as u64 > max_bytes || !input.ends_with(b"\n") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "UI request is incomplete or too large",
        ));
    }
    serde_json::from_slice(&input)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid UI request"))
}

pub fn read_path() -> io::Result<String> {
    let value = read_json_line(16 * 1024)?;
    let path = value
        .as_str()
        .filter(|path| !path.is_empty() && path.len() <= 8192 && !path.contains('\0'))
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid UI path"))?;
    Ok(path.to_owned())
}

pub fn read_uris() -> io::Result<Vec<String>> {
    let value = read_json_line(1024 * 1024)?;
    let urls = value
        .as_array()
        .filter(|urls| !urls.is_empty() && urls.len() <= 256)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid UI file list"))?;
    urls.iter()
        .map(|item| {
            item.as_str()
                .filter(|url| !url.is_empty() && url.len() <= 8192 && !url.contains('\0'))
                .map(str::to_owned)
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid UI file URI"))
        })
        .collect()
}
