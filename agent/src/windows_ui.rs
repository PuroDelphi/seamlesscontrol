//! WebView2 window and tray shell over the existing seamlesscontrold.exe.
//! No input, encryption, file or clipboard protocol is reimplemented here.

use seamlesscontrol_core::windows_clipboard::WindowsClipboard;
use seamlesscontrol_core::windows_discovery::{DiscoveredServer, DiscoveryBrowser};
use std::collections::{BTreeMap, VecDeque};
use std::error::Error;
use std::io::{self, BufRead, BufReader, Write};
use std::net::{IpAddr, SocketAddr};
use std::os::windows::ffi::OsStrExt;
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use tao::dpi::LogicalSize;
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy};
use tao::platform::windows::WindowExtWindows;
use tao::window::{Icon as WindowIcon, Window, WindowBuilder};
use tray_icon::menu::{Menu, MenuEvent, MenuItem};
use tray_icon::{Icon as TrayImage, MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use windows_sys::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
use windows_sys::Win32::System::Registry::{
    HKEY_CURRENT_USER, REG_SZ, RRF_RT_REG_SZ, RegDeleteKeyValueW, RegGetValueW, RegSetKeyValueW,
};
use windows_sys::Win32::UI::Controls::Dialogs::{
    GetOpenFileNameW, OFN_FILEMUSTEXIST, OFN_PATHMUSTEXIST, OPENFILENAMEW,
};
use windows_sys::Win32::UI::Shell::ShellExecuteW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    MB_ICONERROR, MB_ICONQUESTION, MB_OK, MB_SETFOREGROUND, MB_TOPMOST, MB_YESNO, MessageBoxW,
};
use wry::{WebView, WebViewBuilder};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const HTML: &str = include_str!("windows_ui.html");
const DEFAULT_FILE_LIMIT_MIB: u64 = 100;
const MAX_FILE_LIMIT_MIB: u64 = 10240;
const STARTUP_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const STARTUP_VALUE: &str = "SeamlessControl";

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn startup_command(executable: &std::path::Path) -> io::Result<Vec<u16>> {
    let mut command = vec![u16::from(b'"')];
    command.extend(executable.as_os_str().encode_wide());
    command.extend("\" --background".encode_utf16());
    if command.len() >= 260 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "The app folder path is too long for Windows startup. Move both executables to a shorter folder.",
        ));
    }
    command.push(0);
    Ok(command)
}

fn startup_status(executable: &std::path::Path) -> io::Result<&'static str> {
    let key = wide(STARTUP_KEY);
    let name = wide(STARTUP_VALUE);
    let mut bytes = 0u32;
    let result = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            key.as_ptr(),
            name.as_ptr(),
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut bytes,
        )
    };
    if result == ERROR_FILE_NOT_FOUND {
        return Ok("off");
    }
    if result != ERROR_SUCCESS {
        return Err(io::Error::from_raw_os_error(result as i32));
    }
    if bytes % 2 != 0 || bytes == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid Windows startup entry",
        ));
    }
    let mut value = vec![0u16; (bytes / 2) as usize];
    let result = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            key.as_ptr(),
            name.as_ptr(),
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            value.as_mut_ptr().cast(),
            &mut bytes,
        )
    };
    if result != ERROR_SUCCESS {
        return Err(io::Error::from_raw_os_error(result as i32));
    }
    Ok(if value == startup_command(executable)? {
        "on"
    } else {
        "moved"
    })
}

fn set_startup(executable: &std::path::Path, enabled: bool) -> io::Result<()> {
    let key = wide(STARTUP_KEY);
    let name = wide(STARTUP_VALUE);
    let result = if enabled {
        let command = startup_command(executable)?;
        unsafe {
            RegSetKeyValueW(
                HKEY_CURRENT_USER,
                key.as_ptr(),
                name.as_ptr(),
                REG_SZ,
                command.as_ptr().cast(),
                (command.len() * std::mem::size_of::<u16>()) as u32,
            )
        }
    } else {
        unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, key.as_ptr(), name.as_ptr()) }
    };
    if result == ERROR_SUCCESS || (!enabled && result == ERROR_FILE_NOT_FOUND) {
        Ok(())
    } else {
        Err(io::Error::from_raw_os_error(result as i32))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
enum Slot {
    Serve,
    Connect,
    Pair,
    ReceiveFile,
    ReceiveClipboard,
    SendFile,
}

impl Slot {
    fn name(self) -> &'static str {
        match self {
            Self::Serve => "receive",
            Self::Connect => "connect",
            Self::Pair => "pair",
            Self::ReceiveFile => "files",
            Self::ReceiveClipboard => "copied files",
            Self::SendFile => "send",
        }
    }
}

enum UiEvent {
    Command(String),
    Line(Slot, String),
    Discovery(Vec<DiscoveredServer>),
    Tray(TrayIconEvent),
    Menu(MenuEvent),
    ClipboardDecision(String, bool),
}

struct Process {
    child: Child,
    stdin: Option<ChildStdin>,
}

struct PairCode {
    slot: Slot,
    digits: String,
}

struct Controller {
    cli: PathBuf,
    executable: PathBuf,
    startup: &'static str,
    receive_port: u16,
    file_limit_mib: u64,
    file_limit_path: PathBuf,
    approval_path: PathBuf,
    approval_mode: String,
    approval_minutes: u64,
    approval_until: BTreeMap<String, Instant>,
    language: String,
    layout_path: PathBuf,
    layout: BTreeMap<String, String>,
    processes: BTreeMap<Slot, Process>,
    logs: VecDeque<String>,
    peers: BTreeMap<IpAddr, String>,
    discovered: Vec<DiscoveredServer>,
    pair_code: Option<PairCode>,
    file_offer: Option<String>,
    clipboard_offer: Option<String>,
    clipboard_notification_open: bool,
    clipboard: WindowsClipboard,
    clipboard_staging: PathBuf,
    copied_file: Option<PathBuf>,
    last_copied_send: Option<PathBuf>,
    copied_send_active: bool,
    clipboard_offer_since: Option<Instant>,
    clipboard_progress: Option<u8>,
    clipboard_receiver_attempt: Instant,
    file_path: Option<PathBuf>,
    state_message: String,
}

impl Controller {
    fn new(cli: PathBuf, executable: PathBuf) -> Self {
        let startup = startup_status(&executable).unwrap_or("error");
        let layout_path = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_default()
            .join("SeamlessControl")
            .join("ui-layout.json");
        let file_limit_path = layout_path.with_file_name("file-limit-mib");
        let approval_path = layout_path.with_file_name("file-approval.json");
        let approval_settings = std::fs::read(&approval_path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok());
        let approval_mode = approval_settings
            .as_ref()
            .and_then(|settings| settings["mode"].as_str())
            .filter(|mode| matches!(*mode, "always" | "automatic" | "timed"))
            .unwrap_or("always")
            .to_owned();
        let approval_minutes = approval_settings
            .as_ref()
            .and_then(|settings| settings["minutes"].as_u64())
            .filter(|minutes| (1..=1440).contains(minutes))
            .unwrap_or(15);
        let clipboard_staging = layout_path.with_file_name("clipboard-files");
        let file_limit_mib = std::fs::read_to_string(&file_limit_path)
            .ok()
            .and_then(|value| value.trim().parse::<u64>().ok())
            .filter(|value| (1..=MAX_FILE_LIMIT_MIB).contains(value))
            .unwrap_or(DEFAULT_FILE_LIMIT_MIB);
        let layout = std::fs::read(&layout_path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        Self {
            cli,
            executable,
            startup,
            receive_port: 47832,
            file_limit_mib,
            file_limit_path,
            approval_path,
            approval_mode,
            approval_minutes,
            approval_until: BTreeMap::new(),
            language: "en".into(),
            layout_path,
            layout,
            processes: BTreeMap::new(),
            logs: VecDeque::new(),
            peers: BTreeMap::new(),
            discovered: Vec::new(),
            pair_code: None,
            file_offer: None,
            clipboard_offer: None,
            clipboard_notification_open: false,
            clipboard: WindowsClipboard::new(),
            clipboard_staging,
            copied_file: None,
            last_copied_send: None,
            copied_send_active: false,
            clipboard_offer_since: None,
            clipboard_progress: None,
            clipboard_receiver_attempt: Instant::now() - Duration::from_secs(5),
            file_path: None,
            state_message: "Ready. Start receiving or select a nearby computer.".into(),
        }
    }

    fn log(&mut self, message: impl Into<String>) {
        let message = message.into();
        self.state_message = message.clone();
        self.logs.push_back(message);
        while self.logs.len() > 36 {
            self.logs.pop_front();
        }
    }

    fn command(&self, args: &[&str]) -> Command {
        let mut command = Command::new(&self.cli);
        command.args(args).creation_flags(CREATE_NO_WINDOW);
        command
    }

    fn start(
        &mut self,
        slot: Slot,
        args: &[&str],
        proxy: &EventLoopProxy<UiEvent>,
    ) -> io::Result<()> {
        self.stop(slot);
        let mut command = self.command(args);
        if matches!(
            slot,
            Slot::ReceiveFile | Slot::ReceiveClipboard | Slot::SendFile
        ) {
            command.env(
                "SEAMLESSCONTROL_MAX_FILE_BYTES",
                (self.file_limit_mib * 1024 * 1024).to_string(),
            );
        }
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let stdin = child.stdin.take();
        for stream in [
            child
                .stdout
                .take()
                .map(|x| Box::new(x) as Box<dyn io::Read + Send>),
            child
                .stderr
                .take()
                .map(|x| Box::new(x) as Box<dyn io::Read + Send>),
        ]
        .into_iter()
        .flatten()
        {
            let proxy = proxy.clone();
            thread::spawn(move || {
                for line in BufReader::new(stream).lines() {
                    let Ok(line) = line else { break };
                    if proxy.send_event(UiEvent::Line(slot, line)).is_err() {
                        break;
                    }
                }
            });
        }
        self.processes.insert(slot, Process { child, stdin });
        self.log(format!("{} started", slot.name()));
        Ok(())
    }

    fn stop(&mut self, slot: Slot) {
        if let Some(mut process) = self.processes.remove(&slot) {
            let _ = process.child.kill();
            let _ = process.child.wait();
            self.log(format!("{} stopped", slot.name()));
        }
        if self
            .pair_code
            .as_ref()
            .is_some_and(|code| code.slot == slot)
        {
            self.pair_code = None;
        }
        if slot == Slot::ReceiveFile {
            self.file_offer = None;
        }
        if slot == Slot::ReceiveClipboard {
            self.clipboard_offer = None;
            self.clipboard_notification_open = false;
            self.clipboard_offer_since = None;
            self.clipboard_progress = None;
        }
        if slot == Slot::SendFile {
            self.copied_send_active = false;
        }
    }

    fn poll(&mut self, proxy: &EventLoopProxy<UiEvent>) {
        let mut finished = Vec::new();
        for (&slot, process) in &mut self.processes {
            if let Ok(Some(status)) = process.child.try_wait() {
                finished.push((slot, status.success()));
            }
        }
        for (slot, success) in finished {
            self.processes.remove(&slot);
            self.log(format!(
                "{} {}",
                slot.name(),
                if success {
                    "completed"
                } else {
                    "ended with an error"
                }
            ));
            if self
                .pair_code
                .as_ref()
                .is_some_and(|code| code.slot == slot)
            {
                self.pair_code = None;
            }
            if slot == Slot::ReceiveFile {
                self.file_offer = None;
            }
            if slot == Slot::ReceiveClipboard {
                self.clipboard_offer = None;
                self.clipboard_notification_open = false;
                self.clipboard_offer_since = None;
                self.clipboard_progress = None;
            }
            if slot == Slot::SendFile {
                self.copied_send_active = false;
            }
            if slot == Slot::Connect {
                let _ = self.resume_receiving(proxy);
            }
        }
        if !self.processes.contains_key(&Slot::ReceiveClipboard)
            && self.clipboard_receiver_attempt.elapsed() >= Duration::from_secs(4)
        {
            self.clipboard_receiver_attempt = Instant::now();
            if let Err(error) = self.start(
                Slot::ReceiveClipboard,
                &["receive-file-clipboard-ui", "0.0.0.0:47834"],
                proxy,
            ) {
                self.log(format!("Could not wait for copied files: {error}"));
            }
        }
        match self
            .clipboard
            .copied_file(self.file_limit_mib * 1024 * 1024, &self.clipboard_staging)
        {
            Ok(Some(path)) => {
                if self.copied_send_active {
                    self.stop(Slot::SendFile);
                    self.log("Copied-file offer canceled because the clipboard changed.");
                }
                self.copied_file = path;
                self.last_copied_send = None;
            }
            Ok(None) => {}
            Err(error) => self.log(format!("Could not inspect copied file: {error}")),
        }
        if self.peers.len() == 1
            && self.copied_file.is_some()
            && self.copied_file != self.last_copied_send
            && !self.processes.contains_key(&Slot::SendFile)
        {
            let ip = *self.peers.keys().next().expect("one peer");
            let _ = self.send_copied_file(ip, proxy);
        }
        if self
            .clipboard_offer_since
            .is_some_and(|since| since.elapsed() >= Duration::from_secs(120))
        {
            let _ = self.decide_copied_file(false);
            self.log("Copied-file offer expired after two minutes.");
        }
    }

    fn resume_receiving(&mut self, proxy: &EventLoopProxy<UiEvent>) -> io::Result<()> {
        self.start(
            Slot::Serve,
            &["serve", &format!("0.0.0.0:{}", self.receive_port)],
            proxy,
        )
    }

    fn shutdown(&mut self) {
        for slot in [
            Slot::Connect,
            Slot::Serve,
            Slot::ReceiveFile,
            Slot::ReceiveClipboard,
            Slot::SendFile,
            Slot::Pair,
        ] {
            self.stop(slot);
        }
    }

    fn line(&mut self, slot: Slot, line: String) {
        let line = line.trim();
        if let Some(code) = line.strip_prefix("PAIRING CODE: ") {
            if code.len() == 6 && code.bytes().all(|byte| byte.is_ascii_digit()) {
                self.pair_code = Some(PairCode {
                    slot,
                    digits: code.to_owned(),
                });
            }
        }
        if slot == Slot::ReceiveFile && line.starts_with("OFFER\t") {
            self.file_offer = Some(line.to_owned());
            if let Some(peer) = offer_peer(line)
                && self.should_auto_accept(peer)
            {
                if self.reply(Slot::ReceiveFile, "SI").is_ok() {
                    self.file_offer = None;
                    self.log(format!("File automatically accepted from {peer}."));
                }
            }
        }
        if slot == Slot::ReceiveClipboard {
            if line.starts_with("OFFER\t") {
                self.clipboard_offer = Some(line.to_owned());
                self.clipboard_notification_open = false;
                self.clipboard_offer_since = Some(Instant::now());
                self.clipboard_progress = None;
                if let Some(peer) = offer_peer(line)
                    && self.should_auto_accept(peer)
                {
                    if self.reply(Slot::ReceiveClipboard, "SI").is_ok() {
                        self.clipboard_offer = None;
                        self.clipboard_offer_since = None;
                        self.log(format!("Copied file automatically accepted from {peer}."));
                    }
                }
            } else if let Some(percent) = line.strip_prefix("PROGRESS\t") {
                self.clipboard_progress = percent.parse::<u8>().ok().filter(|value| *value <= 100);
            } else if let Some(encoded) = line.strip_prefix("FILE_READY\t") {
                if let Ok(path) = serde_json::from_str::<String>(encoded) {
                    self.log(format!("File ready to paste: {path}"));
                }
                self.clipboard_offer = None;
                self.clipboard_offer_since = None;
                self.clipboard_progress = None;
            } else if line == "FILE_DECLINED" {
                self.clipboard_offer = None;
                self.clipboard_offer_since = None;
                self.clipboard_progress = None;
            } else if line == "STAGING_FULL" {
                self.log("Copied-file staging folder is full. Free space and copy again.");
            }
        }
        if line.starts_with("Paired with ") {
            if self
                .pair_code
                .as_ref()
                .is_some_and(|code| code.slot == slot)
            {
                self.pair_code = None;
            }
            self.refresh_peers();
        } else if slot == Slot::Serve && line.starts_with("SeamlessControl connection from ") {
            self.pair_code = None;
        }
        if !line.is_empty() {
            self.log(format!("{} · {}", slot.name(), line));
        }
    }

    fn reply(&mut self, slot: Slot, text: &str) -> io::Result<()> {
        let process = self
            .processes
            .get_mut(&slot)
            .ok_or_else(|| io::Error::other("operation is no longer running"))?;
        let stdin = process
            .stdin
            .as_mut()
            .ok_or_else(|| io::Error::other("operation cannot receive a reply"))?;
        stdin.write_all(text.as_bytes())?;
        stdin.write_all(b"\n")?;
        stdin.flush()
    }

    fn refresh_peers(&mut self) {
        match self.command(&["peers"]).output() {
            Ok(output) if output.status.success() => {
                self.peers.clear();
                for line in String::from_utf8_lossy(&output.stdout).lines() {
                    let fields: Vec<_> = line.split('\t').collect();
                    if fields.len() == 3 && fields[0] == "PEER" {
                        if let Ok(ip) = fields[1].parse::<IpAddr>() {
                            self.peers.insert(ip, fields[2].to_owned());
                        }
                    }
                }
            }
            Ok(output) => self.log(format!(
                "Could not read paired computers: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )),
            Err(error) => self.log(format!("Agent unavailable: {error}")),
        }
    }

    fn send_copied_file(
        &mut self,
        ip: IpAddr,
        proxy: &EventLoopProxy<UiEvent>,
    ) -> Result<(), Box<dyn Error>> {
        if !self.peers.contains_key(&ip) {
            return Err("pair this computer before sending a copied file".into());
        }
        let path = self
            .copied_file
            .clone()
            .ok_or("copy one local file first")?;
        if !seamlesscontrol_core::clipboard_file::local_regular_file(
            &path,
            self.file_limit_mib * 1024 * 1024,
        )? {
            return Err("copied file is no longer valid or exceeds the limit".into());
        }
        self.last_copied_send = Some(path.clone());
        self.start(
            Slot::SendFile,
            &[
                "send-file",
                &SocketAddr::new(ip, 47834).to_string(),
                &path.to_string_lossy(),
            ],
            proxy,
        )?;
        self.copied_send_active = true;
        Ok(())
    }

    fn decide_copied_file(&mut self, accept: bool) -> io::Result<()> {
        let peer = self
            .clipboard_offer
            .as_deref()
            .and_then(offer_peer)
            .ok_or_else(|| io::Error::other("no copied file offer is waiting"))?;
        self.reply(Slot::ReceiveClipboard, if accept { "SI" } else { "NO" })?;
        if accept {
            self.remember_approval(peer);
        }
        self.clipboard_offer = None;
        self.clipboard_offer_since = None;
        Ok(())
    }

    fn decide_manual_file(&mut self, accept: bool) -> io::Result<()> {
        let peer = self
            .file_offer
            .as_deref()
            .and_then(offer_peer)
            .ok_or_else(|| io::Error::other("no file offer is waiting"))?;
        self.reply(Slot::ReceiveFile, if accept { "SI" } else { "NO" })?;
        if accept {
            self.remember_approval(peer);
        }
        self.file_offer = None;
        Ok(())
    }

    fn should_auto_accept(&self, peer: IpAddr) -> bool {
        match self.approval_mode.as_str() {
            "automatic" => true,
            "timed" => self
                .peers
                .get(&peer)
                .and_then(|fingerprint| self.approval_until.get(fingerprint))
                .is_some_and(|until| Instant::now() < *until),
            _ => false,
        }
    }

    fn remember_approval(&mut self, peer: IpAddr) {
        if self.approval_mode == "timed" {
            if let Some(fingerprint) = self.peers.get(&peer) {
                self.approval_until.insert(
                    fingerprint.clone(),
                    Instant::now() + Duration::from_secs(self.approval_minutes * 60),
                );
            }
        }
    }

    fn set_file_approval(&mut self, mode: &str, minutes: &str) -> Result<(), Box<dyn Error>> {
        if !matches!(mode, "always" | "automatic" | "timed") {
            return Err("choose a valid file approval mode".into());
        }
        let minutes: u64 = minutes.parse()?;
        if !(1..=1440).contains(&minutes) {
            return Err("approval time must be between 1 and 1440 minutes".into());
        }
        if let Some(parent) = self.approval_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let bytes = serde_json::to_vec(&serde_json::json!({"mode": mode, "minutes": minutes}))?;
        let temporary = self.approval_path.with_extension("json.tmp");
        std::fs::write(&temporary, bytes)?;
        std::fs::rename(temporary, &self.approval_path)?;
        self.approval_mode = mode.to_owned();
        self.approval_minutes = minutes;
        self.approval_until.clear();
        self.log("File approval preference saved. Existing temporary approvals were cleared.");
        Ok(())
    }

    fn set_layout(&mut self, fingerprint: &str, edge: Option<&str>) -> Result<(), Box<dyn Error>> {
        if !self.peers.values().any(|value| value == fingerprint) {
            return Err("pair this computer before placing it in the layout".into());
        }
        if let Some(edge) = edge {
            if !matches!(edge, "left" | "right" | "top" | "bottom") {
                return Err("choose a valid layout edge".into());
            }
        }
        let mut next = self.layout.clone();
        next.remove(fingerprint);
        if let Some(edge) = edge {
            next.retain(|_, placed| placed != edge);
            next.insert(fingerprint.to_owned(), edge.to_owned());
        }
        if let Some(parent) = self.layout_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&self.layout_path, serde_json::to_vec_pretty(&next)?)?;
        self.layout = next;
        self.log("Computer layout saved.");
        Ok(())
    }

    fn set_file_limit(&mut self, value: &str) -> Result<(), Box<dyn Error>> {
        let limit: u64 = value.parse()?;
        if !(1..=MAX_FILE_LIMIT_MIB).contains(&limit) {
            return Err("file limit must be between 1 and 10240 MiB".into());
        }
        if let Some(parent) = self.file_limit_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&self.file_limit_path, format!("{limit}\n"))?;
        self.file_limit_mib = limit;
        self.log(format!(
            "File limit saved: {limit} MiB. Restart file waiting if active."
        ));
        Ok(())
    }

    fn own_fingerprint(&self) -> String {
        self.command(&["identity"])
            .output()
            .ok()
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .and_then(|line| {
                line.trim()
                    .strip_prefix("Local identity: ")
                    .map(str::to_owned)
            })
            .unwrap_or_default()
    }

    fn snapshot(&self) -> serde_json::Value {
        let discovered: Vec<_> = self
            .discovered
            .iter()
            .map(|peer| {
                let pinned = self.peers.get(&peer.address.ip());
                serde_json::json!({
                    "name": peer.name,
                    "address": peer.address.to_string(),
                    "fingerprint": peer.fingerprint,
                    "trust": match pinned {
                        Some(value) if value == &peer.fingerprint => "paired",
                        Some(_) => "changed",
                        None => "new",
                    }
                })
            })
            .collect();
        let peers: Vec<_> = self
            .peers
            .iter()
            .map(|(ip, fingerprint)| {
                serde_json::json!({
                    "ip": ip.to_string(), "fingerprint": fingerprint,
                })
            })
            .collect();
        serde_json::json!({
            "receive": self.processes.contains_key(&Slot::Serve),
            "connect": self.processes.contains_key(&Slot::Connect),
            "pairing": self.processes.contains_key(&Slot::Pair),
            "fileReceive": self.processes.contains_key(&Slot::ReceiveFile),
            "fileSend": self.processes.contains_key(&Slot::SendFile),
            "fileLimitMiB": self.file_limit_mib,
            "startup": self.startup,
            "approvalMode": self.approval_mode,
            "approvalMinutes": self.approval_minutes,
            "pairCode": self.pair_code.as_ref().map(|code| &code.digits),
            "fileOffer": self.file_offer,
            "clipboardOffer": self.clipboard_offer,
            "clipboardProgress": self.clipboard_progress,
            "clipboardReady": self.processes.contains_key(&Slot::ReceiveClipboard),
            "copiedFile": self.copied_file.as_ref().map(|path| path.to_string_lossy().to_string()),
            "filePath": self.file_path.as_ref().map(|path| path.to_string_lossy().to_string()),
            "defaultDownload": std::env::var_os("USERPROFILE")
                .map(|path| PathBuf::from(path).join("Downloads").to_string_lossy().to_string()),
            "message": self.state_message,
            "logs": self.logs,
            "discovered": discovered,
            "peers": peers,
            "layout": self.layout,
        })
    }

    fn handle(&mut self, payload: &str, proxy: &EventLoopProxy<UiEvent>, window: &Window) -> bool {
        let Ok(command) = serde_json::from_str::<serde_json::Value>(payload) else {
            self.log("Invalid interface request");
            return false;
        };
        let string = |name: &str| {
            command
                .get(name)
                .and_then(|value| value.as_str())
                .unwrap_or("")
                .trim()
        };
        let action = string("action");
        let result: Result<(), Box<dyn Error>> = (|| {
            match action {
                "startReceive" => {
                    let port = parse_port(string("port"))?;
                    self.receive_port = port;
                    self.stop(Slot::Connect);
                    self.resume_receiving(proxy)?;
                }
                "stopReceive" => self.stop(Slot::Serve),
                "connect" => {
                    let address = parse_address(string("address"))?;
                    let edge = string("edge");
                    if !matches!(edge, "left" | "right" | "top" | "bottom") {
                        return Err("choose the Windows edge facing the other computer".into());
                    }
                    self.stop(Slot::Serve);
                    self.start(
                        Slot::Connect,
                        &["connect", &address.to_string(), edge],
                        proxy,
                    )?;
                }
                "stopConnect" => {
                    self.stop(Slot::Connect);
                    self.resume_receiving(proxy)?;
                }
                "pair" => {
                    let address = parse_address(string("address"))?;
                    self.start(Slot::Pair, &["pair", &address.to_string()], proxy)?;
                }
                "approvePair" => {
                    let Some(code) = self.pair_code.as_ref() else {
                        return Err("no pairing code is waiting".into());
                    };
                    if string("code") != code.digits {
                        return Err("type the exact code displayed on both computers".into());
                    }
                    let slot = code.slot;
                    self.reply(slot, &code.digits.clone())?;
                    self.pair_code = None;
                    self.log("Pairing code approved here; approve it on the other computer too.");
                }
                "declinePair" => {
                    if let Some(code) = self.pair_code.take() {
                        self.reply(code.slot, "NO")?;
                    }
                }
                "refreshPeers" => self.refresh_peers(),
                "setFileLimit" => {
                    self.set_file_limit(string("value"))?;
                    if self.clipboard_offer.is_none() {
                        self.stop(Slot::ReceiveClipboard);
                        self.clipboard_receiver_attempt = Instant::now() - Duration::from_secs(5);
                    }
                }
                "setFileApproval" => self.set_file_approval(string("mode"), string("minutes"))?,
                "enableStartup" | "disableStartup" => {
                    let enabled = action == "enableStartup";
                    set_startup(&self.executable, enabled)?;
                    self.startup = startup_status(&self.executable)?;
                    self.log(if enabled {
                        "SeamlessControl will start in the tray when you sign in to Windows."
                    } else {
                        "Start with Windows turned off."
                    });
                }
                "setLanguage" => {
                    if !matches!(string("language"), "en" | "es") {
                        return Err("choose English or Español".into());
                    }
                    self.language = string("language").to_owned();
                }
                "sendCopiedFile" => {
                    let ip = string("ip").parse::<IpAddr>()?;
                    self.send_copied_file(ip, proxy)?;
                }
                "acceptCopiedFile" => self.decide_copied_file(true)?,
                "declineCopiedFile" => self.decide_copied_file(false)?,
                "waitFile" => {
                    let port = parse_port(string("port"))?;
                    let directory = PathBuf::from(string("directory"));
                    if !directory.is_dir() {
                        return Err("choose an existing destination folder".into());
                    }
                    self.start(
                        Slot::ReceiveFile,
                        &[
                            "receive-file-ui",
                            &format!("0.0.0.0:{port}"),
                            &directory.to_string_lossy(),
                        ],
                        proxy,
                    )?;
                }
                "stopFile" => self.stop(Slot::ReceiveFile),
                "acceptFile" => self.decide_manual_file(true)?,
                "declineFile" => self.decide_manual_file(false)?,
                "browseFile" => {
                    self.file_path = choose_file(window);
                }
                "sendFile" => {
                    let address = parse_address(string("address"))?;
                    let path = self.file_path.clone().ok_or("choose a file first")?;
                    if !path.is_file() {
                        return Err("selected file is no longer available".into());
                    }
                    self.copied_send_active = false;
                    self.start(
                        Slot::SendFile,
                        &["send-file", &address.to_string(), &path.to_string_lossy()],
                        proxy,
                    )?;
                }
                "firewallControl" | "firewallFiles" | "firewallCopied" | "firewallDiscovery" => {
                    let (port, protocol) = if action == "firewallDiscovery" {
                        (5353, "UDP")
                    } else if action == "firewallCopied" {
                        (47834, "TCP")
                    } else {
                        (parse_port(string("port"))?, "TCP")
                    };
                    request_firewall_rule(window, &self.cli, port, protocol)?;
                    self.log(format!(
                        "Windows administrator approval requested for Private LAN {protocol} {port}."
                    ));
                }
                "revoke" => {
                    let ip = string("ip").parse::<IpAddr>()?;
                    let output = self.command(&["revoke", &ip.to_string()]).output()?;
                    if !output.status.success() {
                        return Err(String::from_utf8_lossy(&output.stderr).to_string().into());
                    }
                    self.refresh_peers();
                    self.log(format!("Trust revoked for {ip}"));
                }
                "setLayout" => {
                    self.set_layout(string("fingerprint"), Some(string("edge")))?;
                }
                "clearLayout" => {
                    self.set_layout(string("fingerprint"), None)?;
                }
                "hide" => window.set_visible(false),
                "quit" => {
                    self.shutdown();
                    return Ok(());
                }
                _ => return Err("unknown interface action".into()),
            }
            Ok(())
        })();
        if let Err(error) = result {
            self.log(error.to_string());
        }
        action == "quit"
    }
}

impl Drop for Controller {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn parse_port(value: &str) -> Result<u16, Box<dyn Error>> {
    let port: u16 = value.parse()?;
    if port == 0 {
        return Err("port must be between 1 and 65535".into());
    }
    Ok(port)
}

fn offer_peer(line: &str) -> Option<IpAddr> {
    let mut fields = line.split('\t');
    (fields.next() == Some("OFFER"))
        .then(|| fields.next()?.parse::<IpAddr>().ok())
        .flatten()
}

fn parse_address(value: &str) -> Result<SocketAddr, Box<dyn Error>> {
    let address: SocketAddr = value.parse()?;
    if address.port() == 0 {
        return Err("peer port must be nonzero".into());
    }
    let local = match address.ip() {
        IpAddr::V4(ip) => ip.is_private() || ip.is_link_local() || ip.is_loopback(),
        IpAddr::V6(ip) => {
            ip.is_loopback() || ip.is_unicast_link_local() || ip.segments()[0] & 0xfe00 == 0xfc00
        }
    };
    if !local {
        return Err("choose a private LAN address".into());
    }
    Ok(address)
}

fn show_copied_file_prompt(offer: String, spanish: bool, proxy: EventLoopProxy<UiEvent>) {
    thread::spawn(move || {
        let fields: Vec<_> = offer.split('\t').collect();
        let body = if fields.len() == 5 && spanish {
            format!(
                "{}\n\nDe: {}\nTamaño: {} bytes\n\nSi lo acepta, se verificará y quedará listo para pegar en el Explorador. El resultado aparecerá en Actividad.",
                fields[2], fields[1], fields[3]
            )
        } else if fields.len() == 5 {
            format!(
                "{}\n\nFrom: {}\nSize: {} bytes\n\nIf accepted, it will be verified and made ready to paste in Explorer. The result appears in Activity.",
                fields[2], fields[1], fields[3]
            )
        } else {
            if spanish {
                "¿Aceptar el archivo copiado desde el equipo emparejado?".to_owned()
            } else {
                "Accept the copied file from the paired computer?".to_owned()
            }
        };
        let title: Vec<u16> = if spanish {
            "SeamlessControl · Archivo entrante\0"
        } else {
            "SeamlessControl · Incoming file\0"
        }
        .encode_utf16()
        .collect();
        let body: Vec<u16> = format!("{body}\0").encode_utf16().collect();
        let response = unsafe {
            MessageBoxW(
                std::ptr::null_mut(),
                body.as_ptr(),
                title.as_ptr(),
                MB_YESNO | MB_ICONQUESTION | MB_TOPMOST | MB_SETFOREGROUND,
            )
        };
        let _ = proxy.send_event(UiEvent::ClipboardDecision(offer, response == 6));
    });
}

fn choose_file(window: &Window) -> Option<PathBuf> {
    let mut path = [0u16; 32768];
    let filter: Vec<u16> = "All files\0*.*\0\0".encode_utf16().collect();
    let mut dialog = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        hwndOwner: window.hwnd() as _,
        lpstrFilter: filter.as_ptr(),
        lpstrFile: path.as_mut_ptr(),
        nMaxFile: path.len() as u32,
        Flags: OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST,
        ..Default::default()
    };
    if unsafe { GetOpenFileNameW(&mut dialog) } == 0 {
        return None;
    }
    let end = path.iter().position(|unit| *unit == 0)?;
    Some(PathBuf::from(String::from_utf16_lossy(&path[..end])))
}

fn request_firewall_rule(
    window: &Window,
    cli: &PathBuf,
    port: u16,
    protocol: &str,
) -> Result<(), Box<dyn Error>> {
    let verb: Vec<u16> = "runas\0".encode_utf16().collect();
    let executable: Vec<u16> = cli
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let parameters: Vec<u16> = format!("firewall-allow {port} {protocol}\0")
        .encode_utf16()
        .collect();
    let result = unsafe {
        ShellExecuteW(
            window.hwnd() as _,
            verb.as_ptr(),
            executable.as_ptr(),
            parameters.as_ptr(),
            std::ptr::null(),
            1,
        )
    } as isize;
    if result <= 32 {
        return Err(
            format!("Windows did not authorize the firewall request (code {result})").into(),
        );
    }
    Ok(())
}

fn icon_rgba() -> Vec<u8> {
    // Keep these pixels in sync with assets/seamlesscontrol.ico, the EXE icon.
    let mut pixels = vec![0u8; 32 * 32 * 4];
    for y in 0..32 {
        for x in 0..32 {
            let offset = (y * 32 + x) * 4;
            let circle = (x as i32 - 16).pow(2) + (y as i32 - 16).pow(2) <= 15 * 15;
            let left = (4..=16).contains(&x) && (10..=22).contains(&y);
            let right = (16..=28).contains(&x) && (7..=19).contains(&y);
            let border = (left && (x == 4 || x == 16 || y == 10 || y == 22))
                || (right && (x == 16 || x == 28 || y == 7 || y == 19));
            let flow = (10..=22).contains(&x) && (13..=16).contains(&y);
            let rgba = if border || flow {
                [151, 215, 226, 255]
            } else if circle {
                [16, 36, 52, 255]
            } else {
                [0, 0, 0, 0]
            };
            pixels[offset..offset + 4].copy_from_slice(&rgba);
        }
    }
    pixels
}

fn show_error(message: &str) {
    let body: Vec<u16> = format!("SeamlessControl could not start:\n\n{message}\0")
        .encode_utf16()
        .collect();
    let title: Vec<u16> = "SeamlessControl\0".encode_utf16().collect();
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            body.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONERROR,
        )
    };
}

fn push_state(webview: &WebView, controller: &Controller) {
    let script = format!("window.seamlessReceive({})", controller.snapshot());
    let _ = webview.evaluate_script(&script);
}

pub fn run() {
    if let Err(error) = run_app() {
        show_error(&error.to_string());
    }
}

fn run_app() -> Result<(), Box<dyn Error>> {
    let background = std::env::args_os().skip(1).any(|arg| arg == "--background");
    let current = std::env::current_exe()?;
    let cli = current
        .parent()
        .ok_or("application folder is unavailable")?
        .join("seamlesscontrold.exe");
    if !cli.is_file() {
        return Err(
            "seamlesscontrold.exe must be in the same folder as SeamlessControl.exe".into(),
        );
    }
    let event_loop = EventLoopBuilder::<UiEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();
    let icon = icon_rgba();
    let window_icon = WindowIcon::from_rgba(icon.clone(), 32, 32)?;
    let window = WindowBuilder::new()
        .with_title("SeamlessControl")
        .with_inner_size(LogicalSize::new(1080.0, 760.0))
        .with_min_inner_size(LogicalSize::new(850.0, 640.0))
        .with_visible(!background)
        .with_window_icon(Some(window_icon))
        .build(&event_loop)?;
    let ipc_proxy = proxy.clone();
    let webview = WebViewBuilder::new()
        .with_html(HTML)
        .with_ipc_handler(move |request| {
            let _ = ipc_proxy.send_event(UiEvent::Command(request.body().clone()));
        })
        .build(&window)?;
    let menu = Menu::new();
    let open_item = MenuItem::with_id("open", "Open SeamlessControl", true, None);
    let exit_item = MenuItem::with_id("exit", "Exit SeamlessControl", true, None);
    menu.append(&open_item)?;
    menu.append(&exit_item)?;
    let _tray = TrayIconBuilder::new()
        .with_icon(TrayImage::from_rgba(icon, 32, 32)?)
        .with_tooltip("SeamlessControl · open the panel")
        .with_menu(Box::new(menu))
        .build()?;
    let tray_proxy = proxy.clone();
    TrayIconEvent::set_event_handler(Some(move |event| {
        let _ = tray_proxy.send_event(UiEvent::Tray(event));
    }));
    let menu_proxy = proxy.clone();
    MenuEvent::set_event_handler(Some(move |event| {
        let _ = menu_proxy.send_event(UiEvent::Menu(event));
    }));
    let mut controller = Controller::new(cli, current);
    controller.refresh_peers();
    if let Err(error) = controller.start(Slot::Serve, &["serve", "0.0.0.0:47832"], &proxy) {
        controller.log(format!("Could not receive control automatically: {error}"));
    }
    let own_fingerprint = controller.own_fingerprint();
    let discovery_proxy = proxy.clone();
    let _browser = match DiscoveryBrowser::start(own_fingerprint, move |items| {
        let _ = discovery_proxy.send_event(UiEvent::Discovery(items));
    }) {
        Ok(browser) => Some(browser),
        Err(error) => {
            controller.log(format!(
                "Automatic discovery unavailable: {error}. Manual IP remains available."
            ));
            None
        }
    };
    push_state(&webview, &controller);
    let mut last_poll = Instant::now();
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(400));
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                window.set_visible(false);
            }
            Event::UserEvent(UiEvent::Command(payload)) => {
                let quit = controller.handle(&payload, &proxy, &window);
                push_state(&webview, &controller);
                if quit {
                    *control_flow = ControlFlow::Exit;
                }
            }
            Event::UserEvent(UiEvent::Line(slot, line)) => {
                controller.line(slot, line);
                if controller.clipboard_offer.is_some() && !controller.clipboard_notification_open {
                    controller.clipboard_notification_open = true;
                    show_copied_file_prompt(
                        controller.clipboard_offer.clone().unwrap_or_default(),
                        controller.language == "es",
                        proxy.clone(),
                    );
                }
                push_state(&webview, &controller);
            }
            Event::UserEvent(UiEvent::ClipboardDecision(offer, accept)) => {
                if controller.clipboard_offer.as_deref() == Some(offer.as_str()) {
                    if let Err(error) = controller.decide_copied_file(accept) {
                        controller.log(format!("Could not answer file offer: {error}"));
                    }
                    controller.clipboard_notification_open = false;
                }
                push_state(&webview, &controller);
            }
            Event::UserEvent(UiEvent::Discovery(items)) => {
                controller.discovered = items;
                push_state(&webview, &controller);
            }
            Event::UserEvent(UiEvent::Tray(TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            }))
            | Event::UserEvent(UiEvent::Tray(TrayIconEvent::DoubleClick {
                button: MouseButton::Left,
                ..
            })) => {
                window.set_visible(true);
                window.set_focus();
                push_state(&webview, &controller);
            }
            Event::UserEvent(UiEvent::Menu(event)) if &event.id == open_item.id() => {
                window.set_visible(true);
                window.set_focus();
            }
            Event::UserEvent(UiEvent::Menu(event)) if &event.id == exit_item.id() => {
                controller.shutdown();
                *control_flow = ControlFlow::Exit;
            }
            _ => {}
        }
        if last_poll.elapsed() >= Duration::from_millis(400) {
            last_poll = Instant::now();
            controller.poll(&proxy);
            push_state(&webview, &controller);
        }
    });
}
