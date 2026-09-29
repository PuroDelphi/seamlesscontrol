//! Local identity and trust persistence. Files are never silently recreated
//! after corruption: doing so would change the device identity unexpectedly.

use crate::secure::Identity;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::Path;

const IDENTITY_MAGIC: &[u8; 8] = b"SCID0001";
const IDENTITY_SIZE: usize = 8 + 32 + 32;

#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

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
                fs::create_dir_all(parent)?;
                #[cfg(unix)]
                fs::set_permissions(parent, fs::Permissions::from_mode(0o700))?;
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
}
