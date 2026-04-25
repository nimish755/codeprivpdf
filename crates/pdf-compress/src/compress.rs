
use std::io::Cursor;
use std::io::Write;

use flate2::write::ZlibEncoder;
use flate2::Compression;
use image::DynamicImage;
use lopdf::{Document, Object, ObjectId, Stream};
use serde::{Deserialize, Serialize};

use pdf_codecs::{decode_image, encode_jpeg, ImageFilter};
use pdf_core::{CompressionMode, PdfError, PdfResult};

/// Statistics about PDF compression cause IT FUCKIN BROKE AND OMFG DO NOT EVEN GET ME STARTED on THIS STUPID FUCKIN CODE IG GIVE up  hefiefhiweb.
/// Actually wrote the comments for this one cause even my fuckass does not understand what i did here to break compress this much
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionStats {
    /// Original file size in bytes
    pub original_size: usize,
    /// Compressed file size in bytes
    pub compressed_size: usize,
    /// Number of images found
    pub image_count: usize,
    /// Number of images that were recompressed
    pub images_recompressed: usize,
    /// Total size of images before compression
    pub images_original_size: usize,
    /// Total size of images after compression
    pub images_compressed_size: usize,
    /// Compression ratio (original / compressed)
    pub compression_ratio: f64,
    /// Size reduction percentage
    pub size_reduction_percent: f64,
}

impl CompressionStats {
    fn new(original_size: usize) -> Self {
        Self {
            original_size,
            compressed_size: original_size,
            image_count: 0,
            images_recompressed: 0,
            images_original_size: 0,
            images_compressed_size: 0,
            compression_ratio: 1.0,
            size_reduction_percent: 0.0,
        }
    }

    fn finalize(&mut self, compressed_size: usize) {
        self.compressed_size = compressed_size;
        if compressed_size > 0 {
            self.compression_ratio = self.original_size as f64 / compressed_size as f64;
        }
        if self.original_size > 0 {
            self.size_reduction_percent =
                (1.0 - (compressed_size as f64 / self.original_size as f64)) * 100.0;
        }
    }
}

/// check PDF  for compression stuff my shitty code can do.
pub fn analyze_document(pdf_bytes: &[u8]) -> PdfResult<CompressionStats> {
    let cursor = Cursor::new(pdf_bytes);
    let doc = Document::load_from(cursor).map_err(|e| PdfError::ParseError(e.to_string()))?;

    let mut stats = CompressionStats::new(pdf_bytes.len());

    // Find all image XObjects
    for (_, object) in &doc.objects {
        if let Ok(stream) = object.as_stream() {
            if is_image_stream(&stream.dict) {
                stats.image_count += 1;
                stats.images_original_size += stream.content.len();
            }
        }
    }

    stats.images_compressed_size = stats.images_original_size;
    Ok(stats)
}

/// Compress like family
pub fn compress_document(pdf_bytes: &[u8], mode: &CompressionMode) -> PdfResult<Vec<u8>> {
    let cursor = Cursor::new(pdf_bytes);
    let mut doc = Document::load_from(cursor).map_err(|e| PdfError::ParseError(e.to_string()))?;

    let original_size = pdf_bytes.len();
    let mut stats = CompressionStats::new(original_size);

    // image objects to process that we got
    let image_ids: Vec<ObjectId> = doc
        .objects
        .iter()
        .filter_map(|(id, obj)| {
            if let Ok(stream) = obj.as_stream() {
                if is_image_stream(&stream.dict) {
                    return Some(*id);
                }
            }
            None
        })
        .collect();

    stats.image_count = image_ids.len();

    // For target size mode, we need to iteratively compress with decreasing quality
    // THIS IS SHITTY CODE and an even more shitty idea... idk who in their right mind thought this would work... and that person is ME
    // I do not have better ideas sooo...
    if let CompressionMode::TargetSize(target_bytes) = mode {
        return compress_to_target_size(&mut doc, &image_ids, *target_bytes as usize, original_size);
    }

    // Process each image for quality or lossless modes
    for image_id in &image_ids {
        match compress_image_object(&doc, *image_id, mode) {
            Ok(compressed) => {
                let original_img_size = doc
                    .get_object(*image_id)
                    .ok()
                    .and_then(|o| o.as_stream().ok())
                    .map(|s| s.content.len())
                    .unwrap_or(0);

                stats.images_original_size += original_img_size;
                stats.images_compressed_size += compressed.content.len();

                // Only replace if we achieved compression (or it's a format change)
                if compressed.content.len() < original_img_size {
                    doc.objects.insert(*image_id, Object::Stream(compressed));
                    stats.images_recompressed += 1;
                }
            }
            Err(_) => {
                // Skip images that can't be compressed (e.g., JPEG in lossless mode)
                continue;
            }
        }
    }

    // Apply general PDF compression (removes unused objects, compresses streams)
    doc.compress();

    // Write output
    let mut output = Vec::new();
    doc.save_to(&mut output)
        .map_err(|e| PdfError::WriteError(e.to_string()))?;

    stats.finalize(output.len());

    Ok(output)
}

/// Unsed for now cause i think these functions break it
fn remove_document_metadata(doc: &mut Document) {
    // Clear the trailer's Info dictionary reference
    if let Ok(info_ref) = doc.trailer.get(b"Info") {
        if let Ok(obj_id) = info_ref.as_reference() {
            // Remove the Info object
            doc.objects.remove(&obj_id);
        }
    }
    doc.trailer.remove(b"Info");
}
/// Unsed for now cause i think these functions break it
fn remove_xmp_metadata(doc: &mut Document) {
    let metadata_ids: Vec<ObjectId> = doc
        .objects
        .iter()
        .filter_map(|(id, obj)| {
            if let Ok(stream) = obj.as_stream() {
                // Check if it's a Metadata stream
                if stream.dict.get(b"Type")
                    .ok()
                    .and_then(|t| t.as_name().ok())
                    .map(|n| n == b"Metadata")
                    .unwrap_or(false)
                {
                    return Some(*id);
                }
                // Also check Subtype
                if stream.dict.get(b"Subtype")
                    .ok()
                    .and_then(|t| t.as_name().ok())
                    .map(|n| n == b"XML")
                    .unwrap_or(false)
                {
                    return Some(*id);
                }
            }
            None
        })
        .collect();

    for id in metadata_ids {
        doc.objects.remove(&id);
    }

    if let Ok(catalog) = doc.catalog_mut() {
        catalog.remove(b"Metadata");
    }
}
/// Unsed for now cause i think these functions break it
fn remove_thumbnails(doc: &mut Document) {
    // First pass: collect page IDs and their thumbnail object IDs
    let mut thumb_objects_to_remove: Vec<ObjectId> = Vec::new();
    let mut pages_to_update: Vec<ObjectId> = Vec::new();
    
    for (id, obj) in &doc.objects {
        if let Ok(dict) = obj.as_dict() {
            if dict.get(b"Type")
                .ok()
                .and_then(|t| t.as_name().ok())
                .map(|n| n == b"Page")
                .unwrap_or(false)
            {
                if let Ok(thumb_ref) = dict.get(b"Thumb") {
                    pages_to_update.push(*id);
                    if let Ok(thumb_id) = thumb_ref.as_reference() {
                        thumb_objects_to_remove.push(thumb_id);
                    }
                }
            }
        }
    }

    for thumb_id in thumb_objects_to_remove {
        doc.objects.remove(&thumb_id);
    }

    for page_id in pages_to_update {
        if let Ok(Object::Dictionary(dict)) = doc.get_object_mut(page_id) {
            dict.remove(b"Thumb");
        }
    }
}



/// Compress PDF to a target file size by iteratively reducing quality.
fn compress_to_target_size(
    doc: &mut Document,
    image_ids: &[ObjectId],
    target_bytes: usize,
    _original_size: usize,
) -> PdfResult<Vec<u8>> {
    // Try quality levels from high to low
    let quality_levels = [90, 80, 70, 60, 50, 40, 30, 20, 10];
    
    let mut best_output: Option<Vec<u8>> = None;
    
    for quality in quality_levels {
        let mode = CompressionMode::Quality(quality);
        
        // Create a fresh copy for each attempt
        let mut doc_copy = doc.clone();
        
        for image_id in image_ids {
            if let Ok(compressed) = compress_image_object(&doc_copy, *image_id, &mode) {
                doc_copy.objects.insert(*image_id, Object::Stream(compressed));
            }
        }
        
        doc_copy.compress();
        
        let mut output = Vec::new();
        doc_copy.save_to(&mut output)
            .map_err(|e| PdfError::WriteError(e.to_string()))?;
        
        // Check if we hit the target
        if output.len() <= target_bytes {
            return Ok(output);
        }
        
        // Keep track of best result
        if best_output.is_none() || output.len() < best_output.as_ref().unwrap().len() {
            best_output = Some(output);
        }
    }
    
    // Return the smallest we achieved, even if larger than target
    best_output.ok_or_else(|| PdfError::WriteError("Failed to compress PDF".to_string()))
}

/// Check if a dictionary represents an image stream.
fn is_image_stream(dict: &lopdf::Dictionary) -> bool {
    dict.get(b"Subtype")
        .ok()
        .and_then(|s| s.as_name().ok())
        .map(|n| n == b"Image")
        .unwrap_or(false)
}

/// Get image filter information from a stream dictionary.
fn get_image_filter(dict: &lopdf::Dictionary) -> Option<ImageFilter> {
    let width = dict
        .get(b"Width")
        .ok()
        .and_then(|w| w.as_i64().ok())
        .unwrap_or(0) as u32;
    let height = dict
        .get(b"Height")
        .ok()
        .and_then(|h| h.as_i64().ok())
        .unwrap_or(0) as u32;

    if width == 0 || height == 0 {
        return None;
    }

    let filter_name = dict
        .get(b"Filter")
        .ok()
        .and_then(|f| {
            // Filter can be a name or an array of names
            if let Ok(name) = f.as_name() {
                Some(String::from_utf8_lossy(name).to_string())
            } else if let Ok(arr) = f.as_array() {
                // Get the first filter in the chain
                arr.first()
                    .and_then(|f| f.as_name().ok())
                    .map(|n| String::from_utf8_lossy(n).to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "Raw".to_string());

    let bits = dict
        .get(b"BitsPerComponent")
        .ok()
        .and_then(|b| b.as_i64().ok())
        .unwrap_or(8) as u8;

    let color_space = dict
        .get(b"ColorSpace")
        .ok()
        .and_then(|cs| {
            if let Ok(name) = cs.as_name() {
                Some(String::from_utf8_lossy(name).to_string())
            } else if let Ok(arr) = cs.as_array() {
                arr.first()
                    .and_then(|n| n.as_name().ok())
                    .map(|n| String::from_utf8_lossy(n).to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "DeviceRGB".to_string());

    Some(
        ImageFilter::new(&filter_name, width, height)
            .with_bits(bits)
            .with_color_space(&color_space),
    )
}

/// Compress a single image object.
fn compress_image_object(
    doc: &Document,
    image_id: ObjectId,
    mode: &CompressionMode,
) -> PdfResult<Stream> {
    let object = doc
        .get_object(image_id)
        .map_err(|e| PdfError::ParseError(e.to_string()))?;

    let stream = object
        .as_stream()
        .map_err(|_| PdfError::ParseError("Not a stream object".to_string()))?;

    let filter = get_image_filter(&stream.dict)
        .ok_or_else(|| PdfError::ImageError("Could not determine image filter".to_string()))?;

    // Decode the image
    let image = decode_image(&stream.content, &filter)?;

    // Re-encode based on compression mode
    let (new_content, new_filter) = encode_image(&image, mode, &filter)?;

    // Create new stream with updated dictionary
    let mut new_dict = stream.dict.clone();
    new_dict.set("Filter", Object::Name(new_filter.into_bytes()));
    new_dict.set("Length", Object::Integer(new_content.len() as i64));

    // Update color space if needed
    if matches!(image, DynamicImage::ImageRgb8(_) | DynamicImage::ImageRgba8(_)) {
        new_dict.set("ColorSpace", Object::Name(b"DeviceRGB".to_vec()));
    } else if matches!(image, DynamicImage::ImageLuma8(_)) {
        new_dict.set("ColorSpace", Object::Name(b"DeviceGray".to_vec()));
    }

    Ok(Stream::new(new_dict, new_content))
}

/// Encode an image based on compression mode.
fn encode_image(
    image: &DynamicImage,
    mode: &CompressionMode,
    original_filter: &ImageFilter,
) -> PdfResult<(Vec<u8>, String)> {
    match mode {
        CompressionMode::Lossless => {
            // For lossless, check if original was JPEG - if so, don't re-encode
            // Only optimize if we can do better than original
            if original_filter.name == "DCTDecode" {
                // Already JPEG, can't improve losslessly - use original
                // Return error to signal we should skip this image
                return Err(PdfError::ImageError("Skip JPEG in lossless mode".to_string()));
            }
            // For non-JPEG, encode as raw with FlateDecode
            let rgb = image.to_rgb8();
            let raw_data = rgb.as_raw().to_vec();
            
            // Compress with flate
            let mut encoder = ZlibEncoder::new(Vec::new(), Compression::best());
            encoder.write_all(&raw_data)
                .map_err(|e| PdfError::ImageError(format!("Flate encode error: {}", e)))?;
            let data = encoder.finish()
                .map_err(|e| PdfError::ImageError(format!("Flate finish error: {}", e)))?;
            
            Ok((data, "FlateDecode".to_string()))
        }
        CompressionMode::Quality(quality) => {
            // For quality mode, use JPEG
            let data = encode_jpeg(image, *quality)?;
            Ok((data, "DCTDecode".to_string()))
        }
        CompressionMode::TargetSize(_target) => {
            // For target size, use medium quality JPEG (we'll iterate externally)
            let data = encode_jpeg(image, 50)?;
            Ok((data, "DCTDecode".to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compression_stats() {
        let mut stats = CompressionStats::new(1000);
        stats.finalize(500);

        assert_eq!(stats.original_size, 1000);
        assert_eq!(stats.compressed_size, 500);
        assert!((stats.compression_ratio - 2.0).abs() < 0.001);
        assert!((stats.size_reduction_percent - 50.0).abs() < 0.001);
    }

    #[test]
    fn test_is_image_stream() {
        let mut dict = lopdf::Dictionary::new();
        dict.set("Subtype", Object::Name(b"Image".to_vec()));

        assert!(is_image_stream(&dict));

        let mut non_image = lopdf::Dictionary::new();
        non_image.set("Subtype", Object::Name(b"Form".to_vec()));

        assert!(!is_image_stream(&non_image));
    }
}
