//! Omarchy clipboard adapter using wl-clipboard's selection notifications.

use crate::clipboard::{ClipboardEvent, ClipboardPacket, ClipboardSync, MAX_TEXT_BYTES};
use crate::clipboard_file::{file_uri, local_regular_file, parse_one_file_uri};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

pub fn emit_watched_event() -> io::Result<()> {
    let event = match std::env::var("CLIPBOARD_STATE").as_deref() {
        Ok("data") => {
            let mut bytes = Vec::new();
            io::stdin()
                .take((MAX_TEXT_BYTES + 1) as u64)
                .read_to_end(&mut bytes)?;
            if bytes.len() > MAX_TEXT_BYTES
                || std::str::from_utf8(&bytes).is_err()
                || has_file_uri_type()
            {
                ClipboardEvent::Ignore
            } else {
                ClipboardEvent::Text(bytes)
            }
        }
        Ok("clear") => ClipboardEvent::Clear,
        _ => ClipboardEvent::Ignore,
    };
    let mut output = io::stdout().lock();
    event.write_framed(&mut output)?;
    output.flush()
}

fn has_file_uri_type() -> bool {
    Command::new("wl-paste")
        .arg("--list-types")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .is_some_and(|output| {
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .any(|line| line.trim() == "text/uri-list")
        })
}

pub fn copied_file(limit: u64, staging: &Path) -> io::Result<Option<PathBuf>> {
    if !has_file_uri_type() {
        return Ok(None);
    }
    let output = Command::new("wl-paste")
        .args(["--no-newline", "--type", "text/uri-list"])
        .output()?;
    if !output.status.success() || output.stdout.len() > 64 * 1024 {
        return Ok(None);
    }
    let Some(path) = parse_one_file_uri(&output.stdout)? else {
        return Ok(None);
    };
    if path.starts_with(staging) || !local_regular_file(&path, limit).unwrap_or(false) {
        return Ok(None);
    }
    Ok(Some(path))
}

pub fn publish_file(path: &Path) -> io::Result<()> {
    let uri = file_uri(path)?;
    let mut child = Command::new("wl-copy")
        .args(["--type", "text/uri-list"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    child
        .stdin
        .take()
        .ok_or_else(|| io::Error::other("wl-copy stdin missing"))?
        .write_all(&uri)?;
    wait_child(child)
}

fn wait_child(mut child: Child) -> io::Result<()> {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if let Some(status) = child.try_wait()? {
            return if status.success() {
                Ok(())
            } else {
                Err(io::Error::other(format!("wl-copy exited with {status}")))
            };
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(io::Error::new(io::ErrorKind::TimedOut, "wl-copy timed out"));
        }
        thread::sleep(Duration::from_millis(20));
    }
}

pub fn apply(event: &ClipboardEvent) -> io::Result<()> {
    match event {
        ClipboardEvent::Text(bytes) => {
            if bytes.len() > MAX_TEXT_BYTES || std::str::from_utf8(bytes).is_err() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "invalid clipboard text",
                ));
            }
            let mut child = Command::new("wl-copy")
                .args(["--type", "text/plain"])
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()?;
            child
                .stdin
                .take()
                .ok_or_else(|| io::Error::other("wl-copy stdin missing"))?
                .write_all(bytes)?;
            wait_child(child)
        }
        ClipboardEvent::Clear => wait_child(
            Command::new("wl-copy")
                .arg("--clear")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()?,
        ),
        ClipboardEvent::Ignore => Ok(()),
    }
}

pub fn spawn_apply_worker(
    sync: Arc<Mutex<ClipboardSync>>,
) -> (mpsc::Sender<ClipboardPacket>, JoinHandle<()>) {
    let (sender, mut receiver) = mpsc::channel::<ClipboardPacket>(8);
    let worker = thread::spawn(move || {
        while let Some(packet) = receiver.blocking_recv() {
            let Ok(mut state) = sync.lock() else { break };
            match state.remote_needs_apply(&packet) {
                Ok(false) => {}
                Ok(true) => match apply(&packet.event) {
                    Ok(()) => state.remote_applied(&packet),
                    Err(error) => {
                        eprintln!("SeamlessControl: no se pudo aplicar el portapapeles: {error}")
                    }
                },
                Err(error) => {
                    eprintln!("SeamlessControl: cambio de portapapeles inválido: {error}")
                }
            }
        }
    });
    (sender, worker)
}

pub fn spawn_apply_events() -> (mpsc::Sender<ClipboardEvent>, JoinHandle<()>) {
    let (sender, mut receiver) = mpsc::channel::<ClipboardEvent>(8);
    let worker = thread::spawn(move || {
        while let Some(event) = receiver.blocking_recv() {
            if let Err(error) = apply(&event) {
                eprintln!("SeamlessControl: no se pudo aplicar el portapapeles: {error}");
            }
        }
    });
    (sender, worker)
}

pub struct ClipboardWatch {
    child: Arc<Mutex<Child>>,
    worker: Option<JoinHandle<()>>,
}

impl ClipboardWatch {
    pub fn start(
        sync: Arc<Mutex<ClipboardSync>>,
        mut forward: impl FnMut(ClipboardPacket) + Send + 'static,
    ) -> io::Result<Self> {
        Self::start_events(move |event| {
            let Ok(mut state) = sync.lock() else { return };
            if let Some(packet) = state.local_changed(&event) {
                drop(state);
                forward(packet);
            }
        })
    }

    pub fn start_events(
        mut forward: impl FnMut(ClipboardEvent) + Send + 'static,
    ) -> io::Result<Self> {
        let executable = std::env::current_exe()?;
        let mut child = Command::new("wl-paste")
            .args(["--type", "text/plain", "--watch"])
            .arg(executable)
            .arg("clipboard-helper")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let mut output = child
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("wl-paste stdout missing"))?;
        let child = Arc::new(Mutex::new(child));
        let worker = thread::spawn(move || {
            loop {
                match ClipboardEvent::read_framed(&mut output) {
                    Ok(Some(event)) => forward(event),
                    Ok(None) => break,
                    Err(error) => {
                        eprintln!("SeamlessControl: error observando portapapeles: {error}");
                        break;
                    }
                }
            }
        });
        Ok(Self {
            child,
            worker: Some(worker),
        })
    }

    pub fn stop(mut self) {
        if let Ok(mut child) = self.child.lock() {
            let _ = child.kill();
            let _ = child.wait();
        }
        if let Some(worker) = self.worker.take()
            && worker.is_finished()
        {
            let _ = worker.join();
        }
    }
}
