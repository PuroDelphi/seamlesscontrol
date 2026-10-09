//! A bounded, portable bundle carried by the existing encrypted file session.
//! It contains only regular files and directories; links and unsafe paths are rejected.

use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const MAGIC: &[u8; 4] = b"SCB1";
const MAX_ITEMS: usize = 256;
const MAX_PATH_BYTES: usize = 1024;
const MAX_MANIFEST_BYTES: usize = 512 * 1024;
const CHUNK: usize = 64 * 1024;
const SUFFIX: &str = " items.scbundle";

#[derive(Clone)]
struct Entry {
    path: String,
    source: PathBuf,
    size: u64,
    hash: [u8; 32],
    directory: bool,
}

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn regular_source(source: &Path) -> io::Result<File> {
    let before = fs::symlink_metadata(source)?;
    if !before.file_type().is_file() {
        return Err(invalid("bundle source is not a regular file"));
    }
    #[cfg(windows)]
    if std::os::windows::fs::MetadataExt::file_attributes(&before) & 0x400 != 0 {
        return Err(invalid("bundle source is a Windows reparse point"));
    }
    let file = File::open(source)?;
    let after = fs::symlink_metadata(source)?;
    if !after.file_type().is_file() || file.metadata()?.len() != before.len() {
        return Err(invalid("bundle source changed while opening"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if before.dev() != after.dev()
            || before.ino() != after.ino()
            || before.dev() != file.metadata()?.dev()
            || before.ino() != file.metadata()?.ino()
        {
            return Err(invalid("bundle source changed while opening"));
        }
    }
    #[cfg(windows)]
    if std::os::windows::fs::MetadataExt::file_attributes(&after) & 0x400 != 0 {
        return Err(invalid("bundle source became a Windows reparse point"));
    }
    Ok(file)
}

pub fn is_bundle_name(name: &str) -> Option<usize> {
    name.strip_prefix("SeamlessControl ")?
        .strip_suffix(SUFFIX)?
        .parse::<usize>()
        .ok()
        .filter(|count| (1..=MAX_ITEMS).contains(count))
}

fn validate_relative(path: &str) -> io::Result<()> {
    if path.is_empty()
        || path.len() > MAX_PATH_BYTES
        || path.starts_with('/')
        || path.contains('\\')
    {
        return Err(invalid("bundle path is not relative or is too long"));
    }
    for component in path.split('/') {
        if component.is_empty()
            || component == "."
            || component == ".."
            || component.ends_with([' ', '.'])
            || component
                .chars()
                .any(|c| c.is_control() || "<>:\"|?*".contains(c))
        {
            return Err(invalid("unsafe bundle path component"));
        }
        let stem = component
            .split('.')
            .next()
            .unwrap_or("")
            .to_ascii_uppercase();
        if matches!(
            stem.as_str(),
            "CON"
                | "PRN"
                | "AUX"
                | "NUL"
                | "COM1"
                | "COM2"
                | "COM3"
                | "COM4"
                | "COM5"
                | "COM6"
                | "COM7"
                | "COM8"
                | "COM9"
                | "LPT1"
                | "LPT2"
                | "LPT3"
                | "LPT4"
                | "LPT5"
                | "LPT6"
                | "LPT7"
                | "LPT8"
                | "LPT9"
        ) {
            return Err(invalid("reserved Windows bundle path"));
        }
    }
    Ok(())
}

fn walk(
    source: &Path,
    relative: &str,
    entries: &mut Vec<Entry>,
    total: &mut u64,
    limit: u64,
) -> io::Result<()> {
    validate_relative(relative)?;
    if entries.len() >= MAX_ITEMS {
        return Err(invalid("too many bundle items"));
    }
    let meta = fs::symlink_metadata(source)?;
    if meta.file_type().is_symlink() {
        return Err(invalid("bundle contains a symlink"));
    }
    if meta.is_dir() {
        entries.push(Entry {
            path: relative.to_owned(),
            source: source.to_owned(),
            size: 0,
            hash: [0; 32],
            directory: true,
        });
        let mut children = fs::read_dir(source)?
            .map(|item| item.map(|entry| entry.path()))
            .collect::<io::Result<Vec<_>>>()?;
        children.sort();
        for child in children {
            let name = child
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| invalid("bundle name is not UTF-8"))?;
            walk(&child, &format!("{relative}/{name}"), entries, total, limit)?;
        }
    } else if meta.is_file() {
        *total = total
            .checked_add(meta.len())
            .ok_or_else(|| invalid("bundle size overflow"))?;
        if *total > limit {
            return Err(invalid("bundle exceeds the configured limit"));
        }
        let mut input = regular_source(source)?;
        let mut hasher = Sha256::new();
        let copied = io::copy(&mut input, &mut hasher)?;
        if copied != meta.len() {
            return Err(invalid("file changed while preparing bundle"));
        }
        entries.push(Entry {
            path: relative.to_owned(),
            source: source.to_owned(),
            size: copied,
            hash: hasher.finalize().into(),
            directory: false,
        });
    } else {
        return Err(invalid("bundle contains a non-regular item"));
    }
    Ok(())
}

pub fn create_bundle(sources: &[PathBuf], limit: u64, staging: &Path) -> io::Result<PathBuf> {
    if sources.is_empty() || sources.len() > MAX_ITEMS {
        return Err(invalid("choose 1 to 256 files or folders"));
    }
    let mut entries = Vec::new();
    let mut total = 0;
    for source in sources {
        let name = source
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| invalid("bundle name is not UTF-8"))?;
        walk(source, name, &mut entries, &mut total, limit)?;
    }
    if entries.len() == 1 && !entries[0].directory {
        return Err(invalid("one regular file does not need a bundle"));
    }
    let mut seen = BTreeSet::new();
    let mut manifest_size = 6usize;
    for entry in &entries {
        if !seen.insert(entry.path.to_ascii_lowercase()) {
            return Err(invalid("duplicate bundle path"));
        }
        manifest_size = manifest_size
            .checked_add(43 + entry.path.len())
            .ok_or_else(|| invalid("bundle manifest overflow"))?;
    }
    if manifest_size > MAX_MANIFEST_BYTES || total.saturating_add(manifest_size as u64) > limit {
        return Err(invalid("bundle exceeds the configured limit"));
    }
    fs::create_dir_all(staging)?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(io::Error::other)?
        .as_nanos();
    // Use a per-copy subfolder to prevent collisions and keep the package private.
    let folder = staging.join(format!("bundle-{nonce}-{}", std::process::id()));
    fs::create_dir(&folder)?;
    let output = folder.join(format!("SeamlessControl {} items.scbundle", entries.len()));
    let result = (|| -> io::Result<()> {
        let mut writer = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&output)?;
        writer.write_all(MAGIC)?;
        writer.write_all(&(entries.len() as u16).to_be_bytes())?;
        for entry in &entries {
            writer.write_all(&[u8::from(entry.directory)])?;
            writer.write_all(&(entry.path.len() as u16).to_be_bytes())?;
            writer.write_all(entry.path.as_bytes())?;
            writer.write_all(&entry.size.to_be_bytes())?;
            writer.write_all(&entry.hash)?;
        }
        for entry in entries.iter().filter(|entry| !entry.directory) {
            let mut reader = regular_source(&entry.source)?;
            let mut hasher = Sha256::new();
            let mut copied = 0u64;
            let mut chunk = [0; CHUNK];
            loop {
                let count = reader.read(&mut chunk)?;
                if count == 0 {
                    break;
                }
                copied = copied
                    .checked_add(count as u64)
                    .ok_or_else(|| invalid("bundle size overflow"))?;
                if copied > entry.size {
                    return Err(invalid("file grew while packing bundle"));
                }
                writer.write_all(&chunk[..count])?;
                hasher.update(&chunk[..count]);
            }
            if copied != entry.size || hasher.finalize().as_slice() != entry.hash {
                return Err(invalid("file changed while packing bundle"));
            }
        }
        writer.sync_all()
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&folder);
    }
    result.map(|()| output)
}

pub fn extract_bundle(
    bundle: &Path,
    destination: &Path,
    expected_count: usize,
    limit: u64,
) -> io::Result<PathBuf> {
    let mut reader = File::open(bundle)?;
    if reader.metadata()?.len() > limit {
        return Err(invalid("bundle exceeds the configured limit"));
    }
    let mut magic = [0; 4];
    reader.read_exact(&mut magic)?;
    if &magic != MAGIC {
        return Err(invalid("invalid bundle magic"));
    }
    let mut two = [0; 2];
    reader.read_exact(&mut two)?;
    let count = u16::from_be_bytes(two) as usize;
    if count != expected_count || !(1..=MAX_ITEMS).contains(&count) {
        return Err(invalid("bundle item count changed"));
    }
    let mut entries = Vec::with_capacity(count);
    let mut seen = BTreeSet::new();
    let mut manifest_size = 6usize;
    let mut total = 0u64;
    for _ in 0..count {
        let mut kind = [0];
        reader.read_exact(&mut kind)?;
        if kind[0] > 1 {
            return Err(invalid("invalid bundle item kind"));
        }
        reader.read_exact(&mut two)?;
        let path_len = u16::from_be_bytes(two) as usize;
        if path_len == 0 || path_len > MAX_PATH_BYTES {
            return Err(invalid("invalid bundle path length"));
        }
        manifest_size = manifest_size
            .checked_add(43 + path_len)
            .ok_or_else(|| invalid("bundle manifest overflow"))?;
        if manifest_size > MAX_MANIFEST_BYTES {
            return Err(invalid("bundle manifest too large"));
        }
        let mut path_bytes = vec![0; path_len];
        reader.read_exact(&mut path_bytes)?;
        let path =
            String::from_utf8(path_bytes).map_err(|_| invalid("bundle path is not UTF-8"))?;
        validate_relative(&path)?;
        if !seen.insert(path.to_ascii_lowercase()) {
            return Err(invalid("duplicate bundle path"));
        }
        let mut eight = [0; 8];
        reader.read_exact(&mut eight)?;
        let size = u64::from_be_bytes(eight);
        let mut hash = [0; 32];
        reader.read_exact(&mut hash)?;
        if kind[0] == 1 && (size != 0 || hash != [0; 32]) {
            return Err(invalid("invalid directory entry"));
        }
        total = total
            .checked_add(size)
            .ok_or_else(|| invalid("bundle size overflow"))?;
        if total.saturating_add(manifest_size as u64) > limit {
            return Err(invalid("bundle exceeds the configured limit"));
        }
        entries.push((path, kind[0] == 1, size, hash));
    }
    let root = destination.join("Received group");
    fs::create_dir(&root)?;
    let result = (|| -> io::Result<()> {
        for (path, directory, size, hash) in &entries {
            let target = root.join(path);
            if *directory {
                fs::create_dir_all(&target)?;
                continue;
            }
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut output = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&target)?;
            let mut hasher = Sha256::new();
            let mut remaining = *size;
            let mut chunk = [0; CHUNK];
            while remaining > 0 {
                let read = remaining.min(CHUNK as u64) as usize;
                reader.read_exact(&mut chunk[..read])?;
                output.write_all(&chunk[..read])?;
                hasher.update(&chunk[..read]);
                remaining -= read as u64;
            }
            if hasher.finalize().as_slice() != hash {
                return Err(invalid("bundle item checksum mismatch"));
            }
            output.sync_all()?;
        }
        let mut tail = [0];
        if reader.read(&mut tail)? != 0 {
            return Err(invalid("bundle has trailing bytes"));
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&root);
    }
    result.map(|()| root)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_unsafe_paths() {
        for path in [
            "../bad",
            "C:/bad",
            "CON.txt",
            "/absolute",
            "a/../b",
            "a\\b",
            "a.",
        ] {
            assert!(validate_relative(path).is_err(), "{path}");
        }
        assert!(validate_relative("safe/folder/áé.txt").is_ok());
    }

    #[test]
    fn bundle_round_trip_and_checksum_failure() {
        let base = std::env::temp_dir().join(format!("sc-bundle-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let source = base.join("source");
        fs::create_dir_all(source.join("folder")).unwrap();
        fs::write(source.join("one.txt"), b"one").unwrap();
        fs::write(source.join("folder/two.txt"), b"two").unwrap();
        let bundle = create_bundle(
            &[source.join("one.txt"), source.join("folder")],
            1024 * 1024,
            &base.join("staging"),
        )
        .unwrap();
        let destination = base.join("destination");
        fs::create_dir(&destination).unwrap();
        let root = extract_bundle(&bundle, &destination, 3, 1024 * 1024).unwrap();
        assert_eq!(fs::read(root.join("one.txt")).unwrap(), b"one");
        assert_eq!(fs::read(root.join("folder/two.txt")).unwrap(), b"two");
        fs::remove_dir_all(root).unwrap();
        let mut bytes = fs::read(&bundle).unwrap();
        *bytes.last_mut().unwrap() ^= 1;
        fs::write(&bundle, bytes).unwrap();
        assert!(extract_bundle(&bundle, &destination, 3, 1024 * 1024).is_err());
        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn empty_folder_round_trip_and_limits() {
        let base = std::env::temp_dir().join(format!(
            "sc-empty-bundle-test-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&base);
        let empty = base.join("empty");
        fs::create_dir_all(&empty).unwrap();
        let staging = base.join("staging");
        let bundle = create_bundle(std::slice::from_ref(&empty), 1024, &staging).unwrap();
        assert_eq!(
            is_bundle_name(bundle.file_name().unwrap().to_str().unwrap()),
            Some(1)
        );
        let destination = base.join("destination");
        fs::create_dir(&destination).unwrap();
        let root = extract_bundle(&bundle, &destination, 1, 1024).unwrap();
        assert!(root.join("empty").is_dir());
        assert!(create_bundle(std::slice::from_ref(&empty), 4, &staging).is_err());
        assert!(extract_bundle(&bundle, &destination, 2, 1024).is_err());
        fs::remove_dir_all(base).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn rejects_links_and_case_collisions() {
        use std::os::unix::fs::symlink;
        let base = std::env::temp_dir().join(format!(
            "sc-link-bundle-test-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir(&base).unwrap();
        let first = base.join("one");
        fs::write(&first, b"data").unwrap();
        let link = base.join("link");
        symlink(&first, &link).unwrap();
        assert!(create_bundle(&[first.clone(), link], 1024, &base.join("staging")).is_err());
        let collision = base.join("ONE");
        fs::write(&collision, b"data").unwrap();
        assert!(create_bundle(&[first, collision], 1024, &base.join("staging")).is_err());
        fs::remove_dir_all(base).unwrap();
    }
}
