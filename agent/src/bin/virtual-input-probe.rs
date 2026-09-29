#[cfg(target_os = "linux")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _devices = seamlesscontrol_core::omarchy::VirtualInput::connect()?;
    println!("Hyprland aceptó teclado y puntero virtuales; no se enviaron eventos.");
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("Este probe sólo funciona en Omarchy/Linux.");
}
