//! LAN hints through Avahi DNS-SD. Discovery never grants trust: Noise pairing
//! still requires the code shown and approved on both machines.

use crate::secure::Identity;
use crate::storage::key_fingerprint;
use std::collections::BTreeSet;
use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

const SERVICE_TYPE: &str = "_seamlesscontrol._tcp";
const PROTOCOL_VERSION: &str = "5";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscoveredServer {
    pub name: String,
    pub address: SocketAddr,
    pub fingerprint: String,
}

fn lan_ipv4(ip: Ipv4Addr) -> bool {
    !ip.is_loopback() && (ip.is_private() || ip.is_link_local())
}

pub fn auto_lan_address(port: u16) -> io::Result<SocketAddr> {
    if port == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "port must be nonzero",
        ));
    }
    // RFC 6762 uses 224.0.0.251:5353. Connect chooses its route without sending.
    let probe = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?;
    probe.connect((Ipv4Addr::new(224, 0, 0, 251), 5353))?;
    let IpAddr::V4(ip) = probe.local_addr()?.ip() else {
        return Err(io::Error::other("mDNS route is not IPv4"));
    };
    if !lan_ipv4(ip) {
        return Err(io::Error::new(
            io::ErrorKind::AddrNotAvailable,
            "mDNS route has no private IPv4 LAN address; enter a local IP manually",
        ));
    }
    Ok(SocketAddr::new(IpAddr::V4(ip), port))
}

pub struct ServiceAdvertisement(Child);

impl ServiceAdvertisement {
    pub fn publish(address: SocketAddr, identity: &Identity) -> io::Result<Option<Self>> {
        let IpAddr::V4(ip) = address.ip() else {
            return Ok(None);
        };
        if !lan_ipv4(ip) {
            return Ok(None);
        }
        let fingerprint = key_fingerprint(&identity.public);
        let hostname = std::env::var("HOSTNAME")
            .ok()
            .or_else(|| std::fs::read_to_string("/etc/hostname").ok())
            .unwrap_or_else(|| "equipo".to_owned());
        let hostname: String = hostname
            .trim()
            .chars()
            .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '-' || *ch == '_')
            .take(25)
            .collect();
        let name = format!(
            "SeamlessControl {} {}",
            if hostname.is_empty() {
                "equipo"
            } else {
                &hostname
            },
            &fingerprint[..6]
        );
        let mut child = Command::new("avahi-publish-service")
            .args([
                name.as_str(),
                SERVICE_TYPE,
                &address.port().to_string(),
                &format!("protocol={PROTOCOL_VERSION}"),
                &format!("fingerprint={fingerprint}"),
                &format!("address={ip}"),
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        std::thread::sleep(Duration::from_millis(150));
        if let Some(status) = child.try_wait()? {
            return Err(io::Error::other(format!(
                "avahi-publish-service exited with {status}"
            )));
        }
        Ok(Some(Self(child)))
    }
}

impl Drop for ServiceAdvertisement {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn txt_value<'a>(txt: &'a str, key: &str) -> Option<&'a str> {
    txt.split('"').find_map(|part| part.strip_prefix(key))
}

fn decode_avahi_name(raw: &str) -> Option<String> {
    let mut bytes = Vec::with_capacity(raw.len());
    let mut input = raw.as_bytes().iter().copied();
    while let Some(byte) = input.next() {
        if byte == b'\\' {
            let digits = [input.next()?, input.next()?, input.next()?];
            if !digits.iter().all(u8::is_ascii_digit) {
                return None;
            }
            let value = u16::from(digits[0] - b'0') * 100
                + u16::from(digits[1] - b'0') * 10
                + u16::from(digits[2] - b'0');
            bytes.push(value.try_into().ok()?);
        } else {
            bytes.push(byte);
        }
    }
    String::from_utf8(bytes).ok()
}

pub fn parse_browse_output(output: &str) -> Vec<DiscoveredServer> {
    let mut found = Vec::new();
    let mut seen = BTreeSet::new();
    for line in output.lines() {
        let fields: Vec<&str> = line.split(';').collect();
        if fields.len() < 10
            || fields[0] != "="
            || fields[2] != "IPv4"
            || fields[4] != SERVICE_TYPE
            || txt_value(fields[9], "protocol=") != Some(PROTOCOL_VERSION)
        {
            continue;
        }
        let Some(name) = decode_avahi_name(fields[3]) else {
            continue;
        };
        let name = name.trim();
        if name.is_empty() || name.len() > 63 || name.chars().any(char::is_control) {
            continue;
        }
        let fingerprint = txt_value(fields[9], "fingerprint=").unwrap_or("");
        if fingerprint.len() != 64
            || !fingerprint
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        {
            continue;
        }
        let ip = txt_value(fields[9], "address=").unwrap_or(fields[7]);
        let Ok(ip) = ip.parse::<Ipv4Addr>() else {
            continue;
        };
        if !lan_ipv4(ip) {
            continue;
        }
        let Ok(port) = fields[8].parse::<u16>() else {
            continue;
        };
        if port == 0 {
            continue;
        }
        let address = SocketAddr::new(IpAddr::V4(ip), port);
        if seen.insert((fingerprint.to_owned(), address)) {
            found.push(DiscoveredServer {
                name: name.to_owned(),
                address,
                fingerprint: fingerprint.to_owned(),
            });
        }
    }
    found.sort_by(|a, b| a.name.cmp(&b.name).then(a.address.cmp(&b.address)));
    found
}

pub async fn browse(include_local: bool) -> io::Result<Vec<DiscoveredServer>> {
    let mut command = tokio::process::Command::new("avahi-browse");
    command.args(["--resolve", "--terminate", "--parsable"]);
    if !include_local {
        command.arg("--ignore-local");
    }
    command.arg(SERVICE_TYPE).kill_on_drop(true);
    let output = tokio::time::timeout(Duration::from_secs(5), command.output())
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "Avahi discovery timed out"))??;
    if !output.status.success() {
        return Err(io::Error::other(
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ));
    }
    Ok(parse_browse_output(&String::from_utf8_lossy(
        &output.stdout,
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browse_filters_untrusted_addresses_versions_and_duplicates() {
        let key = "a".repeat(64);
        let line = |ip: &str, version: &str, advertised: &str| {
            format!(
                "=;wlan0;IPv4;SeamlessControl\\032sala;{SERVICE_TYPE};local;sala.local;{ip};47832;\"protocol={version}\" \"fingerprint={key}\" \"address={advertised}\""
            )
        };
        let output = [
            line("192.168.50.20", "5", "192.168.50.20"),
            line("172.17.0.1", "5", "192.168.50.20"),
            line("192.168.50.30", "4", "192.168.50.30"),
            line("192.168.50.40", "5", "8.8.8.8"),
            line("192.168.50.50", "5", "127.0.0.1"),
        ]
        .join("\n");
        assert_eq!(
            parse_browse_output(&output),
            vec![DiscoveredServer {
                name: "SeamlessControl sala".to_owned(),
                address: "192.168.50.20:47832".parse().unwrap(),
                fingerprint: key,
            }]
        );
    }
}
