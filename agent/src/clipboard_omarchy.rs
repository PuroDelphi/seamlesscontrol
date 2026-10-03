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

const FILE_ACTION_MIMES: [&str; 3] = [
    "x-special/gnome-copied-files",
    "x-special/mate-copied-files",
    "x-special/nautilus-clipboard",
];

pub fn emit_watched_event() -> io::Result<()> {
    let event = match std::env::var("CLIPBOARD_STATE").as_deref() {
        Ok("data") => {
            let mut bytes = Vec::new();
            io::stdin()
                .take((MAX_TEXT_BYTES + 1) as u64)
                .read_to_end(&mut bytes)?;
            if bytes.len() > MAX_TEXT_BYTES
                || std::str::from_utf8(&bytes).is_err()
                || has_file_clipboard_type()
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

fn clipboard_types() -> Vec<String> {
    wl_paste_bounded(&["--list-types"], 8 * 1024)
        .ok()
        .flatten()
        .map(|output| {
            String::from_utf8_lossy(&output)
                .lines()
                .map(|line| line.trim().to_owned())
                .collect()
        })
        .unwrap_or_default()
}

fn has_file_clipboard_type() -> bool {
    clipboard_types()
        .iter()
        .any(|mime| mime == "text/uri-list" || FILE_ACTION_MIMES.contains(&mime.as_str()))
}

fn wl_paste_bounded(args: &[&str], limit: usize) -> io::Result<Option<Vec<u8>>> {
    let mut child = Command::new("wl-paste")
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let mut output = Vec::new();
    child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("wl-paste stdout missing"))?
        .take((limit + 1) as u64)
        .read_to_end(&mut output)?;
    if output.len() > limit {
        let _ = child.kill();
        let _ = child.wait();
        return Ok(None);
    }
    Ok(child.wait()?.success().then_some(output))
}

pub fn copied_file(limit: u64, staging: &Path) -> io::Result<Option<PathBuf>> {
    let types = clipboard_types();
    if types
        .iter()
        .any(|mime| mime == "application/x-kde-cutselection")
        && let Some(action) = wl_paste_bounded(
            &["--no-newline", "--type", "application/x-kde-cutselection"],
            16,
        )?
        && action.first() == Some(&b'1')
    {
        return Ok(None);
    }
    for mime in FILE_ACTION_MIMES {
        if !types.iter().any(|offered| offered == mime) {
            continue;
        }
        let Some(output) = wl_paste_bounded(&["--no-newline", "--type", mime], 64 * 1024)? else {
            continue;
        };
        if output.starts_with(b"cut\n") || output.starts_with(b"cut\r\n") {
            return Ok(None);
        }
        if let Some(uri_list) = output
            .strip_prefix(b"copy\n")
            .or_else(|| output.strip_prefix(b"copy\r\n"))
            && let Some(path) = resolve_uri_list(uri_list)?
            && valid_copied_file(&path, limit, staging)
        {
            return Ok(Some(path));
        }
    }
    if !types.iter().any(|mime| mime == "text/uri-list") {
        return Ok(None);
    }
    let Some(output) = wl_paste_bounded(&["--no-newline", "--type", "text/uri-list"], 64 * 1024)?
    else {
        return Ok(None);
    };
    let Some(path) = resolve_uri_list(&output)? else {
        return Ok(None);
    };
    Ok(valid_copied_file(&path, limit, staging).then_some(path))
}

fn valid_copied_file(path: &Path, limit: u64, staging: &Path) -> bool {
    !path.starts_with(staging) && local_regular_file(path, limit).unwrap_or(false)
}

fn resolve_uri_list(output: &[u8]) -> io::Result<Option<PathBuf>> {
    let path = match parse_one_file_uri(output)? {
        Some(path) => Some(path),
        None => resolve_virtual_uri(output)?,
    };
    Ok(path)
}

fn resolve_virtual_uri(bytes: &[u8]) -> io::Result<Option<PathBuf>> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid URI list"))?;
    let mut entries = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'));
    let Some(uri) = entries.next() else {
        return Ok(None);
    };
    if entries.next().is_some()
        || !["recent://", "starred://", "search://", "trash://"]
            .iter()
            .any(|scheme| uri.starts_with(scheme))
    {
        return Ok(None);
    }
    let output = Command::new("timeout")
        .args(["3s", "gio", "info", "-a", "standard::target-uri", uri])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()?;
    if !output.status.success() || output.stdout.len() > 64 * 1024 {
        return Ok(None);
    }
    let decoded = String::from_utf8_lossy(&output.stdout);
    let target = decoded
        .lines()
        .find_map(|line| line.trim().strip_prefix("standard::target-uri: "))
        .map(str::trim);
    target
        .map(|uri| parse_one_file_uri(uri.as_bytes()))
        .transpose()
        .map(Option::flatten)
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
