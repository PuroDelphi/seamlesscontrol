//! Bounded file messages and verified, atomic publication of received files.
//! The caller must obtain destination consent before constructing FileReceiver.

use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

pub const DEFAULT_MAX_FILE_BYTES: u64 = 100 * 1024 * 1024;
pub const MAX_CHUNK_BYTES: usize = 64 * 1024;
const MAX_NAME_BYTES: usize = 255;
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileOffer {
    pub name: String,
    pub size: u64,
    pub sha256: [u8; 32],
}

impl FileOffer {
    pub fn from_path(path: &Path, limit: u64) -> io::Result<Self> {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| invalid("file name is not UTF-8"))?
            .to_owned();
        validate_name(&name)?;
        let file = File::open(path)?;
        let metadata = file.metadata()?;
        if !metadata.is_file() || metadata.len() > limit {
            return Err(invalid(
                "file is not regular or exceeds the configured limit",
            ));
        }
        let mut hasher = Sha256::new();
        let copied = io::copy(&mut file.take(limit.saturating_add(1)), &mut hasher)?;
        if copied > limit {
            return Err(invalid("file exceeds the configured limit"));
        }
        if copied != metadata.len() {
            return Err(invalid("file changed while calculating its hash"));
        }
        Ok(Self {
            name,
            size: copied,
            sha256: hasher.finalize().into(),
        })
    }

    pub fn validate(&self, limit: u64) -> io::Result<()> {
        validate_name(&self.name)?;
        if self.size > limit {
            return Err(invalid("file exceeds the configured limit"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FileMessage {
    Offer(FileOffer),
    Accept,
    Reject,
    Chunk(Vec<u8>),
    End,
    Complete,
    Cancel,
}

impl FileMessage {
    pub fn encode(&self) -> Vec<u8> {
        match self {
            Self::Offer(offer) => {
                let mut out = Vec::with_capacity(43 + offer.name.len());
                out.push(1);
                out.extend_from_slice(&offer.size.to_be_bytes());
                out.extend_from_slice(&offer.sha256);
                out.extend_from_slice(&(offer.name.len() as u16).to_be_bytes());
                out.extend_from_slice(offer.name.as_bytes());
                out
            }
            Self::Accept => vec![2],
            Self::Reject => vec![3],
            Self::Chunk(data) => {
                let mut out = Vec::with_capacity(data.len() + 1);
                out.push(4);
                out.extend_from_slice(data);
                out
            }
            Self::End => vec![5],
            Self::Complete => vec![6],
            Self::Cancel => vec![7],
        }
    }

    pub fn decode(payload: &[u8]) -> io::Result<Self> {
        match payload {
            [1, rest @ ..] if rest.len() >= 42 => {
                let size = u64::from_be_bytes(rest[0..8].try_into().expect("checked length"));
                let sha256 = rest[8..40].try_into().expect("checked length");
                let name_len =
                    u16::from_be_bytes(rest[40..42].try_into().expect("checked length")) as usize;
                if name_len == 0 || name_len > MAX_NAME_BYTES || rest.len() != 42 + name_len {
                    return Err(invalid("invalid file offer length"));
                }
                let name = std::str::from_utf8(&rest[42..])
                    .map_err(|_| invalid("file name is not UTF-8"))?
                    .to_owned();
                validate_name(&name)?;
                Ok(Self::Offer(FileOffer { name, size, sha256 }))
            }
            [2] => Ok(Self::Accept),
            [3] => Ok(Self::Reject),
            [4, data @ ..] if !data.is_empty() && data.len() <= MAX_CHUNK_BYTES => {
                Ok(Self::Chunk(data.to_vec()))
            }
            [5] => Ok(Self::End),
            [6] => Ok(Self::Complete),
            [7] => Ok(Self::Cancel),
            _ => Err(invalid("invalid file message")),
        }
    }
}

pub struct FileSender {
    file: File,
    pub offer: FileOffer,
    sent: u64,
}

impl FileSender {
    pub fn open(path: &Path, limit: u64) -> io::Result<Self> {
        let offer = FileOffer::from_path(path, limit)?;
        let file = File::open(path)?;
        Ok(Self {
            file,
            offer,
            sent: 0,
        })
    }

    pub fn next_chunk(&mut self) -> io::Result<Option<Vec<u8>>> {
        if self.sent == self.offer.size {
            return Ok(None);
        }
        let remaining = (self.offer.size - self.sent).min(MAX_CHUNK_BYTES as u64) as usize;
        let mut data = vec![0; remaining];
        self.file.read_exact(&mut data)?;
        self.sent += data.len() as u64;
        Ok(Some(data))
    }
}

pub struct FileReceiver {
    offer: FileOffer,
    temp_path: PathBuf,
    target_path: PathBuf,
    file: File,
    received: u64,
    hasher: Sha256,
    published: bool,
}

impl FileReceiver {
    /// Call only after the local user accepted this offer and selected `directory`.
    pub fn accept(offer: FileOffer, directory: &Path, limit: u64) -> io::Result<Self> {
        offer.validate(limit)?;
        let target_path = directory.join(&offer.name);
        for _ in 0..32 {
            let suffix = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
            let temp_path = directory.join(format!(
                ".seamlesscontrol-{}-{suffix}.part",
                std::process::id()
            ));
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let result = options.open(&temp_path);
            match result {
                Ok(file) => {
                    return Ok(Self {
                        offer,
                        temp_path,
                        target_path,
                        file,
                        received: 0,
                        hasher: Sha256::new(),
                        published: false,
                    });
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        }
        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "cannot allocate temporary file",
        ))
    }

    pub fn write_chunk(&mut self, data: &[u8]) -> io::Result<()> {
        if data.is_empty()
            || data.len() > MAX_CHUNK_BYTES
            || self
                .received
                .checked_add(data.len() as u64)
                .is_none_or(|size| size > self.offer.size)
        {
            return Err(invalid("file chunk exceeds declared size"));
        }
        self.file.write_all(data)?;
        self.hasher.update(data);
        self.received += data.len() as u64;
        Ok(())
    }

    pub fn finish(mut self) -> io::Result<PathBuf> {
        if self.received != self.offer.size
            || self.hasher.clone().finalize().as_slice() != self.offer.sha256
        {
            return Err(invalid(
                "received file size or SHA-256 does not match offer",
            ));
        }
        self.file.flush()?;
        self.file.sync_all()?;
        // The temporary file is in the same directory; hard_link will fail if the
        // final name already exists, so an existing download is never replaced.
        fs::hard_link(&self.temp_path, &self.target_path)?;
        self.published = true;
        fs::remove_file(&self.temp_path)?;
        Ok(self.target_path.clone())
    }
}

impl Drop for FileReceiver {
    fn drop(&mut self) {
        if !self.published {
            let _ = fs::remove_file(&self.temp_path);
        }
    }
}

fn validate_name(name: &str) -> io::Result<()> {
    if name.is_empty()
        || name.len() > MAX_NAME_BYTES
        || name == "."
        || name == ".."
        || name
            .chars()
            .any(|ch| ch == '/' || ch == '\\' || ch.is_control())
    {
        return Err(invalid("invalid file name"));
    }
    Ok(())
}

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(label: &str) -> PathBuf {
        let suffix = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "seamlesscontrol-file-{label}-{}-{suffix}",
            std::process::id()
        ));
        fs::create_dir(&dir).unwrap();
        dir
    }

    #[test]
    fn accepted_file_is_published_only_after_hash_and_size_match() {
        let dir = scratch("roundtrip");
        let source = dir.join("source.txt");
        let destination = dir.join("downloads");
        fs::create_dir(&destination).unwrap();
        let contents = vec![b'x'; MAX_CHUNK_BYTES * 2 + 9];
        fs::write(&source, &contents).unwrap();
        let mut sender = FileSender::open(&source, DEFAULT_MAX_FILE_BYTES).unwrap();
        let wire = FileMessage::Offer(sender.offer.clone()).encode();
        let FileMessage::Offer(offer) = FileMessage::decode(&wire).unwrap() else {
            panic!("offer")
        };
        let mut receiver =
            FileReceiver::accept(offer, &destination, DEFAULT_MAX_FILE_BYTES).unwrap();
        while let Some(chunk) = sender.next_chunk().unwrap() {
            let wire = FileMessage::Chunk(chunk).encode();
            let FileMessage::Chunk(chunk) = FileMessage::decode(&wire).unwrap() else {
                panic!("chunk")
            };
            receiver.write_chunk(&chunk).unwrap();
        }
        assert!(!destination.join("source.txt").exists());
        let saved = receiver.finish().unwrap();
        assert_eq!(fs::read(saved).unwrap(), contents);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn reject_interruption_bad_hash_and_collision_never_replace_a_file() {
        let dir = scratch("failure");
        let offer = FileOffer {
            name: "safe.txt".to_owned(),
            size: 4,
            sha256: Sha256::digest(b"good").into(),
        };
        assert_eq!(
            FileMessage::decode(&FileMessage::Reject.encode()).unwrap(),
            FileMessage::Reject
        );
        {
            let mut receiver = FileReceiver::accept(offer.clone(), &dir, 4).unwrap();
            receiver.write_chunk(b"go").unwrap();
        }
        assert!(!dir.join("safe.txt").exists());
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 0);
        let mut receiver = FileReceiver::accept(offer.clone(), &dir, 4).unwrap();
        receiver.write_chunk(b"evil").unwrap();
        assert!(receiver.finish().is_err());
        assert!(!dir.join("safe.txt").exists());
        fs::write(dir.join("safe.txt"), b"original").unwrap();
        let mut receiver = FileReceiver::accept(offer, &dir, 4).unwrap();
        receiver.write_chunk(b"good").unwrap();
        assert!(receiver.finish().is_err());
        assert_eq!(fs::read(dir.join("safe.txt")).unwrap(), b"original");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn untrusted_offer_and_chunks_are_bounded() {
        for name in ["../escape", "a/b", "a\\b", "", ".", "bad\nname"] {
            assert!(
                FileOffer {
                    name: name.to_owned(),
                    size: 0,
                    sha256: [0; 32]
                }
                .validate(1)
                .is_err()
            );
        }
        let offer = FileOffer {
            name: "ok".to_owned(),
            size: 2,
            sha256: [0; 32],
        };
        assert!(offer.validate(1).is_err());
        assert!(FileMessage::decode(&[4]).is_err());
        assert!(
            FileMessage::decode(&FileMessage::Chunk(vec![0; MAX_CHUNK_BYTES + 1]).encode())
                .is_err()
        );
        let dir = scratch("bounds");
        let mut receiver = FileReceiver::accept(offer, &dir, 2).unwrap();
        assert!(receiver.write_chunk(b"three").is_err());
        drop(receiver);
        fs::remove_dir_all(dir).unwrap();
    }
}
