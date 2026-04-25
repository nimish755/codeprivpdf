use image::{DynamicImage, ImageFormat as ImgFormat};
use pdf_core::{PdfError, PdfResult};

use crate::ImageFormat;

/// Filter information for decoding PDF images.
#[derive(Debug, Clone)]
pub struct ImageFilter {
    /// The filter name (e.g., "DCTDecode")
    pub name: String,
    /// Image width in pixels
    pub width: u32,
    /// Image height in pixels
    pub height: u32,
    /// Bits per component (usually 8)
    pub bits_per_component: u8,
    /// Color space (e.g., "DeviceRGB", "DeviceGray")
    pub color_space: String,
}

impl ImageFilter {
    /// Create a new ImageFilter.
    pub fn new(name: &str, width: u32, height: u32) -> Self {
        Self {
            name: name.to_string(),
            width,
            height,
            bits_per_component: 8,
            color_space: "DeviceRGB".to_string(),
        }
    }

    /// Set bits per component.
    pub fn with_bits(mut self, bits: u8) -> Self {
        self.bits_per_component = bits;
        self
    }

    /// Set color space.
    pub fn with_color_space(mut self, cs: &str) -> Self {
        self.color_space = cs.to_string();
        self
    }

    /// Get the image format.
    pub fn format(&self) -> ImageFormat {
        ImageFormat::from_filter_name(self.name.as_bytes())
    }

    /// Get the expected number of color components.
    pub fn components(&self) -> u8 {
        match self.color_space.as_str() {
            "DeviceGray" | "CalGray" => 1,
            "DeviceRGB" | "CalRGB" => 3,
            "DeviceCMYK" => 4,
            _ => 3, // Default to RGB
        }
    }
}

/// Decode image data from a PDF image stream.
pub fn decode_image(data: &[u8], filter: &ImageFilter) -> PdfResult<DynamicImage> {
    let format = filter.format();

    match format {
        ImageFormat::Jpeg => decode_jpeg(data),
        ImageFormat::Png => decode_flate(data, filter),
        ImageFormat::Raw => decode_raw(data, filter),
        #[cfg(feature = "jpeg2000")]
        ImageFormat::Jpeg2000 => decode_jpeg2000(data),
        #[cfg(feature = "jbig2")]
        ImageFormat::Jbig2 => decode_jbig2(data, filter),
        _ => Err(PdfError::UnsupportedFormat(format!("{:?}", format))),
    }
}

/// Decode JPEG image data.
fn decode_jpeg(data: &[u8]) -> PdfResult<DynamicImage> {
    image::load_from_memory_with_format(data, ImgFormat::Jpeg)
        .map_err(|e| PdfError::ImageError(format!("JPEG decode error: {}", e)))
}

/// Decode Flate (zlib) compressed image data.
fn decode_flate(data: &[u8], filter: &ImageFilter) -> PdfResult<DynamicImage> {
    use flate2::read::ZlibDecoder;
    use std::io::Read;

    let mut decoder = ZlibDecoder::new(data);
    let mut decompressed = Vec::new();
    decoder
        .read_to_end(&mut decompressed)
        .map_err(|e| PdfError::ImageError(format!("Flate decode error: {}", e)))?;

    decode_raw(&decompressed, filter)
}

/// Decode raw uncompressed image data.
fn decode_raw(data: &[u8], filter: &ImageFilter) -> PdfResult<DynamicImage> {
    let width = filter.width;
    let height = filter.height;
    let components = filter.components();

    let expected_size = (width * height * components as u32) as usize;

    if data.len() < expected_size {
        return Err(PdfError::ImageError(format!(
            "Raw image data too small: {} bytes, expected {}",
            data.len(),
            expected_size
        )));
    }

    match components {
        1 => {
            // Grayscale
            image::GrayImage::from_raw(width, height, data[..expected_size].to_vec())
                .map(DynamicImage::ImageLuma8)
                .ok_or_else(|| PdfError::ImageError("Failed to create grayscale image".into()))
        }
        3 => {
            // RGB
            image::RgbImage::from_raw(width, height, data[..expected_size].to_vec())
                .map(DynamicImage::ImageRgb8)
                .ok_or_else(|| PdfError::ImageError("Failed to create RGB image".into()))
        }
        4 => {
            // CMYK - convert to RGB
            let rgb_data = cmyk_to_rgb(&data[..expected_size]);
            image::RgbImage::from_raw(width, height, rgb_data)
                .map(DynamicImage::ImageRgb8)
                .ok_or_else(|| PdfError::ImageError("Failed to create RGB image from CMYK".into()))
        }
        _ => Err(PdfError::ImageError(format!(
            "Unsupported component count: {}",
            components
        ))),
    }
}

/// Convert CMYK to RGB.
fn cmyk_to_rgb(cmyk_data: &[u8]) -> Vec<u8> {
    let mut rgb = Vec::with_capacity((cmyk_data.len() / 4) * 3);

    for chunk in cmyk_data.chunks(4) {
        if chunk.len() == 4 {
            let c = chunk[0] as f32 / 255.0;
            let m = chunk[1] as f32 / 255.0;
            let y = chunk[2] as f32 / 255.0;
            let k = chunk[3] as f32 / 255.0;

            let r = 255.0 * (1.0 - c) * (1.0 - k);
            let g = 255.0 * (1.0 - m) * (1.0 - k);
            let b = 255.0 * (1.0 - y) * (1.0 - k);

            rgb.push(r as u8);
            rgb.push(g as u8);
            rgb.push(b as u8);
        }
    }

    rgb
}

/// Decode JPEG2000 image data.
#[cfg(feature = "jpeg2000")]
fn decode_jpeg2000(data: &[u8]) -> PdfResult<DynamicImage> {

    use hayro_jpeg2000::Jpeg2000Image;

    let jp2_image = Jpeg2000Image::from_bytes(data)
        .map_err(|e| PdfError::ImageError(format!("JPEG2000 decode error: {:?}", e)))?;

    let width = jp2_image.width();
    let height = jp2_image.height();
    let pixels = jp2_image.to_rgb8();

    image::RgbImage::from_raw(width, height, pixels)
        .map(DynamicImage::ImageRgb8)
        .ok_or_else(|| PdfError::ImageError("Failed to create image from JPEG2000 data".into()))
}

/// Decode JBIG2 image data.
#[cfg(feature = "jbig2")]
fn decode_jbig2(data: &[u8], filter: &ImageFilter) -> PdfResult<DynamicImage> {
    use hayro_jbig2::Jbig2Image;

    let jbig2_image = Jbig2Image::from_bytes(data)
        .map_err(|e| PdfError::ImageError(format!("JBIG2 decode error: {:?}", e)))?;

    let width = jbig2_image.width();
    let height = jbig2_image.height();
    let pixels = jbig2_image.to_gray8();

    image::GrayImage::from_raw(width, height, pixels)
        .map(DynamicImage::ImageLuma8)
        .ok_or_else(|| PdfError::ImageError("Failed to create image from JBIG2 data".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_format_from_filter() {
        assert_eq!(
            ImageFormat::from_filter_name(b"DCTDecode"),
            ImageFormat::Jpeg
        );
        assert_eq!(
            ImageFormat::from_filter_name(b"FlateDecode"),
            ImageFormat::Png
        );
        assert_eq!(
            ImageFormat::from_filter_name(b"JPXDecode"),
            ImageFormat::Jpeg2000
        );
    }

    #[test]
    fn test_cmyk_to_rgb() {
        let cmyk = vec![255, 0, 0, 0];
        let rgb = cmyk_to_rgb(&cmyk);
        assert_eq!(rgb.len(), 3);
        assert_eq!(rgb[0], 0); 
        assert_eq!(rgb[1], 255); 
        assert_eq!(rgb[2], 255); 
    }
}
