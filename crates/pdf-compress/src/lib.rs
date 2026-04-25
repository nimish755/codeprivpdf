/// AI gened comments as I am too lazy and i gave up on compression atp
use wasm_bindgen::prelude::*;

mod compress;

pub use compress::{compress_document, CompressionStats};

/// Compress a PDF document with the specified compression mode.
///
/// # Arguments
/// * `pdf_bytes` - The PDF document as bytes.
/// * `mode` - Compression mode: { type: "Lossless" } | { type: "Quality", value: 75 } | { type: "TargetSize", value: 500000 }
///
/// # Returns
/// The compressed PDF as bytes.
#[wasm_bindgen]
pub fn compress_pdf(pdf_bytes: &[u8], mode: JsValue) -> Result<Vec<u8>, JsValue> {
    let mode: pdf_core::CompressionMode =
        serde_wasm_bindgen::from_value(mode).map_err(|e| JsValue::from_str(&e.to_string()))?;

    compress_document(pdf_bytes, &mode).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Compress a PDF to achieve a target file size.
///
/// # Arguments
/// * `pdf_bytes` - The PDF document as bytes.
/// * `target_bytes` - Target file size in bytes.
///
/// # Returns
/// The compressed PDF. Note: The result may be larger than target if
/// the PDF cannot be compressed further.
#[wasm_bindgen]
pub fn compress_pdf_to_target(pdf_bytes: &[u8], target_bytes: u32) -> Result<Vec<u8>, JsValue> {
    let mode = pdf_core::CompressionMode::TargetSize(target_bytes);
    compress_document(pdf_bytes, &mode).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Compress a PDF with a specific quality level.
///
/// # Arguments
/// * `pdf_bytes` - The PDF document as bytes.
/// * `quality` - Quality level (1-100, where 100 is best quality).
///
/// # Returns
/// The compressed PDF.
#[wasm_bindgen]
pub fn compress_pdf_with_quality(pdf_bytes: &[u8], quality: u8) -> Result<Vec<u8>, JsValue> {
    let mode = pdf_core::CompressionMode::Quality(quality.clamp(1, 100));
    compress_document(pdf_bytes, &mode).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Apply lossless compression to a PDF.
///
/// This optimizes the PDF structure and applies lossless image optimization
/// without any quality loss.
///
/// # Arguments
/// * `pdf_bytes` - The PDF document as bytes.
///
/// # Returns
/// The optimized PDF.
#[wasm_bindgen]
pub fn compress_pdf_lossless(pdf_bytes: &[u8]) -> Result<Vec<u8>, JsValue> {
    let mode = pdf_core::CompressionMode::Lossless;
    compress_document(pdf_bytes, &mode).map_err(|e| JsValue::from_str(&e.to_string()))
}


/// Get compression statistics for a PDF.
///
/// Analyzes the PDF to determine potential compression savings.
///
/// # Returns
/// An object with statistics about the PDF content.
#[wasm_bindgen]
pub fn analyze_pdf(pdf_bytes: &[u8]) -> Result<JsValue, JsValue> {
    let stats = compress::analyze_document(pdf_bytes).map_err(|e| JsValue::from_str(&e.to_string()))?;
    serde_wasm_bindgen::to_value(&stats).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Get the version of the pdf-compress module.
#[wasm_bindgen]
pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
