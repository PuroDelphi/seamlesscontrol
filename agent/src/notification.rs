//! Show an actionable, metadata-free desktop notification.

use futures_util::StreamExt;
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use std::io::{self, BufRead, Read, Write};

const MAX_REQUEST_BYTES: u64 = 64;

struct FileOfferNotification {
    summary: &'static str,
    body: &'static str,
    accept: &'static str,
    reject: &'static str,
}

fn parse_request(data: &[u8]) -> io::Result<FileOfferNotification> {
    let value: Value = serde_json::from_slice(data)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid notification request"))?;
    let fields = value.as_object().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "invalid notification request")
    })?;
    if fields.len() != 1 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "unexpected notification field",
        ));
    }
    match fields.get("language").and_then(Value::as_str) {
        Some("en") => Ok(FileOfferNotification {
            summary: "Incoming file from another computer",
            body: "Review its name, sender and size in SeamlessControl before accepting.",
            accept: "Accept",
            reject: "Reject",
        }),
        Some("es") => Ok(FileOfferNotification {
            summary: "Archivo entrante desde otro equipo",
            body: "Revise su nombre, remitente y tamaño en SeamlessControl antes de aceptar.",
            accept: "Aceptar",
            reject: "Rechazar",
        }),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "unsupported notification language",
        )),
    }
}

fn read_request() -> io::Result<FileOfferNotification> {
    let stdin = io::stdin();
    let mut input = Vec::new();
    stdin
        .lock()
        .take(MAX_REQUEST_BYTES + 1)
        .read_until(b'\n', &mut input)?;
    if input.len() as u64 > MAX_REQUEST_BYTES || !input.ends_with(b"\n") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "notification request is incomplete or too large",
        ));
    }
    parse_request(&input)
}

pub async fn show_file_offer_from_stdin() -> Result<(), Box<dyn Error>> {
    let request = read_request()?;
    let connection = zbus::Connection::session().await?;
    let proxy = zbus::Proxy::new(
        &connection,
        "org.freedesktop.Notifications",
        "/org/freedesktop/Notifications",
        "org.freedesktop.Notifications",
    )
    .await?;
    // Subscribe before creating the notification so an immediate click is not lost.
    let mut actions = proxy.receive_signal("ActionInvoked").await?;
    let mut hints = HashMap::new();
    hints.insert("urgency", zbus::zvariant::Value::from(2u8));
    let buttons = vec!["accept", &request.accept, "reject", &request.reject];
    let id: u32 = proxy
        .call(
            "Notify",
            &(
                "SeamlessControl",
                0u32,
                "",
                request.summary,
                request.body,
                &buttons,
                &hints,
                120_000i32,
            ),
        )
        .await?;
    while let Some(signal) = actions.next().await {
        let (signal_id, action): (u32, String) = signal.body().deserialize()?;
        if signal_id == id && (action == "accept" || action == "reject") {
            println!("{action}");
            io::stdout().flush()?;
            break;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notification_request_accepts_only_a_fixed_language() {
        let english = parse_request(br#"{"language":"en"}"#).unwrap();
        let spanish = parse_request(br#"{"language":"es"}"#).unwrap();
        assert_eq!(english.accept, "Accept");
        assert_eq!(spanish.reject, "Rechazar");
        assert!(parse_request(br#"{"language":"en","body":"private-file.txt"}"#).is_err());
        assert!(parse_request(br#"{"language":"fr"}"#).is_err());
        assert!(parse_request(br#"{"language":123}"#).is_err());
    }
}
