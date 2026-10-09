//! Local file clipboard helpers. Network bytes still use the authenticated file session.

use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const RETENTION: Duration = Duration::from_secs(7 * 24 * 60 * 60);

pub fn local_regular_file(path: &Path, limit: u64) -> io::Result<bool> {
    let metadata = std::fs::symlink_metadata(path)?;
    Ok(metadata.file_type().is_file() && metadata.len() <= limit)
}

pub fn staging_dir(base: &Path) -> io::Result<PathBuf> {
    let path = base.join("clipboard-files");
    std::fs::create_dir_all(&path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700))?;
    }
    prune_old_sessions(&path);
    Ok(path)
}

pub fn staging_session_dir(root: &Path) -> io::Result<PathBuf> {
    for attempt in 0..100u32 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_nanos();
        let path = root.join(format!("offer-{now}-{}-{attempt}", std::process::id()));
        match std::fs::create_dir(&path) {
            Ok(()) => {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700))?;
                }
                return Ok(path);
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "staging folder collision",
    ))
}

pub fn staging_can_fit(root: &Path, next_size: u64, file_limit: u64) -> io::Result<bool> {
    let quota = file_limit.saturating_mul(4).max(1024 * 1024 * 1024);
    let mut used = 0u64;
    for session in std::fs::read_dir(root)? {
        let session = session?;
        if !session.file_type()?.is_dir() || !managed_session(&session) {
            continue;
        }
        let mut pending = vec![session.path()];
        while let Some(folder) = pending.pop() {
            for item in std::fs::read_dir(folder)? {
                let item = item?;
                let kind = item.file_type()?;
                if kind.is_dir() {
                    pending.push(item.path());
                } else if kind.is_file() {
                    used = used.saturating_add(item.metadata()?.len());
                }
            }
        }
    }
    Ok(used.saturating_add(next_size) <= quota)
}

fn managed_session(entry: &std::fs::DirEntry) -> bool {
    let name = entry.file_name();
    let name = name.to_string_lossy();
    name.starts_with("offer-") || name.starts_with("bundle-")
}

fn prune_old_sessions(root: &Path) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if !kind.is_dir() || !managed_session(&entry) {
            continue;
        }
        let stale = entry
            .metadata()
            .ok()
            .and_then(|meta| meta.modified().ok())
            .and_then(|modified| modified.elapsed().ok())
            .is_some_and(|age| age > RETENTION);
        if stale {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
}

#[cfg(target_os = "linux")]
pub fn parse_one_file_uri(bytes: &[u8]) -> io::Result<Option<PathBuf>> {
    use std::os::unix::ffi::OsStringExt;

    let text = std::str::from_utf8(bytes)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid URI list"))?;
    let entries: Vec<_> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();
    if entries.len() != 1 {
        return Ok(None);
    }
    let Some(encoded) = entries[0]
        .strip_prefix("file://localhost")
        .or_else(|| entries[0].strip_prefix("file://"))
    else {
        return Ok(None);
    };
    if !encoded.starts_with('/') || encoded.contains(['?', '#']) {
        return Ok(None);
    }
    let mut decoded = Vec::with_capacity(encoded.len());
    let mut source = encoded.as_bytes().iter().copied();
    while let Some(byte) = source.next() {
        if byte == b'%' {
            let digit = |value: u8| match value {
                b'0'..=b'9' => Some(value - b'0'),
                b'A'..=b'F' => Some(value - b'A' + 10),
                b'a'..=b'f' => Some(value - b'a' + 10),
                _ => None,
            };
            let high = source
                .next()
                .and_then(digit)
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid URI escape"))?;
            let low = source
                .next()
                .and_then(digit)
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid URI escape"))?;
            decoded.push((high << 4) | low);
        } else {
            decoded.push(byte);
        }
    }
    if decoded.contains(&0) {
        return Ok(None);
    }
    Ok(Some(PathBuf::from(std::ffi::OsString::from_vec(decoded))))
}

#[cfg(target_os = "linux")]
pub fn parse_file_uris(bytes: &[u8]) -> io::Result<Option<Vec<PathBuf>>> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid URI list"))?;
    let lines: Vec<_> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();
    if lines.is_empty() || lines.len() > 256 {
        return Ok(None);
    }
    let mut paths = Vec::with_capacity(lines.len());
    for line in lines {
        let Some(path) = parse_one_file_uri(line.as_bytes())? else {
            return Ok(None);
        };
        paths.push(path);
    }
    Ok(Some(paths))
}

#[cfg(target_os = "linux")]
pub fn file_uri(path: &Path) -> io::Result<Vec<u8>> {
    use std::os::unix::ffi::OsStrExt;

    if !path.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "file path is relative",
        ));
    }
    let mut uri = b"file://".to_vec();
    for byte in path.as_os_str().as_bytes() {
        if byte.is_ascii_alphanumeric() || b"/-._~".contains(byte) {
            uri.push(*byte);
        } else {
            uri.extend_from_slice(format!("%{byte:02X}").as_bytes());
        }
    }
    uri.extend_from_slice(b"\r\n");
    Ok(uri)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn staging_keeps_repeated_filenames_separate_and_respects_quota() {
        let base = std::env::temp_dir().join(format!(
            "seamlesscontrol-clipboard-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let root = staging_dir(&base).unwrap();
        let first = staging_session_dir(&root).unwrap();
        let second = staging_session_dir(&root).unwrap();
        assert_ne!(first, second);
        std::fs::write(first.join("same.txt"), b"one").unwrap();
        std::fs::write(second.join("same.txt"), b"two").unwrap();
        assert!(staging_can_fit(&root, 32, 1024).unwrap());
        assert!(!staging_can_fit(&root, 1024 * 1024 * 1024, 1024).unwrap());
        let bundle = root.join("bundle-test").join("nested");
        std::fs::create_dir_all(&bundle).unwrap();
        std::fs::File::create(bundle.join("sparse"))
            .unwrap()
            .set_len(1024 * 1024 * 1024)
            .unwrap();
        assert!(!staging_can_fit(&root, 1, 1024).unwrap());
        std::fs::remove_dir_all(base).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn copied_file_rejects_symlink() {
        let base =
            std::env::temp_dir().join(format!("seamlesscontrol-link-test-{}", std::process::id()));
        std::fs::create_dir_all(&base).unwrap();
        let target = base.join("target");
        let link = base.join("link");
        std::fs::write(&target, b"data").unwrap();
        std::os::unix::fs::symlink(&target, &link).unwrap();
        assert!(local_regular_file(&target, 4).unwrap());
        assert!(!local_regular_file(&link, 4).unwrap());
        std::fs::remove_dir_all(base).unwrap();
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn uri_list_accepts_one_local_file_and_round_trips_unicode() {
        let file = Path::new("/tmp/archivo ñ.txt");
        let uri = file_uri(file).unwrap();
        assert_eq!(parse_one_file_uri(&uri).unwrap().as_deref(), Some(file));
        assert_eq!(parse_one_file_uri(b"file://other/tmp/a\r\n").unwrap(), None);
        assert_eq!(
            parse_one_file_uri(b"file:///tmp/a\r\nfile:///tmp/b\r\n").unwrap(),
            None
        );
        assert_eq!(parse_one_file_uri(b"file:///tmp/%00bad\r\n").unwrap(), None);
        assert_eq!(
            parse_file_uris(b"file:///tmp/a\r\nfile:///tmp/b\r\n")
                .unwrap()
                .unwrap()
                .len(),
            2
        );
    }
}
