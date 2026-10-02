//! Local identity and trust persistence. Files are never silently recreated
//! after corruption: doing so would change the device identity unexpectedly.

use crate::secure::Identity;
use crate::topology::Topology;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

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
            let identity = Identity { private, public };
            identity.validate().map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidData, "identity keys do not match")
            })?;
            Ok(identity)
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

/// Replace an existing identity atomically. Other devices must explicitly
/// revoke the old public key and pair the new key before connecting again.
pub fn rotate_identity(path: &Path) -> io::Result<Identity> {
    fs::symlink_metadata(path)?;
    load_or_create_identity(path)?;
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("identity path has no parent"))?;
    ensure_private_directory(parent)?;
    let identity = Identity::generate().map_err(io::Error::other)?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(io::Error::other)?
        .as_nanos();
    let temp = parent.join(format!(".identity-{}-{nonce}.tmp", std::process::id()));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    let result = (|| {
        let mut file = options.open(&temp)?;
        file.write_all(IDENTITY_MAGIC)?;
        file.write_all(&identity.private)?;
        file.write_all(&identity.public)?;
        file.sync_all()?;
        fs::rename(&temp, path)?;
        if let Err(error) = File::open(parent).and_then(|directory| directory.sync_all()) {
            eprintln!(
                "SeamlessControl: rotated identity but could not sync its directory: {error}"
            );
        }
        Ok(identity)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
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

fn revoked_path(peers_dir: &Path, key: &[u8; 32]) -> io::Result<PathBuf> {
    let parent = peers_dir
        .parent()
        .ok_or_else(|| io::Error::other("peers directory has no parent"))?;
    Ok(parent.join("revoked").join(key_fingerprint(key)))
}

pub fn is_revoked(peers_dir: &Path, key: &[u8; 32]) -> io::Result<bool> {
    match fs::symlink_metadata(revoked_path(peers_dir, key)?) {
        Ok(metadata) => {
            if !metadata.is_file() || metadata.file_type().is_symlink() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "revocation marker is not a regular file",
                ));
            }
            #[cfg(unix)]
            if metadata.permissions().mode() & 0o077 != 0 {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "revocation marker is accessible by others",
                ));
            }
            Ok(true)
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

/// Revoke an IP's pinned identity. The key marker is durable before the pin
/// is removed, so interrupted revocation still blocks that key.
pub fn revoke_peer_key(peers_dir: &Path, address: IpAddr) -> io::Result<[u8; 32]> {
    let key = load_peer_key(peers_dir, address)?
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "peer is not paired"))?;
    let marker = revoked_path(peers_dir, &key)?;
    let dir = marker.parent().expect("revoked marker parent");
    ensure_private_directory(dir)?;
    if !is_revoked(peers_dir, &key)? {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        options.mode(0o600);
        let mut file = options.open(&marker)?;
        file.write_all(b"revoked\n")?;
        file.sync_all()?;
    }
    fs::remove_file(peer_path(peers_dir, address))?;
    Ok(key)
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

pub fn list_peer_keys(dir: &Path) -> io::Result<Vec<(IpAddr, [u8; 32])>> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let metadata = fs::symlink_metadata(dir)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "peers path is not a regular directory",
        ));
    }
    let mut peers = Vec::new();
    for entry in entries {
        let entry = entry?;
        let raw = entry.file_name().into_string().map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidData, "peer filename is not UTF-8")
        })?;
        let address: IpAddr = raw.replace('_', ":").parse().map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "peer filename is not an IP address",
            )
        })?;
        if peer_path(dir, address) != entry.path() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "peer filename is not canonical",
            ));
        }
        let key = load_peer_key(dir, address)?.ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "peer disappeared during listing")
        })?;
        peers.push((address, key));
    }
    peers.sort_by_key(|entry| entry.0.to_string());
    Ok(peers)
}

/// Move a trusted identity to a new address only after an authenticated
/// handshake proved possession of its pinned key. Keep the old pin until the
/// topology has been written, so interrupted updates do not lose trust.
pub fn relocate_peer_key(dir: &Path, old: IpAddr, new: IpAddr, key: &[u8; 32]) -> io::Result<()> {
    if old == new {
        return remember_peer_key(dir, new, key);
    }
    if load_peer_key(dir, old)? != Some(*key) || is_revoked(dir, key)? {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "old peer pin is absent or revoked",
        ));
    }
    if load_peer_key(dir, new)?.is_some() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "new address already has a peer pin",
        ));
    }
    let topology_path = dir
        .parent()
        .ok_or_else(|| io::Error::other("peers directory has no parent"))?
        .join("topology");
    let mut topology = load_topology(&topology_path)?;
    let changed = topology
        .rename_peer(old, new)
        .map_err(|error| io::Error::new(io::ErrorKind::AlreadyExists, error))?;
    remember_peer_key(dir, new, key)?;
    if changed && let Err(error) = save_topology(&topology_path, &topology) {
        let _ = fs::remove_file(peer_path(dir, new));
        return Err(error);
    }
    fs::remove_file(peer_path(dir, old))?;
    #[cfg(unix)]
    File::open(dir)?.sync_all()?;
    Ok(())
}

pub fn remember_peer_key(dir: &Path, address: IpAddr, key: &[u8; 32]) -> io::Result<()> {
    if is_revoked(dir, key)? {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "peer identity has been revoked",
        ));
    }
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

pub fn load_topology(path: &Path) -> io::Result<Topology> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if !metadata.is_file() || metadata.file_type().is_symlink() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "topology path is not a regular file",
                ));
            }
            #[cfg(unix)]
            if metadata.permissions().mode() & 0o077 != 0 {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "topology file is accessible by others",
                ));
            }
            let mut raw = String::new();
            File::open(path)?.take(513).read_to_string(&mut raw)?;
            Topology::decode(&raw).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Topology::new()),
        Err(error) => Err(error),
    }
}

pub fn save_topology(path: &Path, topology: &Topology) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("topology path has no parent"))?;
    ensure_private_directory(parent)?;
    match fs::symlink_metadata(path) {
        Ok(metadata) if !metadata.is_file() || metadata.file_type().is_symlink() => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "topology path is not a regular file",
            ));
        }
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(io::Error::other)?
        .as_nanos();
    let temp = parent.join(format!(".topology-{}-{nonce}.tmp", std::process::id()));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    let result = (|| {
        let mut file = options.open(&temp)?;
        file.write_all(topology.encode().as_bytes())?;
        file.sync_all()?;
        fs::rename(&temp, path)?;
        #[cfg(unix)]
        File::open(parent)?.sync_all()?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topology::{Machine, Slot};
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
    fn rotation_replaces_valid_identity_but_never_repairs_mismatched_keys() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "seamlesscontrol-rotation-{}-{unique}",
            std::process::id()
        ));
        let path = dir.join("identity");
        assert_eq!(
            rotate_identity(&path).unwrap_err().kind(),
            io::ErrorKind::NotFound
        );
        let original = load_or_create_identity(&path).unwrap();
        let rotated = rotate_identity(&path).unwrap();
        assert_ne!(original.public, rotated.public);
        assert_eq!(load_or_create_identity(&path).unwrap(), rotated);
        #[cfg(unix)]
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);

        let mut bytes = fs::read(&path).unwrap();
        bytes[40] ^= 1;
        fs::write(&path, &bytes).unwrap();
        assert_eq!(
            load_or_create_identity(&path).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        assert_eq!(
            rotate_identity(&path).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        assert_eq!(fs::read(&path).unwrap(), bytes);
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

    #[test]
    fn authenticated_peer_address_change_preserves_slot_and_rejects_conflicts() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "seamlesscontrol-roaming-{}-{unique}",
            std::process::id()
        ));
        let peers = root.join("peers");
        let old: IpAddr = "192.168.50.10".parse().unwrap();
        let new: IpAddr = "192.168.50.20".parse().unwrap();
        let occupied: IpAddr = "192.168.50.30".parse().unwrap();
        let key = [7; 32];
        remember_peer_key(&peers, old, &key).unwrap();
        remember_peer_key(&peers, occupied, &[8; 32]).unwrap();
        let mut layout = Topology::new();
        layout
            .place(Machine::Peer(old), Slot::new(1, 0).unwrap())
            .unwrap();
        save_topology(&root.join("topology"), &layout).unwrap();

        assert_eq!(
            relocate_peer_key(&peers, old, occupied, &key)
                .unwrap_err()
                .kind(),
            io::ErrorKind::AlreadyExists
        );
        assert_eq!(load_peer_key(&peers, old).unwrap(), Some(key));
        assert_eq!(load_peer_key(&peers, occupied).unwrap(), Some([8; 32]));
        assert_eq!(load_topology(&root.join("topology")).unwrap(), layout);

        assert_eq!(
            relocate_peer_key(&peers, old, new, &[9; 32])
                .unwrap_err()
                .kind(),
            io::ErrorKind::PermissionDenied
        );
        assert_eq!(load_peer_key(&peers, new).unwrap(), None);

        relocate_peer_key(&peers, old, new, &key).unwrap();
        assert_eq!(load_peer_key(&peers, old).unwrap(), None);
        assert_eq!(load_peer_key(&peers, new).unwrap(), Some(key));
        let changed = load_topology(&root.join("topology")).unwrap();
        assert_eq!(changed.edge_to(new).unwrap(), crate::topology::Edge::Right);
        assert!(changed.edge_to(old).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn revoked_identity_cannot_be_pinned_again() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "seamlesscontrol-revoke-{}-{unique}",
            std::process::id()
        ));
        let peers = root.join("peers");
        let address: IpAddr = "127.0.0.1".parse().unwrap();
        let key = [9; 32];
        remember_peer_key(&peers, address, &key).unwrap();
        assert_eq!(revoke_peer_key(&peers, address).unwrap(), key);
        assert!(is_revoked(&peers, &key).unwrap());
        assert_eq!(load_peer_key(&peers, address).unwrap(), None);
        assert_eq!(
            remember_peer_key(&peers, address, &key).unwrap_err().kind(),
            io::ErrorKind::PermissionDenied
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn peer_listing_returns_pinned_ip_and_key() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "seamlesscontrol-list-{}-{unique}",
            std::process::id()
        ));
        let peers = root.join("peers");
        assert!(list_peer_keys(&peers).unwrap().is_empty());
        let address: IpAddr = "192.168.50.4".parse().unwrap();
        remember_peer_key(&peers, address, &[4; 32]).unwrap();
        assert_eq!(list_peer_keys(&peers).unwrap(), vec![(address, [4; 32])]);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn topology_persists_atomically_and_rejects_corruption() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "seamlesscontrol-topology-{}-{unique}",
            std::process::id()
        ));
        let path = root.join("topology");
        let mut layout = load_topology(&path).unwrap();
        let peer: IpAddr = "192.168.50.7".parse().unwrap();
        layout
            .place(
                crate::topology::Machine::Peer(peer),
                crate::topology::Slot::new(1, 0).unwrap(),
            )
            .unwrap();
        save_topology(&path, &layout).unwrap();
        assert_eq!(load_topology(&path).unwrap(), layout);
        #[cfg(unix)]
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        fs::write(&path, "bad").unwrap();
        assert_eq!(
            load_topology(&path).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        fs::remove_dir_all(root).unwrap();
    }
}
