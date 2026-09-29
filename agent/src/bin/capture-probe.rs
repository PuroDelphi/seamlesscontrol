#[cfg(target_os = "linux")]
use ashpd::desktop::input_capture::{
    Barrier, BarrierID, BarrierPosition, Capabilities, CreateSessionOptions, InputCapture,
};

#[cfg(target_os = "linux")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let result = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        let portal = InputCapture::new().await?;
        println!("InputCapture portal v{}", portal.version());
        let (session, capabilities) = portal
            .create_session(
                None,
                CreateSessionOptions::default()
                    .set_capabilities(Capabilities::Keyboard | Capabilities::Pointer),
            )
            .await?;
        println!("Capacidades concedidas: {capabilities}");
        let zones = portal
            .zones(&session, Default::default())
            .await?
            .response()?;
        let region = zones
            .regions()
            .first()
            .ok_or("el portal no anunció monitores")?;
        let x = region.x_offset() + region.width() as i32;
        let y = region.y_offset();
        let id = BarrierID::new(1).ok_or("ID de barrera inválido")?;
        let barrier = Barrier::new(
            id,
            BarrierPosition::new(x, y, x, y + region.height() as i32 - 1),
        );
        let response = portal
            .set_pointer_barriers(&session, &[barrier], zones.zone_set(), Default::default())
            .await?
            .response()?;
        if !response.failed_barriers().is_empty() {
            return Err("Hyprland rechazó la barrera del borde derecho".into());
        }
        let _eis = portal.connect_to_eis(&session, Default::default()).await?;
        println!("Barrera derecha aceptada y conexión EIS abierta. No se capturaron eventos.");
        Ok::<(), Box<dyn std::error::Error>>(())
    })
    .await;
    result.map_err(|_| -> Box<dyn std::error::Error> {
        "El portal no respondió en 30 segundos; quizá espere consentimiento".into()
    })?
}

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("Este probe sólo funciona en Omarchy/Linux.");
}
