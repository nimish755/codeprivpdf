/// AI genned comments.


use wasm_bindgen::prelude::*;

mod split;

pub use split::{split_by_ranges, split_into_pages};

/// Split a PDF into individual single-page PDFs.
///
/// # Arguments
/// * `pdf_bytes` - A Uint8Array containing the PDF document.
///
/// # Returns
/// An array of Uint8Arrays, each containing a single-page PDF.
#[wasm_bindgen]
pub fn split_pdf(pdf_bytes: &[u8]) -> Result<js_sys::Array, JsValue> {
    let pages = split_into_pages(pdf_bytes).map_err(|e| JsValue::from_str(&e.to_string()))?;

    let result = js_sys::Array::new();
    for page in pages {
        let array = js_sys::Uint8Array::from(page.as_slice());
        result.push(&array);
    }

    Ok(result)
}

/// Split a PDF by custom page ranges.
///
/// # Arguments
/// * `pdf_bytes` - A Uint8Array containing the PDF document.
/// * `ranges` - An array of objects with `start` and `end` properties (1-indexed, inclusive).
///
/// # Returns
/// An array of Uint8Arrays, each containing the pages for that range.
///
/// # Example
/// ```javascript
/// const ranges = [
///   { start: 1, end: 3 },   // Pages 1-3
///   { start: 5, end: 5 },   // Just page 5
///   { start: 7, end: 10 }   // Pages 7-10
/// ];
/// const parts = split_pdf_by_ranges(pdfBytes, ranges);
/// ```
#[wasm_bindgen]
pub fn split_pdf_by_ranges(pdf_bytes: &[u8], ranges: JsValue) -> Result<js_sys::Array, JsValue> {
    // Deserialize ranges from JS
    let ranges: Vec<pdf_core::PageRange> =
        serde_wasm_bindgen::from_value(ranges).map_err(|e| JsValue::from_str(&e.to_string()))?;

    if ranges.is_empty() {
        return Err(JsValue::from_str("No page ranges provided"));
    }

    let parts = split_by_ranges(pdf_bytes, &ranges).map_err(|e| JsValue::from_str(&e.to_string()))?;

    let result = js_sys::Array::new();
    for part in parts {
        let array = js_sys::Uint8Array::from(part.as_slice());
        result.push(&array);
    }

    Ok(result)
}

/// Get the number of pages in a PDF document.
#[wasm_bindgen]
pub fn get_page_count(pdf_bytes: &[u8]) -> Result<u32, JsValue> {
    pdf_core::get_page_count(pdf_bytes).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Get the version of the pdf-split module.
#[wasm_bindgen]
pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
