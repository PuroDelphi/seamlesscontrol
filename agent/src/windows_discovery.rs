//! Windows DNS-SD hints compatible with the Omarchy Avahi advertisement.
//! Discovery is never an authorization step: Noise identity pinning and the
//! six-digit pairing code remain required before input or files are accepted.

use crate::secure::Identity;
use crate::storage::key_fingerprint;
use mdns_sd::{DaemonEvent, ServiceDaemon, ServiceEvent, ServiceInfo};
use std::collections::BTreeMap;
use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

pub const SERVICE_TYPE: &str = "_seamlesscontrol._tcp.local.";
const PROTOCOL: &str = "5";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscoveredServer {
    pub name: String,
    pub address: SocketAddr,
    pub fingerprint: String,
}

fn lan_ipv4(ip: Ipv4Addr) -> bool {
    !ip.is_loopback() && (ip.is_private() || ip.is_link_local())
}

fn route_address(port: u16) -> io::Result<SocketAddr> {
    let probe = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?;
    probe.connect((Ipv4Addr::new(224, 0, 0, 251), 5353))?;
    let IpAddr::V4(ip) = probe.local_addr()?.ip() else {
        return Err(io::Error::other("mDNS route is not IPv4"));
    };
    if !lan_ipv4(ip) {
        return Err(io::Error::other("no private IPv4 LAN route for discovery"));
    }
    Ok(SocketAddr::new(IpAddr::V4(ip), port))
}

pub struct ServiceAdvertisement(ServiceDaemon);

impl ServiceAdvertisement {
    pub fn publish(bound: SocketAddr, identity: &Identity) -> io::Result<Self> {
        let address = if bound.ip().is_unspecified() {
            route_address(bound.port())?
        } else {
            bound
        };
        let IpAddr::V4(ip) = address.ip() else {
            return Err(io::Error::other("only IPv4 LAN discovery is supported"));
        };
        if !lan_ipv4(ip) {
            return Err(io::Error::other(
                "receiver is not bound to a private LAN address",
            ));
        }
        let fingerprint = key_fingerprint(&identity.public);
        let hostname = std::env::var("COMPUTERNAME")
            .unwrap_or_else(|_| "Windows".to_owned())
            .chars()
            .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '-')
            .take(25)
            .collect::<String>();
        let hostname = if hostname.is_empty() {
            "Windows"
        } else {
            &hostname
        };
        let instance = format!("SeamlessControl {hostname} {}", &fingerprint[..6]);
        let host = format!("seamlesscontrol-{}.local.", &fingerprint[..12]);
        let address_text = ip.to_string();
        let properties = [
            ("protocol", PROTOCOL),
            ("fingerprint", fingerprint.as_str()),
            ("address", address_text.as_str()),
        ];
        let info = ServiceInfo::new(
            SERVICE_TYPE,
            &instance,
            &host,
            IpAddr::V4(ip),
            address.port(),
            &properties[..],
        )
        .map_err(io::Error::other)?;
        let daemon = ServiceDaemon::new().map_err(io::Error::other)?;
        let events = daemon.monitor().map_err(io::Error::other)?;
        daemon.register(info).map_err(io::Error::other)?;
        let deadline = Instant::now() + Duration::from_secs(5);
        let expected_interface = ip.to_string();
        let mut diagnostic = String::new();
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                break;
            }
            match events.recv_timeout(remaining) {
                Ok(DaemonEvent::Announce(_, interfaces)) => {
                    if interfaces
                        .trim_matches(['[', ']'])
                        .split(',')
                        .any(|entry| entry.trim() == expected_interface)
                    {
                        println!("mDNS announcement sent on {ip}:{}.", address.port());
                        return Ok(Self(daemon));
                    }
                    diagnostic = format!("mDNS announced on {interfaces}, not {ip}");
                }
                Ok(DaemonEvent::Error(error)) => diagnostic = error.to_string(),
                Ok(_) => {}
                Err(error) => {
                    if diagnostic.is_empty() {
                        diagnostic = error.to_string();
                    }
                    break;
                }
            }
        }
        if diagnostic.is_empty() {
            diagnostic = format!("no mDNS announcement from LAN address {ip}");
        }
        let _ = daemon.shutdown();
        Err(io::Error::other(diagnostic))
    }
}

impl Drop for ServiceAdvertisement {
    fn drop(&mut self) {
        let _ = self.0.shutdown();
    }
}

pub struct DiscoveryBrowser {
    running: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl DiscoveryBrowser {
    pub fn start(
        own_fingerprint: String,
        on_change: impl Fn(Vec<DiscoveredServer>) + Send + 'static,
    ) -> io::Result<Self> {
        let daemon = ServiceDaemon::new().map_err(io::Error::other)?;
        let receiver = daemon.browse(SERVICE_TYPE).map_err(io::Error::other)?;
        let running = Arc::new(AtomicBool::new(true));
        let flag = Arc::clone(&running);
        let thread = thread::spawn(move || {
            let mut found = BTreeMap::<String, DiscoveredServer>::new();
            while flag.load(Ordering::Acquire) {
                match receiver.recv_timeout(Duration::from_secs(1)) {
                    Ok(ServiceEvent::ServiceResolved(info)) => {
                        if info.get_property_val_str("protocol") != Some(PROTOCOL) {
                            continue;
                        }
                        let Some(fingerprint) = info.get_property_val_str("fingerprint") else {
                            continue;
                        };
                        if fingerprint == own_fingerprint
                            || fingerprint.len() != 64
                            || !fingerprint
                                .bytes()
                                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
                            || info.get_port() == 0
                        {
                            continue;
                        }
                        let address = info
                            .get_property_val_str("address")
                            .and_then(|value| value.parse::<Ipv4Addr>().ok())
                            .filter(|ip| lan_ipv4(*ip))
                            .or_else(|| {
                                info.get_addresses_v4().into_iter().find(|ip| lan_ipv4(*ip))
                            });
                        let Some(ip) = address else {
                            continue;
                        };
                        let fullname = info.get_fullname().to_owned();
                        let name = fullname
                            .strip_suffix(SERVICE_TYPE)
                            .unwrap_or(&fullname)
                            .trim_end_matches('.')
                            .trim()
                            .to_owned();
                        if name.is_empty() || name.len() > 63 || name.chars().any(char::is_control)
                        {
                            continue;
                        }
                        found.insert(
                            fullname,
                            DiscoveredServer {
                                name,
                                address: SocketAddr::new(IpAddr::V4(ip), info.get_port()),
                                fingerprint: fingerprint.to_owned(),
                            },
                        );
                        on_change(found.values().cloned().collect());
                    }
                    Ok(ServiceEvent::ServiceRemoved(_, fullname)) => {
                        if found.remove(&fullname).is_some() {
                            on_change(found.values().cloned().collect());
                        }
                    }
                    Ok(_) | Err(_) => {}
                }
            }
            let _ = daemon.shutdown();
        });
        Ok(Self {
            running,
            thread: Some(thread),
        })
    }
}

impl Drop for DiscoveryBrowser {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
