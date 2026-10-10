//! Short-lived local proof that an authenticated control channel is open.
//! Pairing alone never authorizes clipboard-file or manual file transfers.

use crate::storage::key_fingerprint;
use std::fs;
use std::io;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const REFRESH: Duration = Duration::from_millis(250);
const MAX_AGE_MS: u128 = 750;

fn now_ms() -> io::Result<u128> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(io::Error::other)?
        .as_millis())
}

fn peer_dir(config: &Path, key: &[u8; 32]) -> PathBuf {
    config.join("active-sessions").join(key_fingerprint(key))
}

pub struct ActiveSession {
    path: PathBuf,
    alternate: PathBuf,
    running: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl ActiveSession {
    pub fn start(config: &Path, peer: IpAddr, key: &[u8; 32]) -> io::Result<Self> {
        let dir = peer_dir(config, key);
        fs::create_dir_all(&dir)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(
                config.join("active-sessions"),
                fs::Permissions::from_mode(0o700),
            )?;
            fs::set_permissions(&dir, fs::Permissions::from_mode(0o700))?;
        }
        let path = dir.join(format!("{}-{}.lease", std::process::id(), now_ms()?));
        let alternate = path.with_extension("backup-lease");
        let write = move |path: &Path| fs::write(path, format!("{peer}\n{}\n", now_ms()?));
        write(&path)?;
        write(&alternate)?;
        let running = Arc::new(AtomicBool::new(true));
        let flag = Arc::clone(&running);
        let target = path.clone();
        let other = alternate.clone();
        let worker = thread::spawn(move || {
            let mut toggle = false;
            while flag.load(Ordering::Acquire) {
                thread::sleep(REFRESH);
                if !flag.load(Ordering::Acquire) {
                    break;
                }
                toggle = !toggle;
                if write(if toggle { &target } else { &other }).is_err() {
                    break;
                }
            }
        });
        Ok(Self {
            path,
            alternate,
            running,
            worker: Some(worker),
        })
    }
}

impl Drop for ActiveSession {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Release);
        let _ = fs::remove_file(&self.path);
        let _ = fs::remove_file(&self.alternate);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        let _ = fs::remove_file(&self.path);
        let _ = fs::remove_file(&self.alternate);
    }
}

pub fn is_active(config: &Path, peer: IpAddr, key: &[u8; 32]) -> io::Result<bool> {
    let dir = peer_dir(config, key);
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error),
    };
    let now = now_ms()?;
    for entry in entries {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let Ok(data) = fs::read_to_string(entry.path()) else {
            continue;
        };
        let mut lines = data.lines();
        let ip_ok = lines.next() == Some(peer.to_string().as_str());
        let timestamp = lines.next().and_then(|line| line.parse::<u128>().ok());
        if timestamp.is_some_and(|stamp| stamp <= now && now - stamp > 30_000) {
            let _ = fs::remove_file(entry.path());
        }
        if ip_ok && timestamp.is_some_and(|stamp| stamp <= now && now - stamp <= MAX_AGE_MS) {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pairing_without_session_does_not_authorize_files() {
        let dir = std::env::temp_dir().join(format!(
            "sc-active-{}-{}",
            std::process::id(),
            now_ms().unwrap()
        ));
        let key = [7; 32];
        let ip = "192.0.2.10".parse().unwrap();
        assert!(!is_active(&dir, ip, &key).unwrap());
        let session = ActiveSession::start(&dir, ip, &key).unwrap();
        assert!(is_active(&dir, ip, &key).unwrap());
        assert!(!is_active(&dir, "192.0.2.11".parse().unwrap(), &key).unwrap());
        assert!(!is_active(&dir, ip, &[8; 32]).unwrap());
        drop(session);
        assert!(!is_active(&dir, ip, &key).unwrap());
        let _ = fs::remove_dir_all(dir);
    }
}
