//! Exercise the normal authenticated receiver on this Hyprland session without
//! opening InputCapture. Only a four-pixel pointer movement is sent and undone.

#[cfg(target_os = "linux")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use seamlesscontrol_core::hypr_ipc::{HyprIpc, SessionLockState};
    use seamlesscontrol_core::protocol::{Frame, Kind};
    use seamlesscontrol_core::secure::{Role, SecureChannel};
    use seamlesscontrol_core::state::InputEvent;
    use seamlesscontrol_core::storage::{load_or_create_identity, load_peer_key};
    use std::net::{SocketAddr, TcpStream};
    use std::path::Path;
    use std::thread;
    use std::time::Duration;

    let mut args = std::env::args().skip(1);
    let address: SocketAddr = args.next().ok_or("missing loopback address")?.parse()?;
    let config = args.next().ok_or("missing temporary client config")?;
    if args.next().is_some() || !address.ip().is_loopback() {
        return Err("usage: injection_local <127.0.0.1:port> <temporary-client-config>".into());
    }
    let config = Path::new(&config);
    let identity = load_or_create_identity(&config.join("identity"))?;
    let pinned = load_peer_key(&config.join("peers"), address.ip())?
        .ok_or("pair the temporary client before testing injection")?;
    let hypr = HyprIpc::from_env().ok_or("Hyprland IPC is unavailable")?;
    if hypr.session_lock_state()? != SessionLockState::Unlocked {
        return Err("Hyprland is locked or its lock state is unknown".into());
    }
    let before = hypr.cursor_position()?;

    let stream = TcpStream::connect_timeout(&address, Duration::from_secs(5))?;
    stream.set_nodelay(true)?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    let (mut channel, _) =
        SecureChannel::connect(stream, Role::Initiator, &identity, Some(&pinned), |_| false)?;
    Frame {
        kind: Kind::Hello,
        epoch: 0,
        sequence: 0,
        payload: b"seamlesscontrol/5".to_vec(),
    }
    .write_to(&mut channel)?;
    let hello = Frame::read_from(&mut channel)?;
    if hello.kind != Kind::Hello || hello.payload != b"seamlesscontrol/5" {
        return Err("receiver protocol mismatch".into());
    }
    Frame {
        kind: Kind::Control,
        epoch: 0,
        sequence: 0,
        payload: b"CLAIM".to_vec(),
    }
    .write_to(&mut channel)?;
    let reply = Frame::read_from(&mut channel)?;
    if reply.kind != Kind::Control || reply.payload != b"READY" {
        return Err("receiver did not grant input ownership".into());
    }

    let epoch = 1;
    Frame {
        kind: Kind::Control,
        epoch,
        sequence: 1,
        payload: b"BEGIN".to_vec(),
    }
    .write_to(&mut channel)?;
    Frame {
        kind: Kind::Input,
        epoch,
        sequence: 2,
        payload: InputEvent::Motion {
            dx_milli: 4_000,
            dy_milli: 0,
        }
        .encode(),
    }
    .write_to(&mut channel)?;
    thread::sleep(Duration::from_millis(300));
    let shifted = hypr.cursor_position();
    Frame {
        kind: Kind::Input,
        epoch,
        sequence: 3,
        payload: InputEvent::Motion {
            dx_milli: -4_000,
            dy_milli: 0,
        }
        .encode(),
    }
    .write_to(&mut channel)?;
    Frame {
        kind: Kind::Control,
        epoch,
        sequence: 4,
        payload: b"END".to_vec(),
    }
    .write_to(&mut channel)?;
    thread::sleep(Duration::from_millis(300));
    let restored = hypr.cursor_position()?;
    let shifted = shifted?;
    println!(
        "INJECTION\tbefore={},{}\tshifted={},{}\trestored={},{}",
        before.0, before.1, shifted.0, shifted.1, restored.0, restored.1
    );
    if shifted != (before.0 + 4, before.1) || restored != before {
        return Err("authenticated receiver did not move and restore the cursor".into());
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("This example requires Omarchy/Linux.");
}
