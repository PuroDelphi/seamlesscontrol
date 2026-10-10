//! Local choices for a pinned peer identity. Addresses may change through DHCP;
//! permissions are therefore indexed by the public key, not the address.

use crate::storage::key_fingerprint;
use serde_json::Value;
use std::fs::{self, OpenOptions};
use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Capability {
    Control,
    Text,
    Files,
}

impl Capability {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "control" => Some(Self::Control),
            "text" => Some(Self::Text),
            "files" => Some(Self::Files),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PeerPolicy {
    pub control: bool,
    pub text: bool,
    pub files: bool,
    pub last_connected_ms: u64,
}

impl Default for PeerPolicy {
    fn default() -> Self {
        Self {
            control: true,
            text: true,
            files: true,
            last_connected_ms: 0,
        }
    }
}

fn policy_path(config: &Path, key: &[u8; 32]) -> PathBuf {
    config
        .join("peer-policy")
        .join(format!("{}.json", key_fingerprint(key)))
}

impl PeerPolicy {
    pub fn load(config: &Path, key: &[u8; 32]) -> io::Result<Self> {
        let path = policy_path(config, key);
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(error) => return Err(error),
        };
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "peer policy is not a regular file",
            ));
        }
        let bytes = fs::read(path)?;
        let value: Value = serde_json::from_slice(&bytes).map_err(io::Error::other)?;
        let boolean = |name: &str| {
            value.get(name).and_then(Value::as_bool).ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "invalid peer permission")
            })
        };
        Ok(Self {
            control: boolean("control")?,
            text: boolean("text")?,
            files: boolean("files")?,
            last_connected_ms: value
                .get("lastConnectedMs")
                .and_then(Value::as_u64)
                .unwrap_or(0),
        })
    }

    pub fn save(&self, config: &Path, key: &[u8; 32]) -> io::Result<()> {
        let path = policy_path(config, key);
        let parent = path.parent().expect("policy path has parent");
        fs::create_dir_all(parent)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(parent, fs::Permissions::from_mode(0o700))?;
        }
        let data = serde_json::to_vec(&serde_json::json!({
            "control": self.control, "text": self.text, "files": self.files,
            "lastConnectedMs": self.last_connected_ms,
        }))
        .map_err(io::Error::other)?;
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_nanos();
        let temporary = path.with_extension(format!("{}-{nonce}.tmp", std::process::id()));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&temporary)?;
        file.write_all(&data)?;
        file.sync_all()?;
        drop(file);
        fs::rename(temporary, path)
    }

    pub fn permits(&self, capability: Capability) -> bool {
        match capability {
            Capability::Control => self.control,
            Capability::Text => self.text,
            Capability::Files => self.files,
        }
    }

    pub fn set(&mut self, capability: Capability, allowed: bool) {
        match capability {
            Capability::Control => self.control = allowed,
            Capability::Text => self.text = allowed,
            Capability::Files => self.files = allowed,
        }
    }

    pub fn mark_connected(&mut self) -> io::Result<()> {
        self.last_connected_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_millis() as u64;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn choices_follow_key_even_if_address_changes() {
        let dir = std::env::temp_dir().join(format!("sc-peer-policy-{}", std::process::id()));
        let key = [7; 32];
        let other = [8; 32];
        let mut policy = PeerPolicy::load(&dir, &key).unwrap();
        assert!(policy.permits(Capability::Files));
        policy.set(Capability::Files, false);
        policy.save(&dir, &key).unwrap();
        assert!(!PeerPolicy::load(&dir, &key).unwrap().files);
        assert!(PeerPolicy::load(&dir, &other).unwrap().files);
        fs::remove_dir_all(dir).unwrap();
    }
}
