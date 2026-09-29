#[cfg(target_os = "linux")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut devices = seamlesscontrol_core::omarchy::VirtualInput::connect()?;
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--edge-test") {
        devices.motion(2000.0, 0.0, 0)?;
        devices.sync()?;
        std::thread::sleep(std::time::Duration::from_millis(250));
        devices.motion(10.0, 0.0, 1)?;
        println!("Se envió movimiento hacia el borde derecho; no se enviaron teclas ni clics.");
    } else if args.len() == 4 && args[1] == "--motion" {
        let dx: f64 = args[2].parse()?;
        let dy: f64 = args[3].parse()?;
        devices.motion(dx, dy, 0)?;
        println!("Se envió movimiento relativo ({dx}, {dy}).");
    } else {
        println!("Hyprland aceptó teclado y puntero virtuales; no se enviaron eventos.");
    }
    devices.sync()?;
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("Este probe sólo funciona en Omarchy/Linux.");
}
