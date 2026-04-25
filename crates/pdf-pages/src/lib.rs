
/// AI genned comments.

use wasm_bindgen::prelude::*;

mod pages;

pub use pages::{extract_pages_from_pdf, remove_pages_from_pdf};

/// Remove specific pages from a PDF document.
///
/// # Arguments
/// * `pdf_bytes` - A Uint8Array containing the PDF document.
/// * `page_numbers` - An array of page numbers to remove (1-indexed).
///
/// # Returns
/// A Uint8Array containing the PDF with the specified pages removed.
#[wasm_bindgen]
pub fn remove_pages(pdf_bytes: &[u8], page_numbers: &[u32]) -> Result<Vec<u8>, JsValue> {
    if page_numbers.is_empty() {
        // Nothing to remove, return original
        return Ok(pdf_bytes.to_vec());
    }

    remove_pages_from_pdf(pdf_bytes, page_numbers).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Extract specific pages from a PDF document into a new PDF.
///
/// # Arguments
/// * `pdf_bytes` - A Uint8Array containing the PDF document.
/// * `page_numbers` - An array of page numbers to extract (1-indexed).
///
/// # Returns
/// A Uint8Array containing only the specified pages.
#[wasm_bindgen]
pub fn extract_pages(pdf_bytes: &[u8], page_numbers: &[u32]) -> Result<Vec<u8>, JsValue> {
    if page_numbers.is_empty() {
        return Err(JsValue::from_str("No pages specified for extraction"));
    }

    extract_pages_from_pdf(pdf_bytes, page_numbers).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Reorder pages in a PDF document.
///
/// # Arguments
/// * `pdf_bytes` - A Uint8Array containing the PDF document.
/// * `new_order` - An array of page numbers in the desired order (1-indexed).
///                 Must contain all pages exactly once.
///
/// # Returns
/// A Uint8Array containing the PDF with pages reordered.
#[wasm_bindgen]
pub fn reorder_pages(pdf_bytes: &[u8], new_order: &[u32]) -> Result<Vec<u8>, JsValue> {
    if new_order.is_empty() {
        return Err(JsValue::from_str("New order cannot be empty"));
    }

    pages::reorder_pages_in_pdf(pdf_bytes, new_order).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Get the number of pages in a PDF document.
#[wasm_bindgen]
pub fn get_page_count(pdf_bytes: &[u8]) -> Result<u32, JsValue> {
    pdf_core::get_page_count(pdf_bytes).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Get the version of the pdf-pages module.
#[wasm_bindgen]
pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
