//! Convert common raster clipboard representations to the PNG wire format.

use crate::clipboard::{MAX_IMAGE_BYTES, valid_png_header};
use image::{ImageFormat, ImageReader, Limits};
use std::io::{self, Cursor, Seek, SeekFrom, Write};

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
    if bytes.is_empty() || bytes.len() > MAX_IMAGE_BYTES {
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
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size.width(), size.height())?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::default(),
        &mut pixmap.as_mut(),
    );
    let png = pixmap.encode_png().ok()?;
    valid_png_header(&png).then_some(png)
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
}
