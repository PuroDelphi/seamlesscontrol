//! Local identity and trust persistence. Files are never silently recreated
//! after corruption: doing so would change the device identity unexpectedly.

use crate::secure::Identity;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::net::IpAddr;
use std::path::Path;

const IDENTITY_MAGIC: &[u8; 8] = b"SCID0001";
const IDENTITY_SIZE: usize = 8 + 32 + 32;

#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

fn ensure_private_directory(path: &Path) -> io::Result<()> {
    fs::create_dir_all(path)?;
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "configuration directory is not a regular directory",
        ));
    }
    #[cfg(unix)]
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    Ok(())
}

pub fn load_or_create_identity(path: &Path) -> io::Result<Identity> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if !metadata.is_file() || metadata.file_type().is_symlink() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "identity path is not a regular file",
                ));
            }
            #[cfg(unix)]
            if metadata.permissions().mode() & 0o077 != 0 {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "identity file is accessible by others",
                ));
            }
            let file = File::open(path)?;
            let mut bytes = Vec::new();
            file.take((IDENTITY_SIZE + 1) as u64)
                .read_to_end(&mut bytes)?;
            if bytes.len() != IDENTITY_SIZE || &bytes[..8] != IDENTITY_MAGIC {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "identity file is malformed",
                ));
            }
            let private = bytes[8..40].try_into().expect("fixed identity length");
            let public = bytes[40..72].try_into().expect("fixed identity length");
            Ok(Identity { private, public })
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            if let Some(parent) = path.parent() {
                ensure_private_directory(parent)?;
            }
            let identity = Identity::generate().map_err(io::Error::other)?;
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            options.mode(0o600);
            let mut file = options.open(path)?;
            file.write_all(IDENTITY_MAGIC)?;
            file.write_all(&identity.private)?;
            file.write_all(&identity.public)?;
            file.sync_all()?;
            Ok(identity)
        }
        Err(error) => Err(error),
    }
}

pub fn key_fingerprint(key: &[u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(64);
    for byte in key {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 15) as usize] as char);
    }
    out
}

fn peer_path(dir: &Path, address: IpAddr) -> std::path::PathBuf {
    dir.join(address.to_string().replace(':', "_"))
}

pub fn load_peer_key(dir: &Path, address: IpAddr) -> io::Result<Option<[u8; 32]>> {
    let path = peer_path(dir, address);
    match fs::symlink_metadata(&path) {
        Ok(metadata) => {
            if !metadata.is_file() || metadata.file_type().is_symlink() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "peer key path is not a regular file",
                ));
            }
            #[cfg(unix)]
            if metadata.permissions().mode() & 0o077 != 0 {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "peer key file is accessible by others",
                ));
            }
            let mut raw = String::new();
            File::open(path)?.take(66).read_to_string(&mut raw)?;
            let raw = raw.trim_end_matches('\n');
            if raw.len() != 64 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "peer key length is invalid",
                ));
            }
            let mut key = [0; 32];
            for (index, chunk) in raw.as_bytes().as_chunks::<2>().0.iter().enumerate() {
                let digit = |byte: u8| -> io::Result<u8> {
                    match byte {
                        b'0'..=b'9' => Ok(byte - b'0'),
                        b'a'..=b'f' => Ok(byte - b'a' + 10),
                        _ => Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "peer key is not lowercase hex",
                        )),
                    }
                };
                key[index] = digit(chunk[0])? << 4 | digit(chunk[1])?;
            }
            Ok(Some(key))
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

pub fn remember_peer_key(dir: &Path, address: IpAddr, key: &[u8; 32]) -> io::Result<()> {
    if let Some(existing) = load_peer_key(dir, address)? {
        return if existing == *key {
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "peer key changed; revoke the old pairing explicitly",
            ))
        };
    }
    ensure_private_directory(dir)?;
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options.open(peer_path(dir, address))?;
    file.write_all(key_fingerprint(key).as_bytes())?;
    file.write_all(b"\n")?;
    file.sync_all()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn identity_persists_and_malformed_file_is_not_replaced() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("seamlesscontrol-{}-{unique}", std::process::id()));
        let path = dir.join("identity");
        let original = load_or_create_identity(&path).unwrap();
        assert_eq!(load_or_create_identity(&path).unwrap(), original);
        #[cfg(unix)]
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        fs::write(&path, b"broken").unwrap();
        assert_eq!(
            load_or_create_identity(&path).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn peer_key_is_pinned_and_cannot_be_overwritten() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "seamlesscontrol-peers-{}-{unique}",
            std::process::id()
        ));
        let address: IpAddr = "127.0.0.1".parse().unwrap();
        let key = [7; 32];
        remember_peer_key(&dir, address, &key).unwrap();
        assert_eq!(load_peer_key(&dir, address).unwrap(), Some(key));
        assert_eq!(
            remember_peer_key(&dir, address, &[8; 32])
                .unwrap_err()
                .kind(),
            io::ErrorKind::PermissionDenied
        );
        fs::remove_dir_all(dir).unwrap();
    }
}
