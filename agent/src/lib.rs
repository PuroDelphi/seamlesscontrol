//! Portable core shared by the Omarchy and Windows agents.
//! Platform capture, injection, network transport, and persistence are adapters.

pub mod clipboard;
pub mod clipboard_file;
pub mod diagnostics;
pub mod edge_policy;
pub mod file_session;
pub mod file_transfer;
pub mod handoff;
pub mod peer_policy;
pub mod protocol;
pub mod receiver;
pub mod secure;
pub mod state;
pub mod storage;
pub mod topology;
pub mod windows_keymap;

pub const PRODUCT_VERSION: &str = env!("SEAMLESSCONTROL_PRODUCT_VERSION");

#[cfg(target_os = "windows")]
pub mod windows_agent;
#[cfg(target_os = "windows")]
pub mod windows_capture;
#[cfg(target_os = "windows")]
pub mod windows_clipboard;
#[cfg(target_os = "windows")]
pub mod windows_discovery;
#[cfg(target_os = "windows")]
pub mod windows_input;

#[cfg(target_os = "linux")]
pub mod clipboard_omarchy;
#[cfg(target_os = "linux")]
pub mod control;
#[cfg(target_os = "linux")]
pub mod discovery;
#[cfg(target_os = "linux")]
pub mod hypr_ipc;
#[cfg(target_os = "linux")]
pub mod omarchy;
