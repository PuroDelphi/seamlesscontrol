//! Small local control socket for the Omarchy widget. Only the session user
//! can reach it: the runtime directory is private and the socket is 0600.

use crate::secure::PeerInfo;
use crate::storage::{key_fingerprint, revoke_peer_key};
use std::fs;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::IpAddr;
use std::os::unix::fs::{FileTypeExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct ControlStatus {
    pub role: &'static str,
    pub phase: String,
    pub peer: String,
    pub paused: bool,
    pub pair_sas: String,
    pub pair_key: String,
    revoked_active: bool,
    decision: Option<bool>,
}

#[derive(Clone)]
pub struct ControlHandle(Arc<(Mutex<ControlStatus>, Condvar)>);

impl ControlHandle {
    pub fn set_phase(&self, phase: &str) {
        self.0.0.lock().expect("control state lock").phase = phase.to_owned();
    }

    pub fn phase(&self) -> String {
        self.0.0.lock().expect("control state lock").phase.clone()
    }

    pub fn set_peer(&self, peer: &str) {
        let mut state = self.0.0.lock().expect("control state lock");
        state.peer = peer.to_owned();
        state.revoked_active = false;
    }

    pub fn paused(&self) -> bool {
        self.0.0.lock().expect("control state lock").paused
    }

    pub fn revoked_active(&self) -> bool {
        self.0.0.lock().expect("control state lock").revoked_active
    }

    pub fn cancel_pair(&self) {
        let (lock, wake) = &*self.0;
        let mut state = lock.lock().expect("control state lock");
        if state.phase == "pairing" {
            state.decision = Some(false);
            wake.notify_all();
        }
    }

    pub fn confirm_pair(&self, peer: &PeerInfo) -> bool {
        let (lock, wake) = &*self.0;
        let mut state = lock.lock().expect("control state lock");
        let prior = state.phase.clone();
        state.phase = "pairing".to_owned();
        state.pair_sas = peer.sas.clone();
        state.pair_key = key_fingerprint(&peer.public_key);
        state.decision = None;
        println!(
            "Compare el código {} en ambos equipos y apruebe desde Omarchy o con: seamlesscontrold approve {}",
            peer.sas, peer.sas
        );
        let result = wake.wait_timeout_while(state, Duration::from_secs(300), |state| {
            state.decision.is_none()
        });
        let Ok((mut state, _)) = result else {
            return false;
        };
        let accepted = state.decision == Some(true);
        state.phase = prior;
        state.pair_sas.clear();
        state.pair_key.clear();
        state.decision = None;
        accepted
    }
}

pub struct ControlServer {
    path: PathBuf,
    running: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
    handle: ControlHandle,
}

pub fn socket_path() -> io::Result<PathBuf> {
    let runtime = std::env::var_os("XDG_RUNTIME_DIR")
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "XDG_RUNTIME_DIR is not set"))?;
    Ok(PathBuf::from(runtime)
        .join("seamlesscontrol")
        .join("control.sock"))
}

impl ControlServer {
    pub fn start(role: &'static str, config: &Path) -> io::Result<Self> {
        Self::start_at(socket_path()?, role, config)
    }

    pub fn start_at(path: PathBuf, role: &'static str, config: &Path) -> io::Result<Self> {
        let dir = path
            .parent()
            .ok_or_else(|| io::Error::other("socket needs a parent"))?;
        fs::create_dir_all(dir)?;
        let directory = fs::symlink_metadata(dir)?;
        if !directory.is_dir() || directory.file_type().is_symlink() {
            return Err(io::Error::other(
                "control directory must be a real directory",
            ));
        }
        fs::set_permissions(dir, fs::Permissions::from_mode(0o700))?;
        match fs::symlink_metadata(&path) {
            Ok(metadata) => {
                if !metadata.file_type().is_socket() {
                    return Err(io::Error::other("refusing to replace a non-socket path"));
                }
                if UnixStream::connect(&path).is_ok() {
                    return Err(io::Error::new(
                        io::ErrorKind::AddrInUse,
                        "agent already running",
                    ));
                }
                fs::remove_file(&path)?;
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        let listener = UnixListener::bind(&path)?;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
        listener.set_nonblocking(true)?;
        let running = Arc::new(AtomicBool::new(true));
        let handle = ControlHandle(Arc::new((
            Mutex::new(ControlStatus {
                role,
                phase: if role == "serve" {
                    "listening"
                } else {
                    "connecting"
                }
                .to_owned(),
                peer: String::new(),
                paused: false,
                pair_sas: String::new(),
                pair_key: String::new(),
                revoked_active: false,
                decision: None,
            }),
            Condvar::new(),
        )));
        let worker_running = Arc::clone(&running);
        let worker_handle = handle.clone();
        let peers_dir = config.join("peers");
        let worker = thread::spawn(move || {
            while worker_running.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let _ = stream.set_read_timeout(Some(Duration::from_secs(1)));
                        let mut command = String::new();
                        let read = BufReader::new((&mut stream).take(64)).read_line(&mut command);
                        let response = if read.is_ok() {
                            let (lock, wake) = &*worker_handle.0;
                            let mut status = lock.lock().expect("control state lock");
                            match command.trim() {
                                "status" => format!(
                                    "STATUS\t{}\t{}\t{}\t{}\t{}\t{}\n",
                                    status.role,
                                    status.phase,
                                    status.peer,
                                    status.paused,
                                    status.pair_sas,
                                    status.pair_key
                                ),
                                "pause" if status.role == "connect" => {
                                    status.paused = true;
                                    "OK\n".to_owned()
                                }
                                "resume" if status.role == "connect" => {
                                    status.paused = false;
                                    "OK\n".to_owned()
                                }
                                value
                                    if value.starts_with("approve ")
                                        && status.phase == "pairing" =>
                                {
                                    if value.strip_prefix("approve ")
                                        == Some(status.pair_sas.as_str())
                                    {
                                        status.decision = Some(true);
                                        wake.notify_all();
                                        "OK\n".to_owned()
                                    } else {
                                        "ERR\tcode mismatch\n".to_owned()
                                    }
                                }
                                "reject" if status.phase == "pairing" => {
                                    status.decision = Some(false);
                                    wake.notify_all();
                                    "OK\n".to_owned()
                                }
                                value if value.starts_with("revoke ") => {
                                    match value.strip_prefix("revoke ").unwrap().parse::<IpAddr>() {
                                        Ok(address) => match revoke_peer_key(&peers_dir, address) {
                                            Ok(_) => {
                                                if status.peer == address.to_string() {
                                                    status.revoked_active = true;
                                                }
                                                "OK\n".to_owned()
                                            }
                                            Err(error) => format!("ERR\t{error}\n"),
                                        },
                                        Err(_) => "ERR\tinvalid IP address\n".to_owned(),
                                    }
                                }
                                _ => "ERR\tunsupported command\n".to_owned(),
                            }
                        } else {
                            "ERR\tinvalid command\n".to_owned()
                        };
                        let _ = stream.write_all(response.as_bytes());
                    }
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(50));
                    }
                    Err(_) => break,
                }
            }
        });
        Ok(Self {
            path,
            running,
            worker: Some(worker),
            handle,
        })
    }

    pub fn handle(&self) -> ControlHandle {
        self.handle.clone()
    }
}

impl Drop for ControlServer {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        let _ = fs::remove_file(&self.path);
    }
}

pub fn request(command: &str) -> io::Result<String> {
    request_at(&socket_path()?, command)
}

pub fn request_at(path: &Path, command: &str) -> io::Result<String> {
    let mut stream = UnixStream::connect(path)?;
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    stream.write_all(command.as_bytes())?;
    stream.write_all(b"\n")?;
    let mut response = String::new();
    BufReader::new(stream.take(512)).read_line(&mut response)?;
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_socket_reports_and_pauses_sender() {
        let dir =
            std::env::temp_dir().join(format!("seamlesscontrol-control-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let path = dir.join("control.sock");
        let server = ControlServer::start_at(path.clone(), "connect", &dir).unwrap();
        assert!(
            request_at(&path, "status")
                .unwrap()
                .contains("STATUS\tconnect\tconnecting")
        );
        assert_eq!(request_at(&path, "pause").unwrap(), "OK\n");
        assert!(server.handle().paused());
        assert_eq!(request_at(&path, "resume").unwrap(), "OK\n");
        assert!(!server.handle().paused());
        drop(server);
        assert!(!path.exists());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn pairing_requires_the_displayed_code() {
        let dir = std::env::temp_dir().join(format!(
            "seamlesscontrol-pair-control-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        let path = dir.join("control.sock");
        let server = ControlServer::start_at(path.clone(), "serve", &dir).unwrap();
        let handle = server.handle();
        let pairing = thread::spawn(move || {
            handle.confirm_pair(&PeerInfo {
                public_key: [7; 32],
                sas: "007321".to_owned(),
            })
        });
        for _ in 0..100 {
            if request_at(&path, "status").unwrap().contains("\tpairing\t") {
                break;
            }
            thread::sleep(Duration::from_millis(5));
        }
        assert!(request_at(&path, "status").unwrap().contains("\t007321\t"));
        assert!(
            request_at(&path, "approve 999999")
                .unwrap()
                .starts_with("ERR")
        );
        assert_eq!(request_at(&path, "approve 007321").unwrap(), "OK\n");
        assert!(pairing.join().unwrap());
        assert!(!request_at(&path, "status").unwrap().contains("\t007321\t"));
        drop(server);
        fs::remove_dir_all(dir).unwrap();
    }
}
