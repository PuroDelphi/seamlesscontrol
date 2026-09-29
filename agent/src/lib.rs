//! Portable core shared by the Omarchy and future Windows agents.
//! Platform capture, injection, network transport, and persistence are adapters.

pub mod protocol;
pub mod receiver;
pub mod secure;
pub mod state;
pub mod storage;
pub mod topology;

#[cfg(target_os = "linux")]
pub mod omarchy;
