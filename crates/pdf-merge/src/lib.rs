/// AI genned comments.
use wasm_bindgen::prelude::*;

mod merge;

pub use merge::merge_documents;

/// Merge multiple PDF documents into a single PDF.
///
/// # Arguments
/// * `pdf_arrays` - A JavaScript array of Uint8Array, each containing a PDF document.
///
/// # Returns
/// A Uint8Array containing the merged PDF document.
///
/// # Errors
/// Returns a JsValue error if any PDF cannot be parsed or if merging fails.
#[wasm_bindgen]
pub fn merge_pdfs(pdf_arrays: js_sys::Array) -> Result<Vec<u8>, JsValue> {
    // Convert JS array to Vec of byte slices
    let mut pdf_bytes: Vec<Vec<u8>> = Vec::with_capacity(pdf_arrays.length() as usize);

    for i in 0..pdf_arrays.length() {
        let item = pdf_arrays.get(i);
        let array = js_sys::Uint8Array::from(item);
        pdf_bytes.push(array.to_vec());
    }

    if pdf_bytes.is_empty() {
        return Err(JsValue::from_str("No PDF documents provided"));
    }

    if pdf_bytes.len() == 1 {
        // Nothing to merge, return the single PDF
        return Ok(pdf_bytes.into_iter().next().unwrap());
    }

    // Convert to slices for merge function
    let byte_slices: Vec<&[u8]> = pdf_bytes.iter().map(|v| v.as_slice()).collect();

    merge_documents(&byte_slices).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Get the version of the pdf-merge module.
#[wasm_bindgen]
pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
