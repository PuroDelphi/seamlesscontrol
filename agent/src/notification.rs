//! Show an actionable desktop notification without putting file metadata in argv.

use futures_util::StreamExt;
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use std::io::{self, BufRead, Read, Write};

const MAX_REQUEST_BYTES: u64 = 4096;

struct FileOfferNotification {
    summary: String,
    body: String,
    accept: String,
    reject: String,
}

fn field(value: &Value, key: &str, max_chars: usize) -> io::Result<String> {
    let text = value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing notification field"))?;
    if text.is_empty() || text.chars().count() > max_chars {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "notification field outside size limit",
        ));
    }
    Ok(text.to_owned())
}

fn escape_markup(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            '\n' | '\r' => escaped.push(' '),
            _ => escaped.push(character),
        }
    }
    escaped
}

fn parse_request(data: &[u8]) -> io::Result<FileOfferNotification> {
    let value: Value = serde_json::from_slice(data)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid notification request"))?;
    Ok(FileOfferNotification {
        summary: escape_markup(&field(&value, "summary", 120)?),
        body: escape_markup(&field(&value, "body", 1024)?),
        accept: escape_markup(&field(&value, "accept", 48)?),
        reject: escape_markup(&field(&value, "reject", 48)?),
    })
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
                &request.summary,
                &request.body,
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
    fn notification_request_escapes_untrusted_file_metadata() {
        let request = parse_request(
            br#"{"summary":"Incoming file","body":"<img src='x'> & a\nname.txt","accept":"Accept","reject":"Reject"}"#,
        )
        .unwrap();
        assert_eq!(request.body, "&lt;img src=&#39;x&#39;&gt; &amp; a name.txt");
        assert!(parse_request(br#"{"summary":"x","body":"","accept":"x","reject":"y"}"#).is_err());
    }
}
