//! Shared PDF utility functions for object tree traversal and manipulation.

use std::collections::BTreeMap;
use std::io::Cursor;

use lopdf::{Document, Object, ObjectId};

use crate::{PdfError, PdfResult};

/// Get the number of pages in a PDF document.
///
/// # Arguments
/// * `pdf_bytes` - The PDF document as bytes.
///
/// # Returns
/// The number of pages in the document.
pub fn get_page_count(pdf_bytes: &[u8]) -> PdfResult<u32> {
    let cursor = Cursor::new(pdf_bytes);
    let doc = Document::load_from(cursor).map_err(|e| PdfError::ParseError(e.to_string()))?;
    Ok(doc.get_pages().len() as u32)
}

/// Recursively collect a PDF object and all objects it references.
///
/// This function traverses the object graph starting from the given object ID,
/// collecting all referenced objects into a map. It avoids circular references
/// by skipping Pages objects and Parent references.
///
/// # Arguments
/// * `doc` - The PDF document to collect objects from.
/// * `object_id` - The starting object ID.
/// * `collected` - A map to store collected objects.
pub fn collect_object_tree(
    doc: &Document,
    object_id: ObjectId,
    collected: &mut BTreeMap<ObjectId, Object>,
) -> PdfResult<()> {
    if collected.contains_key(&object_id) {
        return Ok(());
    }

    let object = doc
        .get_object(object_id)
        .map_err(|e| PdfError::ParseError(format!("Failed to get object {:?}: {}", object_id, e)))?
        .clone();

    collected.insert(object_id, object.clone());

    collect_references(&object, doc, collected)?;

    Ok(())
}

/// Collect all referenced objects from a PDF object.
///
/// This is a helper function for `collect_object_tree` that recursively
/// traverses arrays, dictionaries, and streams to find object references.
///
/// # Arguments
/// * `object` - The object to scan for references.
/// * `doc` - The PDF document.
/// * `collected` - A map to store collected objects.
pub fn collect_references(
    object: &Object,
    doc: &Document,
    collected: &mut BTreeMap<ObjectId, Object>,
) -> PdfResult<()> {
    match object {
        Object::Reference(id) => {
            // Skip Pages objects to avoid circular references
            if let Ok(obj) = doc.get_object(*id) {
                if let Ok(dict) = obj.as_dict() {
                    let is_pages = dict
                        .get(b"Type")
                        .ok()
                        .and_then(|t| t.as_name().ok())
                        .map(|n| n == b"Pages")
                        .unwrap_or(false);
                    if is_pages {
                        return Ok(());
                    }
                }
            }
            collect_object_tree(doc, *id, collected)?;
        }
        Object::Array(arr) => {
            for item in arr {
                collect_references(item, doc, collected)?;
            }
        }
        Object::Dictionary(dict) => {
            for (key, value) in dict.iter() {
                // Skip Parent key to avoid circular references
                if key == b"Parent" {
                    continue;
                }
                collect_references(value, doc, collected)?;
            }
        }
        Object::Stream(stream) => {
            for (key, value) in stream.dict.iter() {
                if key == b"Parent" {
                    continue;
                }
                collect_references(value, doc, collected)?;
            }
        }
        _ => {}
    }

    Ok(())
}

/// Update all object references in an object using an ID mapping.
///
/// This function recursively traverses the object and replaces any
/// object references with their mapped values.
///
/// # Arguments
/// * `object` - The object to update (mutated in place).
/// * `mapping` - A map from old object IDs to new object IDs.
pub fn update_object_references(object: &mut Object, mapping: &BTreeMap<ObjectId, ObjectId>) {
    match object {
        Object::Reference(id) => {
            if let Some(&new_id) = mapping.get(id) {
                *id = new_id;
            }
        }
        Object::Array(arr) => {
            for item in arr.iter_mut() {
                update_object_references(item, mapping);
            }
        }
        Object::Dictionary(dict) => {
            let keys: Vec<_> = dict.iter().map(|(k, _)| k.clone()).collect();
            for key in keys {
                if let Ok(value) = dict.get_mut(&key) {
                    update_object_references(value, mapping);
                }
            }
        }
        Object::Stream(stream) => {
            let keys: Vec<_> = stream.dict.iter().map(|(k, _)| k.clone()).collect();
            for key in keys {
                if let Ok(value) = stream.dict.get_mut(&key) {
                    update_object_references(value, mapping);
                }
            }
        }
        _ => {}
    }
}

/// Test utilities for creating PDF documents.
/// Only available when the `testutils` feature is enabled.
#[cfg(feature = "testutils")]
pub mod testutils {
    use lopdf::content::{Content, Operation};
    use lopdf::{dictionary, Document, Object, Stream};

    /// Create a multi-page PDF for testing purposes.
    ///
    /// # Arguments
    /// * `num_pages` - The number of pages to create.
    ///
    /// # Returns
    /// The PDF document as bytes.
    pub fn create_multi_page_pdf(num_pages: u32) -> Vec<u8> {
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();

        let font_id = doc.add_object(dictionary! {
            "Type" => "Font",
            "Subtype" => "Type1",
            "BaseFont" => "Courier",
        });

        let resources_id = doc.add_object(dictionary! {
            "Font" => dictionary! {
                "F1" => font_id,
            },
        });

        let mut page_ids = Vec::new();

        for i in 1..=num_pages {
            let content = Content {
                operations: vec![
                    Operation::new("BT", vec![]),
                    Operation::new("Tf", vec!["F1".into(), 24.into()]),
                    Operation::new("Td", vec![100.into(), 700.into()]),
                    Operation::new("Tj", vec![Object::string_literal(format!("Page {}", i))]),
                    Operation::new("ET", vec![]),
                ],
            };

            let content_id =
                doc.add_object(Stream::new(dictionary! {}, content.encode().unwrap()));

            let page_id = doc.add_object(dictionary! {
                "Type" => "Page",
                "Parent" => pages_id,
                "Contents" => content_id,
                "Resources" => resources_id,
                "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
            });

            page_ids.push(page_id);
        }

        let pages = dictionary! {
            "Type" => "Pages",
            "Kids" => page_ids.iter().map(|&id| Object::Reference(id)).collect::<Vec<_>>(),
            "Count" => num_pages,
            "Resources" => resources_id,
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
        };
        doc.objects.insert(pages_id, Object::Dictionary(pages));

        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });

        doc.trailer.set("Root", catalog_id);

        let mut output = Vec::new();
        doc.save_to(&mut output).unwrap();
        output
    }
}
