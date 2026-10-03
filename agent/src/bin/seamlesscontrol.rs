#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

#[cfg(target_os = "windows")]
#[path = "../windows_ui.rs"]
mod windows_ui;

#[cfg(target_os = "windows")]
fn main() {
    windows_ui::run();
}

#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("The SeamlessControl desktop app currently runs on Windows.");
}
