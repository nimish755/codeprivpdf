// Half of these comments were AI gen cause I forgot to write them :D
use image::{DynamicImage, GenericImageView};
use pdf_core::{PdfError, PdfResult};
use std::io::Cursor;

/// Compression quality settings.
#[derive(Debug, Clone, Copy)]
pub struct CompressionQuality {
    /// JPEG quality (1-100)
    pub jpeg_quality: u8,
    /// PNG compression level (0-9)
    pub png_compression: u8,
}

impl Default for CompressionQuality {
    fn default() -> Self {
        Self {
            jpeg_quality: 85,
            png_compression: 6,
        }
    }
}

impl CompressionQuality {
    /// Create a new CompressionQuality with specified JPEG quality.
    pub fn new(jpeg_quality: u8) -> Self {
        Self {
            jpeg_quality: jpeg_quality.clamp(1, 100),
            ..Default::default()
        }
    }

    /// Set PNG compression level.
    pub fn with_png_compression(mut self, level: u8) -> Self {
        self.png_compression = level.clamp(0, 9);
        self
    }
}

/// Downscale an image if it exceeds the maximum dimension.
/// 
/// # Arguments
/// * `image` - The image to potentially downscale.
/// * `max_dimension` - Maximum width or height allowed.
/// 
/// # Returns
/// The original image if within limits, or a downscaled version.
pub fn downscale_image(image: &DynamicImage, max_dimension: u32) -> DynamicImage {
    let (width, height) = image.dimensions();
    
    // Check if downscaling is needed
    if width <= max_dimension && height <= max_dimension {
        return image.clone();
    }
    
    // Calculate new dimensions maintaining aspect ratio
    let scale = if width > height {
        max_dimension as f64 / width as f64
    } else {
        max_dimension as f64 / height as f64
    };
    
    let new_width = (width as f64 * scale) as u32;
    let new_height = (height as f64 * scale) as u32;
    
    // Use Lanczos3 for high-quality downscaling
    image.resize(new_width, new_height, image::imageops::FilterType::Lanczos3)
}

/// Detect if an image is photo-like (good for JPEG) or graphics-like (keep as PNG).
/// 
/// Uses color variance analysis to distinguish photos from graphics/text.
/// Photos typically have smooth gradients and high color variance.
/// Graphics/text have sharp edges and limited color palette.
/// 
/// # Returns
/// `true` if the image appears to be a photo, `false` for graphics/text.
pub fn is_photo_like(image: &DynamicImage) -> bool {
    let rgb = image.to_rgb8();
    let (width, height) = rgb.dimensions();
    
    // For very small images, treat as graphics
    if width < 50 || height < 50 {
        return false;
    }
    
    // Sample pixels and analyze color distribution
    let sample_size = 1000.min((width * height) as usize);
    let step = ((width * height) as usize / sample_size).max(1);
    
    let pixels: Vec<_> = rgb.pixels().step_by(step).take(sample_size).collect();
    
    if pixels.is_empty() {
        return false;
    }
    
    // Count unique colors (approximated by quantizing to 4-bit per channel)
    let mut color_set = std::collections::HashSet::new();
    for pixel in &pixels {
        let quantized = (
            pixel[0] >> 4,
            pixel[1] >> 4,
            pixel[2] >> 4,
        );
        color_set.insert(quantized);
    }
    
    let unique_colors = color_set.len();
    let color_ratio = unique_colors as f64 / sample_size as f64;
    
    // Photos typically have high color variety (>30% unique quantized colors)
    // Graphics/text typically have low color variety (<15%)
    color_ratio > 0.25
}

/// Encode an image as JPEG with specified quality.
///
/// # Arguments
/// * `image` - The image to encode.
/// * `quality` - JPEG quality (1-100).
///
/// # Returns
/// The JPEG-encoded image data.
pub fn encode_jpeg(image: &DynamicImage, quality: u8) -> PdfResult<Vec<u8>> {
    let quality = quality.clamp(1, 100);

    let rgb = image.to_rgb8();
    let mut buffer = Cursor::new(Vec::new());

    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buffer, quality);

    rgb.write_with_encoder(encoder)
        .map_err(|e| PdfError::ImageError(format!("JPEG encode error: {}", e)))?;

    Ok(buffer.into_inner())
}

/// Encode an image as PNG (lossless).
///
/// # Arguments
/// * `image` - The image to encode.
///
/// # Returns
/// The PNG-encoded image data.
pub fn encode_png(image: &DynamicImage) -> PdfResult<Vec<u8>> {
    let mut buffer = Cursor::new(Vec::new());

    image
        .write_to(&mut buffer, image::ImageFormat::Png)
        .map_err(|e| PdfError::ImageError(format!("PNG encode error: {}", e)))?;

    Ok(buffer.into_inner())
}

/// Smart encode: choose JPEG or PNG based on image content.
/// 
/// # Arguments
/// * `image` - The image to encode.
/// * `jpeg_quality` - Quality to use if encoding as JPEG.
/// * `prefer_jpeg_threshold` - Only use JPEG if it saves more than this percentage.
/// 
/// # Returns
/// Tuple of (encoded data, format name "DCTDecode" or "FlateDecode")
pub fn encode_smart(
    image: &DynamicImage,
    jpeg_quality: u8,
    prefer_jpeg_threshold: u8,
) -> PdfResult<(Vec<u8>, String)> {
    // Check if image is photo-like
    if is_photo_like(image) {
        // Try JPEG
        let jpeg_data = encode_jpeg(image, jpeg_quality)?;
        return Ok((jpeg_data, "DCTDecode".to_string()));
    }
    
    // For graphics, try both and compare
    let png_data = encode_png(image)?;
    let jpeg_data = encode_jpeg(image, jpeg_quality)?;
    
    // Calculate savings percentage
    let savings_percent = if png_data.len() > 0 {
        ((png_data.len() as f64 - jpeg_data.len() as f64) / png_data.len() as f64 * 100.0) as u8
    } else {
        0
    };
    
    // Use JPEG only if it saves significantly more
    if savings_percent >= prefer_jpeg_threshold {
        Ok((jpeg_data, "DCTDecode".to_string()))
    } else {
        Ok((png_data, "FlateDecode".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage};

    fn create_test_image() -> DynamicImage {
        let mut img = RgbImage::new(100, 100);
        for (x, y, pixel) in img.enumerate_pixels_mut() {
            *pixel = Rgb([
                (x % 256) as u8,
                (y % 256) as u8,
                ((x + y) % 256) as u8,
            ]);
        }
        DynamicImage::ImageRgb8(img)
    }

    #[test]
    fn test_encode_jpeg() {
        let img = create_test_image();
        let result = encode_jpeg(&img, 85);
        assert!(result.is_ok());

        let data = result.unwrap();
        assert!(!data.is_empty());
        // JPEG magic bytes
        assert_eq!(&data[0..2], &[0xFF, 0xD8]);
    }

    #[test]
    fn test_encode_png() {
        let img = create_test_image();
        let result = encode_png(&img);
        assert!(result.is_ok());

        let data = result.unwrap();
        assert!(!data.is_empty());
        // PNG magic bytes
        assert_eq!(&data[0..4], &[0x89, 0x50, 0x4E, 0x47]);
    }

    #[test]
    fn test_quality_affects_size() {
        let img = create_test_image();

        let high_quality = encode_jpeg(&img, 95).unwrap();
        let low_quality = encode_jpeg(&img, 30).unwrap();

        // Lower quality should produce smaller file
        assert!(low_quality.len() < high_quality.len());
    }

    #[test]
    fn test_encode_to_target_size() {
        let img = create_test_image();

        // Get baseline size
        let high_quality = encode_jpeg(&img, 95).unwrap();
        let target = high_quality.len() / 2;

        let result = encode_to_target_size(&img, target, Some(10)).unwrap();
        assert!(result.len() <= target || result.len() < high_quality.len());
    }

    #[test]
    fn test_compression_ratio() {
        assert!((compression_ratio(1000, 500) - 2.0).abs() < 0.001);
        assert!((compression_ratio(1000, 250) - 4.0).abs() < 0.001);
    }

    #[test]
    fn test_size_reduction_percent() {
        assert!((size_reduction_percent(1000, 500) - 50.0).abs() < 0.001);
        assert!((size_reduction_percent(1000, 250) - 75.0).abs() < 0.001);
    }
}
