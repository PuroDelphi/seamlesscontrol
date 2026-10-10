//! Read-only checks for the connection setup. A TCP response never proves peer identity.

use crate::storage::load_peer_key;
use std::io;
use std::net::{IpAddr, SocketAddr, TcpStream};
use std::path::Path;
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct PeerDiagnosis {
    pub address: SocketAddr,
    pub paired: bool,
    pub reachable: bool,
    pub reason: &'static str,
}

pub fn diagnose_peer(address: SocketAddr, peers_dir: &Path) -> io::Result<PeerDiagnosis> {
    let on_lan = match address.ip() {
        IpAddr::V4(ip) => ip.is_private() || ip.is_loopback() || ip.is_link_local(),
        IpAddr::V6(ip) => {
            ip.is_loopback() || ip.is_unicast_link_local() || ip.segments()[0] & 0xfe00 == 0xfc00
        }
    };
    if !on_lan || address.port() == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "use a private LAN IP and a nonzero port",
        ));
    }
    let paired = load_peer_key(peers_dir, address.ip())?.is_some();
    let reachable = TcpStream::connect_timeout(&address, Duration::from_secs(2)).is_ok();
    let reason = match (reachable, paired) {
        (false, _) => "control_port_unreachable",
        (true, false) => "pair_first",
        (true, true) => "port_open_paired",
    };
    Ok(PeerDiagnosis {
        address,
        paired,
        reachable,
        reason,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn distinguishes_open_port_from_trusted_identity() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let result =
            diagnose_peer(address, Path::new("/nonexistent/seamlesscontrol-peers")).unwrap();
        assert!(result.reachable);
        assert!(!result.paired);
        assert_eq!(result.reason, "pair_first");
    }

    #[test]
    fn rejects_public_addresses_without_contacting_them() {
        let address = "8.8.8.8:47832".parse().unwrap();
        assert_eq!(
            diagnose_peer(address, Path::new("/nonexistent"))
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidInput
        );
    }
}
