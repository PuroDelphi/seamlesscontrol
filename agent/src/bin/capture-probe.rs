#[cfg(target_os = "linux")]
use ashpd::desktop::input_capture::{
    Barrier, BarrierID, BarrierPosition, Capabilities, CreateSessionOptions, InputCapture,
    ReleaseOptions,
};
#[cfg(target_os = "linux")]
use futures_util::StreamExt;
#[cfg(target_os = "linux")]
use reis::{
    ei,
    event::{DeviceCapability, EiEvent},
};
#[cfg(target_os = "linux")]
use std::os::unix::net::UnixStream;

#[cfg(target_os = "linux")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listen = std::env::args().any(|arg| arg == "--listen");
    let verify_input = std::env::args().any(|arg| arg == "--verify-input");
    let result = tokio::time::timeout(std::time::Duration::from_secs(40), async {
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
        let eis = portal.connect_to_eis(&session, Default::default()).await?;
        println!("Barrera derecha aceptada y conexión EIS abierta.");
        if listen {
            let stream = UnixStream::from(eis);
            stream.set_nonblocking(true)?;
            let context = ei::Context::new(stream)?;
            context.flush()?;
            let (_connection, mut events) = context
                .handshake_tokio(
                    "seamlesscontrol-probe",
                    ei::handshake::ContextType::Receiver,
                )
                .await?;
            tokio::time::timeout(std::time::Duration::from_secs(5), async {
                loop {
                    match events.next().await {
                        Some(Ok(EiEvent::SeatAdded(seat))) => {
                            println!("EIS anunció un asiento antes de habilitar la captura.");
                            seat.seat.bind_capabilities(
                                DeviceCapability::Pointer
                                    | DeviceCapability::PointerAbsolute
                                    | DeviceCapability::Keyboard
                                    | DeviceCapability::Scroll
                                    | DeviceCapability::Button,
                            );
                            context.flush()?;
                            break Ok::<_, Box<dyn std::error::Error>>(());
                        }
                        Some(Ok(_)) => {}
                        Some(Err(error)) => break Err(error.into()),
                        None => break Err("EIS terminó antes de anunciar un asiento".into()),
                    }
                }
            })
            .await
            .map_err(|_| "EIS no anunció un asiento en cinco segundos")??;
            let mut activations = portal.receive_activated().await?;
            portal.enable(&session, Default::default()).await?;
            println!(
                "Captura habilitada durante 20 s. Mueva el puntero al borde derecho para probarla."
            );
            // Bind any newly announced seat as well. The first seat was bound
            // before enabling capture so the first real input can be delivered.
            let activation = tokio::time::timeout(std::time::Duration::from_secs(20), async {
                loop {
                    tokio::select! {
                        signal = activations.next() => {
                            return Ok::<_, Box<dyn std::error::Error>>(signal);
                        }
                        event = events.next() => {
                            match event {
                                Some(Ok(EiEvent::SeatAdded(seat))) => {
                                    println!("EIS anunció un asiento.");
                                    seat.seat.bind_capabilities(
                                        DeviceCapability::Pointer
                                            | DeviceCapability::PointerAbsolute
                                            | DeviceCapability::Keyboard
                                            | DeviceCapability::Scroll
                                            | DeviceCapability::Button,
                                    );
                                    context.flush()?;
                                }
                                Some(Ok(EiEvent::DeviceAdded(_))) => println!("EIS anunció un dispositivo."),
                                Some(Ok(EiEvent::DeviceStartEmulating(_))) => println!("EIS inició la captura de un dispositivo."),
                                Some(Ok(EiEvent::DevicePaused(_))) => println!("EIS pausó un dispositivo."),
                                Some(Ok(_)) => {}
                                Some(Err(error)) => return Err(error.into()),
                                None => return Err("EIS terminó antes de activar la barrera".into()),
                            }
                        }
                    }
                }
            })
            .await;
            let activation = match activation {
                Ok(Ok(Some(signal))) => Some(signal),
                Ok(Ok(None)) => return Err("el portal cerró la señal de activación".into()),
                Ok(Err(error)) => return Err(error),
                Err(_) => None,
            };
            if let Some(activation) = activation {
                let x = activation
                    .cursor_position()
                    .map(|pos| pos.0 as f64 - 1.0)
                    .unwrap_or(x as f64 - 1.0);
                let y = activation
                    .cursor_position()
                    .map(|pos| pos.1 as f64)
                    .unwrap_or(y as f64);
                println!("Barrera activada. Leyendo eventos EIS...");
                let mut saw_keyboard = false;
                let mut saw_button = false;
                let read_window = if verify_input { 8 } else { 5 };
                let read = tokio::time::timeout(std::time::Duration::from_secs(read_window), async {
                    while let Some(event) = events.next().await {
                        let event = event?;
                        match event {
                            EiEvent::SeatAdded(seat) => {
                                println!("EIS anunció un asiento tras la activación.");
                                seat.seat.bind_capabilities(
                                    DeviceCapability::Pointer
                                        | DeviceCapability::PointerAbsolute
                                        | DeviceCapability::Keyboard
                                        | DeviceCapability::Scroll
                                        | DeviceCapability::Button,
                                );
                                context.flush()?;
                            }
                            EiEvent::DeviceAdded(_) => println!("EIS anunció un dispositivo tras la activación."),
                            EiEvent::DeviceStartEmulating(_) => println!("EIS inició la captura de un dispositivo."),
                            EiEvent::DevicePaused(_) => println!("EIS pausó un dispositivo."),
                            EiEvent::PointerMotion(_) if !verify_input => return Ok::<_, Box<dyn std::error::Error>>(Some("movimiento relativo")),
                            EiEvent::PointerMotionAbsolute(_) if !verify_input => return Ok(Some("movimiento absoluto")),
                            EiEvent::KeyboardKey(_) => {
                                saw_keyboard = true;
                                if !verify_input { return Ok(Some("teclado")); }
                            }
                            EiEvent::Button(_) => {
                                saw_button = true;
                                if !verify_input { return Ok(Some("botón")); }
                            }
                            EiEvent::ScrollDelta(_) if !verify_input => return Ok(Some("desplazamiento")),
                            _ => {}
                        }
                        if verify_input && saw_keyboard && saw_button {
                            return Ok(Some("teclado y botón"));
                        }
                    }
                    Ok(None)
                })
                .await;
                let release = ReleaseOptions::default()
                    .set_activation_id(activation.activation_id())
                    .set_cursor_position((x, y));
                portal.release(&session, release).await?;
                match read {
                    Ok(Ok(Some(kind))) => println!("Captura liberada; EIS entregó {kind}."),
                    Ok(Ok(None)) => {
                        println!("Captura liberada; EIS terminó sin un evento de entrada.")
                    }
                    Ok(Err(error)) => eprintln!("Captura liberada; error al leer EIS: {error}"),
                    Err(_) if verify_input => println!("Captura liberada; teclado: {}, botón: {}.", saw_keyboard, saw_button),
                    Err(_) => println!("Captura liberada; no llegó un evento EIS en 5 s."),
                }
            } else {
                println!("La barrera no se activó durante la ventana de prueba.");
            }
            portal.disable(&session, Default::default()).await?;
        } else {
            println!("No se capturaron eventos. Use --listen para probar una activación temporal.");
        }
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
