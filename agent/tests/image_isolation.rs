//! Exercise the actual agent subprocess boundary on Linux and Windows.

use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn probe(mime: &str, bytes: &[u8], stall_decoder: bool) -> (bool, Vec<u8>, Duration) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_seamlesscontrold"));
    command
        .arg("__clipboard-image-probe")
        .arg(mime)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    if stall_decoder {
        command.env("SEAMLESSCONTROL_TEST_DECODER_STALL", "1");
    }
    let mut child = command.spawn().expect("start agent probe");
    let started = Instant::now();
    child
        .stdin
        .take()
        .expect("agent stdin")
        .write_all(bytes)
        .expect("write image");
    let deadline = started + Duration::from_secs(16);
    let status = loop {
        if let Some(status) = child.try_wait().expect("poll agent") {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("image worker held the agent past its deadline");
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let mut output = Vec::new();
    child
        .stdout
        .take()
        .expect("agent stdout")
        .read_to_end(&mut output)
        .expect("read image");
    (status.success(), output, started.elapsed())
}

#[test]
fn raster_conversion_uses_a_working_isolated_child() {
    let image = DynamicImage::ImageRgba8(RgbaImage::from_pixel(2, 2, Rgba([20, 40, 60, 255])));
    let mut bytes = std::io::Cursor::new(Vec::new());
    image
        .write_to(&mut bytes, ImageFormat::Jpeg)
        .expect("encode jpeg");
    let (success, png, _) = probe("image/jpeg", bytes.get_ref(), false);
    assert!(success, "the limited image child must accept a small JPEG");
    assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
}

#[test]
fn inherited_dash_array_is_rejected_before_scene_expansion() {
    let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="8" height="8"><g stroke-dasharray="1,2"><path d="M0 0h8"/></g></svg>"#;
    let (success, png, elapsed) = probe("image/svg+xml", svg, false);
    assert!(!success);
    assert!(png.is_empty());
    assert!(elapsed < Duration::from_secs(5));
}

#[test]
fn ordinary_vector_svg_still_converts_in_isolated_child() {
    let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="8" height="8"><rect width="8" height="8" fill="green"/></svg>"#;
    let (success, png, _) = probe("image/svg+xml", svg, false);
    assert!(success);
    assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
}

#[test]
fn stalled_decoder_is_killed_without_stalling_agent() {
    let image = DynamicImage::ImageRgba8(RgbaImage::from_pixel(1, 1, Rgba([1, 2, 3, 255])));
    let mut bytes = std::io::Cursor::new(Vec::new());
    image
        .write_to(&mut bytes, ImageFormat::Jpeg)
        .expect("encode jpeg");
    let (success, png, elapsed) = probe("image/jpeg", bytes.get_ref(), true);
    assert!(!success);
    assert!(png.is_empty());
    assert!(elapsed >= Duration::from_secs(10));
    assert!(elapsed < Duration::from_secs(15));
}
