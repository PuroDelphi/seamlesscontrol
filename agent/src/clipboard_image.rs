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
        _ => None,
    }
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
    image.write_to(&mut output, ImageFormat::Png).ok()?;
    let output = output.0.into_inner();
    valid_png_header(&output).then_some(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, Rgba, RgbaImage};

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
    fn mime_names_map_to_expected_decoders() {
        assert_eq!(format_for_mime("image/jpg"), Some(ImageFormat::Jpeg));
        assert_eq!(format_for_mime("image/x-ms-bmp"), Some(ImageFormat::Bmp));
        assert_eq!(format_for_mime("image/svg+xml"), None);
    }
}
