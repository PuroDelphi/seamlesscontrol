//! Convert common raster clipboard representations to the PNG wire format.

use crate::clipboard::{MAX_IMAGE_BYTES, valid_png_header};
use image::{ImageFormat, ImageReader, Limits};
use std::io::{self, Cursor, Read, Seek, SeekFrom, Write};
#[cfg(all(not(test), target_os = "windows"))]
use std::process::Child;
#[cfg(not(test))]
use std::process::{Command, Stdio};
use std::time::Duration;
#[cfg(not(test))]
use std::time::Instant;

// This covers SVG pixmaps kept live while resvg recursively renders isolated
// groups. The final output already uses up to 32 million RGBA pixels.
const MAX_SVG_LIVE_RGBA_BYTES: u64 = 128 * 1024 * 1024;
const MAX_SVG_TOTAL_LAYER_BYTES: u64 = 256 * 1024 * 1024;
const MAX_SVG_SOURCE_BYTES: usize = 2 * 1024 * 1024;
const MAX_SVG_NODES: usize = 4096;
const MAX_SVG_DEPTH: usize = 64;
const MAX_SVG_TEXT_BYTES: usize = 16 * 1024;
const MAX_SVG_GEOMETRY_BYTES: usize = 256 * 1024;
const MAX_SVG_ATTRIBUTE_BYTES: usize = 512 * 1024;
// Native SVG and raster parsers can allocate before their own output budgets
// run. Keep all untrusted conversion outside the input-control process.
#[cfg(not(test))]
const DECODER_MEMORY_BYTES: u64 = 768 * 1024 * 1024;
#[cfg(not(test))]
const DECODER_WALL_TIME: Duration = Duration::from_secs(10);
pub const DECODER_COMMAND: &str = "__clipboard-image-decode";
pub const DECODER_PROBE_COMMAND: &str = "__clipboard-image-probe";

/// Decode a foreign image in a separate, resource-limited agent process.
/// A failed or stalled decoder simply leaves this clipboard update unsent.
pub fn isolated_to_wire_png_for_mime(mime: &str, bytes: &[u8]) -> Option<Vec<u8>> {
    if bytes.is_empty() || bytes.len() > MAX_IMAGE_BYTES || mime.len() > 128 {
        return None;
    }
    // Unit-test executables are not the agent command dispatcher. Integration
    // tests exercise the actual helper binary and its process limits.
    #[cfg(test)]
    return to_wire_png_for_mime(mime, bytes);
    #[cfg(not(test))]
    isolated_to_wire_png_for_mime_process(mime, bytes)
}

#[cfg(not(test))]
fn isolated_to_wire_png_for_mime_process(mime: &str, bytes: &[u8]) -> Option<Vec<u8>> {
    // PNG is already the wire representation; no native decoder runs here.
    if mime.eq_ignore_ascii_case("image/png") {
        return valid_png_header(bytes).then(|| bytes.to_vec());
    }
    let mut command = Command::new(std::env::current_exe().ok()?);
    command
        .arg(DECODER_COMMAND)
        .arg(mime)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(target_os = "linux")]
    limit_linux_decoder(&mut command);
    let mut child = command.spawn().ok()?;
    #[cfg(target_os = "windows")]
    let _job = match limit_windows_decoder(&child) {
        Some(job) => job,
        None => {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
    };
    let mut stdin = child.stdin.take()?;
    let stdout = child.stdout.take()?;
    let (status, written, output) = std::thread::scope(|scope| {
        let writer = scope.spawn(move || stdin.write_all(bytes));
        let reader = scope.spawn(move || {
            let mut output = Vec::new();
            stdout
                .take((MAX_IMAGE_BYTES + 1) as u64)
                .read_to_end(&mut output)
                .map(|_| output)
        });
        let deadline = Instant::now() + DECODER_WALL_TIME;
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break Some(status),
                Ok(None) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                _ => {
                    let _ = child.kill();
                    let _ = child.wait();
                    break None;
                }
            }
        };
        (status, writer.join(), reader.join())
    });
    let output = output.ok()?.ok()?;
    if !status?.success() || written.ok()?.is_err() || !valid_png_header(&output) {
        return None;
    }
    Some(output)
}

/// The private child command reads only bounded stdin and writes one PNG.
pub fn decode_image_from_stdin(mime: &str) -> io::Result<()> {
    image_from_stdin(mime, to_wire_png_for_mime)
}

/// End-to-end diagnostic path used to test the actual isolation boundary.
pub fn probe_isolated_image_from_stdin(mime: &str) -> io::Result<()> {
    image_from_stdin(mime, isolated_to_wire_png_for_mime)
}

fn image_from_stdin(mime: &str, convert: fn(&str, &[u8]) -> Option<Vec<u8>>) -> io::Result<()> {
    if mime.len() > 128 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid image MIME",
        ));
    }
    let mut bytes = Vec::new();
    io::stdin()
        .take((MAX_IMAGE_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_IMAGE_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "image too large",
        ));
    }
    #[cfg(debug_assertions)]
    if std::env::var_os("SEAMLESSCONTROL_TEST_DECODER_STALL").is_some()
        && std::env::args().any(|arg| arg == DECODER_COMMAND)
    {
        std::thread::sleep(Duration::from_secs(30));
    }
    let png = convert(mime, &bytes)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "image rejected"))?;
    io::stdout().write_all(&png)
}

#[cfg(all(not(test), target_os = "linux"))]
fn limit_linux_decoder(command: &mut Command) {
    use std::os::unix::process::CommandExt;
    // setrlimit is async-signal-safe in the forked child before exec.
    unsafe {
        command.pre_exec(|| {
            let limit = libc::rlimit {
                rlim_cur: DECODER_MEMORY_BYTES,
                rlim_max: DECODER_MEMORY_BYTES,
            };
            if libc::setrlimit(libc::RLIMIT_AS, &limit) != 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
}

#[cfg(all(not(test), target_os = "windows"))]
struct DecoderJob(windows_sys::Win32::Foundation::HANDLE);

#[cfg(all(not(test), target_os = "windows"))]
impl Drop for DecoderJob {
    fn drop(&mut self) {
        unsafe { windows_sys::Win32::Foundation::CloseHandle(self.0) };
    }
}

#[cfg(all(not(test), target_os = "windows"))]
fn limit_windows_decoder(child: &Child) -> Option<DecoderJob> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        JOB_OBJECT_LIMIT_PROCESS_MEMORY, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JobObjectExtendedLimitInformation, SetInformationJobObject,
    };
    let handle = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
    if handle.is_null() {
        return None;
    }
    let job = DecoderJob(handle);
    let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
    limits.BasicLimitInformation.LimitFlags =
        JOB_OBJECT_LIMIT_PROCESS_MEMORY | JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
    limits.ProcessMemoryLimit = DECODER_MEMORY_BYTES as usize;
    let configured = unsafe {
        SetInformationJobObject(
            handle,
            JobObjectExtendedLimitInformation,
            (&raw const limits).cast(),
            std::mem::size_of_val(&limits) as u32,
        )
    };
    if configured == 0 || unsafe { AssignProcessToJobObject(handle, child.as_raw_handle()) } == 0 {
        return None;
    }
    Some(job)
}

struct LimitedCursor(Cursor<Vec<u8>>);

impl Write for LimitedCursor {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.0.position().saturating_add(bytes.len() as u64) > MAX_IMAGE_BYTES as u64 {
            return Err(io::Error::other("clipboard image exceeds the size limit"));
        }
        self.0.write(bytes)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

impl Seek for LimitedCursor {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        let next = self.0.seek(position)?;
        if next > MAX_IMAGE_BYTES as u64 {
            return Err(io::Error::other("clipboard image exceeds the size limit"));
        }
        Ok(next)
    }
}

pub fn format_for_mime(mime: &str) -> Option<ImageFormat> {
    match mime.to_ascii_lowercase().as_str() {
        "image/png" => Some(ImageFormat::Png),
        "image/jpeg" | "image/jpg" | "image/pjpeg" => Some(ImageFormat::Jpeg),
        "image/gif" => Some(ImageFormat::Gif),
        "image/webp" => Some(ImageFormat::WebP),
        "image/tiff" | "image/x-tiff" => Some(ImageFormat::Tiff),
        "image/bmp" | "image/x-bmp" | "image/x-ms-bmp" => Some(ImageFormat::Bmp),
        "image/x-icon" | "image/vnd.microsoft.icon" => Some(ImageFormat::Ico),
        "image/vnd.ms-dds" | "image/x-dds" => Some(ImageFormat::Dds),
        "image/x-exr" | "image/openexr" => Some(ImageFormat::OpenExr),
        "image/vnd.radiance" | "image/x-hdr" => Some(ImageFormat::Hdr),
        "image/x-portable-anymap"
        | "image/x-portable-bitmap"
        | "image/x-portable-graymap"
        | "image/x-portable-pixmap" => Some(ImageFormat::Pnm),
        "image/x-qoi" => Some(ImageFormat::Qoi),
        "image/x-tga" | "image/tga" => Some(ImageFormat::Tga),
        "image/farbfeld" | "image/x-farbfeld" => Some(ImageFormat::Farbfeld),
        _ => None,
    }
}

pub fn to_wire_png_for_mime(mime: &str, bytes: &[u8]) -> Option<Vec<u8>> {
    if mime.eq_ignore_ascii_case("image/svg+xml") {
        return svg_to_wire_png(bytes);
    }
    let format = format_for_mime(mime).or_else(|| {
        mime.to_ascii_lowercase()
            .starts_with("image/")
            .then(|| image::guess_format(bytes).ok())
            .flatten()
    })?;
    to_wire_png(format, bytes)
}

fn svg_to_wire_png(bytes: &[u8]) -> Option<Vec<u8>> {
    if !svg_source_within_parse_budget(bytes) {
        return None;
    }
    let mut options = resvg::usvg::Options::default();
    // SVG raster images bypass the limits of the separate raster clipboard decoder.
    // Refuse both data URLs and local paths, including images inside SVG filters.
    options.image_href_resolver.resolve_data = Box::new(|_, _, _| None);
    options.image_href_resolver.resolve_string = Box::new(|_, _| None);
    let tree = resvg::usvg::Tree::from_data(bytes, &options).ok()?;
    let size = tree.size().to_int_size();
    let pixels = u64::from(size.width()) * u64::from(size.height());
    if size.width() > 16384 || size.height() > 16384 || pixels == 0 || pixels > 32_000_000 {
        return None;
    }
    if !svg_tree_within_render_budget(&tree, size.width(), size.height()) {
        return None;
    }
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size.width(), size.height())?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::default(),
        &mut pixmap.as_mut(),
    );
    let png = pixmap.encode_png().ok()?;
    valid_png_header(&png).then_some(png)
}

fn svg_source_within_parse_budget(bytes: &[u8]) -> bool {
    if bytes.is_empty() || bytes.len() > MAX_SVG_SOURCE_BYTES {
        return false;
    }
    let Ok(source) = std::str::from_utf8(bytes) else {
        return false;
    };
    let options = roxmltree::ParsingOptions {
        nodes_limit: (MAX_SVG_NODES * 2) as u32,
        ..Default::default()
    };
    let Ok(document) = roxmltree::Document::parse_with_options(source, options) else {
        return false;
    };
    let mut elements = 0usize;
    let mut text_bytes = 0usize;
    let mut geometry_bytes = 0usize;
    let mut attribute_bytes = 0usize;
    let mut stack = vec![(document.root_element(), 1usize)];
    while let Some((node, depth)) = stack.pop() {
        elements += 1;
        if elements > MAX_SVG_NODES || depth > MAX_SVG_DEPTH {
            return false;
        }
        // usvg expands <use>, markers, and inherited dash arrays into separate
        // owned geometry before the render-tree budget. Refuse these, CSS
        // (which could assign dash arrays), and pixmap-producing features.
        if matches!(
            node.tag_name().name(),
            "use" | "marker" | "filter" | "mask" | "clipPath" | "pattern" | "style"
        ) {
            return false;
        }
        for attr in node.attributes() {
            if matches!(attr.name(), "style" | "stroke-dasharray") {
                return false;
            }
            attribute_bytes = attribute_bytes.saturating_add(attr.value().len());
            if matches!(attr.name(), "d" | "points") {
                geometry_bytes = geometry_bytes.saturating_add(attr.value().len());
            }
        }
        if attribute_bytes > MAX_SVG_ATTRIBUTE_BYTES || geometry_bytes > MAX_SVG_GEOMETRY_BYTES {
            return false;
        }
        for child in node.children() {
            if child.is_element() {
                stack.push((child, depth + 1));
            } else if child.is_text() {
                text_bytes = text_bytes.saturating_add(child.text().unwrap_or("").len());
                if text_bytes > MAX_SVG_TEXT_BYTES {
                    return false;
                }
            }
        }
    }
    true
}

fn svg_tree_within_render_budget(tree: &resvg::usvg::Tree, width: u32, height: u32) -> bool {
    use resvg::usvg::Node;

    // Filters hold one or more full RGBA results. Clip paths, masks and
    // patterns can recursively allocate extra pixmaps, sometimes larger than
    // the output canvas. They need a separate bounded renderer before they
    // can be accepted from another computer's clipboard.
    if !tree.filters().is_empty()
        || !tree.clip_paths().is_empty()
        || !tree.masks().is_empty()
        || !tree.patterns().is_empty()
    {
        return false;
    }

    let output_bytes = u64::from(width) * u64::from(height) * 4;
    if output_bytes > MAX_SVG_LIVE_RGBA_BYTES {
        return false;
    }
    let mut nodes = 0usize;
    let mut total_layer_bytes = output_bytes;
    // Include the live output pixmap in every branch. Sibling layers are
    // released between nodes, whereas nested isolated layers stay live.
    let mut stack = vec![(tree.root(), 0usize, output_bytes)];
    while let Some((group, depth, live_bytes)) = stack.pop() {
        if depth > MAX_SVG_DEPTH {
            return false;
        }
        for node in group.children() {
            nodes += 1;
            if nodes > MAX_SVG_NODES {
                return false;
            }
            let child = match node {
                Node::Group(child) => Some(child.as_ref()),
                Node::Text(text) => Some(text.flattened()),
                Node::Path(_) => None,
                // Embedded images should already be omitted by the resolvers.
                Node::Image(_) => return false,
            };
            if let Some(child) = child {
                if !child.filters().is_empty()
                    || child.clip_path().is_some()
                    || child.mask().is_some()
                {
                    return false;
                }
                let child_bytes = if child.should_isolate() {
                    // resvg 0.48.1 expands isolated layers for antialiasing
                    // and clamps them to a rectangle five canvas lengths in
                    // each direction. Use the transformed layer bounds and
                    // round up conservatively before that clamp.
                    let bbox = child.abs_layer_bounding_box();
                    let bound = |extent: f32, canvas: u32| -> Option<u64> {
                        if !extent.is_finite() {
                            return None;
                        }
                        Some(
                            ((f64::from(extent).ceil() as u64).saturating_add(8))
                                .min(u64::from(canvas) * 5),
                        )
                    };
                    let w = match bound(bbox.width(), width) {
                        Some(w) => w,
                        None => return false,
                    };
                    let h = match bound(bbox.height(), height) {
                        Some(h) => h,
                        None => return false,
                    };
                    match w.checked_mul(h).and_then(|area| area.checked_mul(4)) {
                        Some(bytes) => bytes,
                        None => return false,
                    }
                } else {
                    0
                };
                let Some(next_live) = live_bytes.checked_add(child_bytes) else {
                    return false;
                };
                if next_live > MAX_SVG_LIVE_RGBA_BYTES {
                    return false;
                }
                let Some(next_total) = total_layer_bytes.checked_add(child_bytes) else {
                    return false;
                };
                if next_total > MAX_SVG_TOTAL_LAYER_BYTES {
                    return false;
                }
                total_layer_bytes = next_total;
                stack.push((child, depth + 1, next_live));
            }
        }
    }
    true
}

pub fn to_wire_png(format: ImageFormat, bytes: &[u8]) -> Option<Vec<u8>> {
    if bytes.is_empty() || bytes.len() > MAX_IMAGE_BYTES {
        return None;
    }
    if format == ImageFormat::Png {
        return valid_png_header(bytes).then(|| bytes.to_vec());
    }
    let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
    let mut limits = Limits::default();
    limits.max_image_width = Some(16384);
    limits.max_image_height = Some(16384);
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
    let image = reader.decode().ok()?;
    let pixels = u64::from(image.width()) * u64::from(image.height());
    if pixels == 0 || pixels > 32_000_000 {
        return None;
    }
    let mut output = LimitedCursor(Cursor::new(Vec::new()));
    // HDR, OpenEXR and Farbfeld can decode to float or 16-bit pixels. The
    // receiving clipboard expects an ordinary 8-bit PNG on every platform.
    image::DynamicImage::ImageRgba8(image.to_rgba8())
        .write_to(&mut output, ImageFormat::Png)
        .ok()?;
    let output = output.0.into_inner();
    valid_png_header(&output).then_some(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, ImageBuffer, Rgb, Rgba, RgbaImage};

    #[test]
    fn common_raster_clipboard_formats_become_png() {
        let source =
            DynamicImage::ImageRgba8(RgbaImage::from_pixel(2, 2, Rgba([40, 80, 120, 255])));
        for format in [
            ImageFormat::Jpeg,
            ImageFormat::Gif,
            ImageFormat::WebP,
            ImageFormat::Tiff,
            ImageFormat::Bmp,
            ImageFormat::Pnm,
            ImageFormat::Qoi,
            ImageFormat::Tga,
        ] {
            let mut encoded = Cursor::new(Vec::new());
            source.write_to(&mut encoded, format).unwrap();
            let png = to_wire_png(format, encoded.get_ref()).expect("supported image");
            assert!(valid_png_header(&png));
            let decoded = image::load_from_memory_with_format(&png, ImageFormat::Png).unwrap();
            assert_eq!((decoded.width(), decoded.height()), (2, 2));
        }
    }

    #[test]
    fn high_depth_clipboard_images_become_png() {
        let farbfeld = DynamicImage::ImageRgba16(ImageBuffer::from_pixel(
            2,
            2,
            Rgba([1000u16, 2000, 3000, u16::MAX]),
        ));
        let hdr = DynamicImage::ImageRgb32F(ImageBuffer::from_pixel(2, 2, Rgb([0.2f32, 0.4, 0.6])));
        for (source, format) in [
            (farbfeld, ImageFormat::Farbfeld),
            (hdr.clone(), ImageFormat::Hdr),
            (hdr, ImageFormat::OpenExr),
        ] {
            let mut encoded = Cursor::new(Vec::new());
            source.write_to(&mut encoded, format).unwrap();
            assert!(to_wire_png(format, encoded.get_ref()).is_some());
        }
    }

    #[test]
    fn mime_names_map_to_expected_decoders() {
        assert_eq!(format_for_mime("image/jpg"), Some(ImageFormat::Jpeg));
        assert_eq!(format_for_mime("image/x-ms-bmp"), Some(ImageFormat::Bmp));
        assert_eq!(format_for_mime("image/svg+xml"), None);
        assert_eq!(format_for_mime("image/x-qoi"), Some(ImageFormat::Qoi));
        assert_eq!(format_for_mime("image/x-dds"), Some(ImageFormat::Dds));
    }

    #[test]
    fn svg_clipboard_renders_without_external_resources() {
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="2" height="2"><rect width="2" height="2" fill="red"/></svg>"#;
        let png = to_wire_png_for_mime("image/svg+xml", svg).expect("SVG render");
        assert!(valid_png_header(&png));
        let decoded = image::load_from_memory_with_format(&png, ImageFormat::Png).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (2, 2));
    }

    #[test]
    fn svg_clipboard_ignores_embedded_webp_and_keeps_vector_shapes() {
        let mut webp = Cursor::new(Vec::new());
        image::DynamicImage::new_rgba8(1, 1)
            .write_to(&mut webp, ImageFormat::WebP)
            .unwrap();
        let encoded =
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, webp.get_ref());
        let svg = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"2\" height=\"2\"><rect width=\"2\" height=\"2\" fill=\"green\"/><image width=\"2\" height=\"2\" href=\"data:image/webp;base64,{encoded}\"/></svg>"
        );
        let png = to_wire_png_for_mime("image/svg+xml", svg.as_bytes()).expect("SVG render");
        let decoded = image::load_from_memory_with_format(&png, ImageFormat::Png).unwrap();
        assert_eq!(decoded.to_rgba8().get_pixel(0, 0).0, [0, 128, 0, 255]);
    }

    #[test]
    fn svg_clipboard_rejects_filter_surfaces_before_rendering() {
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="16384" height="1953"><defs><filter id="f"><feGaussianBlur stdDeviation="1"/><feColorMatrix values="1 0 0 0 0 0 1 0 0 0 0 0 1 0 0 0 0 0 1 0"/></filter></defs><rect width="10" height="10" filter="url(#f)"/></svg>"#;
        assert!(to_wire_png_for_mime("image/svg+xml", svg).is_none());
    }

    #[test]
    fn nested_full_canvas_opacity_layers_exceed_svg_surface_budget() {
        let mut svg = String::from(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"2048\" height=\"2048\">",
        );
        for _ in 0..16 {
            svg.push_str("<g opacity=\"0.9\">");
        }
        svg.push_str("<rect width=\"2048\" height=\"2048\" fill=\"red\"/>");
        for _ in 0..16 {
            svg.push_str("</g>");
        }
        svg.push_str("</svg>");
        let tree = resvg::usvg::Tree::from_data(svg.as_bytes(), &Default::default()).unwrap();
        assert!(matches!(
            tree.root().children().first(),
            Some(resvg::usvg::Node::Group(_))
        ));
        assert!(!svg_tree_within_render_budget(&tree, 2048, 2048));
        assert!(to_wire_png_for_mime("image/svg+xml", svg.as_bytes()).is_none());
    }

    #[test]
    fn modest_opacity_layers_still_render() {
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="32" height="32"><g opacity="0.8"><g opacity="0.8"><rect width="32" height="32" fill="red"/></g></g></svg>"#;
        assert!(to_wire_png_for_mime("image/svg+xml", svg).is_some());
    }

    #[test]
    fn many_sibling_layers_exceed_svg_total_work_budget() {
        let mut svg = String::from(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1024\" height=\"1024\">",
        );
        for _ in 0..80 {
            svg.push_str(
                "<g opacity=\"0.9\"><rect width=\"1024\" height=\"1024\" fill=\"red\"/></g>",
            );
        }
        svg.push_str("</svg>");
        let tree = resvg::usvg::Tree::from_data(svg.as_bytes(), &Default::default()).unwrap();
        assert!(!svg_tree_within_render_budget(&tree, 1024, 1024));
        assert!(to_wire_png_for_mime("image/svg+xml", svg.as_bytes()).is_none());
    }

    #[test]
    fn svg_clip_masks_and_patterns_do_not_reach_renderer() {
        let variants = [
            r#"<clipPath id="c"><rect width="4" height="4"/></clipPath>"#,
            r#"<mask id="m"><rect width="4" height="4" fill="white"/></mask>"#,
            r#"<pattern id="p" width="4" height="4" patternUnits="userSpaceOnUse"><rect width="4" height="4" fill="red"/></pattern>"#,
        ];
        let attributes = [
            "clip-path=\"url(#c)\"",
            "mask=\"url(#m)\"",
            "fill=\"url(#p)\"",
        ];
        for (definition, attribute) in variants.into_iter().zip(attributes) {
            let svg = format!(
                "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"4\" height=\"4\"><defs>{definition}</defs><rect width=\"4\" height=\"4\" {attribute}/></svg>"
            );
            assert!(to_wire_png_for_mime("image/svg+xml", svg.as_bytes()).is_none());
        }
    }

    #[test]
    fn svg_parse_budget_rejects_deep_and_many_element_sources() {
        let mut deep = String::from("<svg>");
        for _ in 0..MAX_SVG_DEPTH {
            deep.push_str("<g>");
        }
        for _ in 0..MAX_SVG_DEPTH {
            deep.push_str("</g>");
        }
        deep.push_str("</svg>");
        assert!(!svg_source_within_parse_budget(deep.as_bytes()));

        let many = format!("<svg>{}</svg>", "<path/>".repeat(MAX_SVG_NODES));
        assert!(!svg_source_within_parse_budget(many.as_bytes()));
        assert!(!svg_source_within_parse_budget(&vec![
            b' ';
            MAX_SVG_SOURCE_BYTES
                + 1
        ]));
    }

    #[test]
    fn svg_references_and_markers_are_rejected_before_usvg_expands_them() {
        let use_svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="8" height="8"><defs><path id="shape" d="M0 0h8v8z"/></defs><use href="#shape"/></svg>"##;
        let marker_svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="8" height="8"><defs><marker id="m"><path d="M0 0h1v1z"/></marker></defs><path d="M0 0h8" marker-end="url(#m)"/></svg>"##;
        for svg in [use_svg.as_slice(), marker_svg.as_slice()] {
            assert!(!svg_source_within_parse_budget(svg));
            assert!(to_wire_png_for_mime("image/svg+xml", svg).is_none());
        }
    }

    #[test]
    fn inherited_dash_arrays_and_css_are_rejected_before_usvg() {
        let group = br#"<svg xmlns="http://www.w3.org/2000/svg" width="8" height="8"><g stroke-dasharray="1,2"><path d="M0 0h8"/></g></svg>"#;
        let inline = br#"<svg xmlns="http://www.w3.org/2000/svg" width="8" height="8"><path style="stroke-dasharray: 1,2" d="M0 0h8"/></svg>"#;
        let stylesheet = br#"<svg xmlns="http://www.w3.org/2000/svg" width="8" height="8"><style>path { stroke-dasharray: 1,2 }</style><path d="M0 0h8"/></svg>"#;
        for svg in [group.as_slice(), inline.as_slice(), stylesheet.as_slice()] {
            assert!(!svg_source_within_parse_budget(svg));
            assert!(to_wire_png_for_mime("image/svg+xml", svg).is_none());
        }
    }

    #[test]
    fn svg_preflight_bounds_text_and_geometry_before_usvg() {
        let long_text = format!(
            "<svg><text>{}</text></svg>",
            "a".repeat(MAX_SVG_TEXT_BYTES + 1)
        );
        assert!(!svg_source_within_parse_budget(long_text.as_bytes()));

        let long_path = format!(
            "<svg><path d=\"{}\"/></svg>",
            "M0 0 ".repeat(MAX_SVG_GEOMETRY_BYTES / 5 + 1)
        );
        assert!(!svg_source_within_parse_budget(long_path.as_bytes()));
    }
}
