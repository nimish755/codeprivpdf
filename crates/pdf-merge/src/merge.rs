/// AI genned comments.

use std::collections::BTreeMap;
use std::io::Cursor;

use lopdf::{Bookmark, Document, Object, ObjectId};
use pdf_core::{PdfError, PdfResult};

/// Merge multiple PDF documents into a single PDF.
///
/// # Arguments
/// * `documents` - Slice of byte slices, each containing a PDF document.
///
/// # Returns
/// The merged PDF as a byte vector.
pub fn merge_documents(documents: &[&[u8]]) -> PdfResult<Vec<u8>> {
    if documents.is_empty() {
        return Err(PdfError::NoPages);
    }

    // Parse all documents
    let mut parsed_docs: Vec<Document> = Vec::with_capacity(documents.len());
    for (i, bytes) in documents.iter().enumerate() {
        let cursor = Cursor::new(*bytes);
        let doc = Document::load_from(cursor)
            .map_err(|e| PdfError::ParseError(format!("Document {}: {}", i + 1, e)))?;
        parsed_docs.push(doc);
    }

    // Merge all documents
    merge_lopdf_documents(parsed_docs)
}

/// Merge parsed lopdf Document instances.
fn merge_lopdf_documents(documents: Vec<Document>) -> PdfResult<Vec<u8>> {
    if documents.is_empty() {
        return Err(PdfError::NoPages);
    }

    if documents.len() == 1 {
        // Single document, just return it
        let mut output = Vec::new();
        documents
            .into_iter()
            .next()
            .unwrap()
            .save_to(&mut output)
            .map_err(|e| PdfError::WriteError(e.to_string()))?;
        return Ok(output);
    }

    // Create new document for merged output
    let mut merged_doc = Document::with_version("1.5");

    // Track max object ID for renumbering
    let mut max_id = 1;
    let mut page_num = 1;

    // Collect all pages and objects from all documents
    let mut all_pages: BTreeMap<ObjectId, Object> = BTreeMap::new();
    let mut all_objects: BTreeMap<ObjectId, Object> = BTreeMap::new();

    for mut doc in documents {
        let mut first_page = true;

        // Renumber objects to avoid ID conflicts
        doc.renumber_objects_with(max_id);
        max_id = doc.max_id + 1;

        // Collect pages with bookmarks
        for (_, object_id) in doc.get_pages() {
            if let Ok(page_obj) = doc.get_object(object_id) {
                // Add bookmark for first page of each document
                if first_page {
                    let bookmark = Bookmark::new(
                        format!("Document {}", page_num),
                        [0.0, 0.0, 1.0], // Blue color
                        0,
                        object_id,
                    );
                    merged_doc.add_bookmark(bookmark, None);
                    first_page = false;
                    page_num += 1;
                }

                all_pages.insert(object_id, page_obj.clone());
            }
        }

        // Collect all objects
        all_objects.extend(doc.objects);
    }

    // Process objects - find Catalog and Pages
    let mut catalog_object: Option<(ObjectId, Object)> = None;
    let mut pages_object: Option<(ObjectId, Object)> = None;

    for (object_id, object) in all_objects.iter() {
        match object.type_name().unwrap_or("") {
            "Catalog" => {
                if catalog_object.is_none() {
                    catalog_object = Some((*object_id, object.clone()));
                }
            }
            "Pages" => {
                // Merge Pages dictionaries
                if let Ok(dictionary) = object.as_dict() {
                    let mut dictionary = dictionary.clone();
                    if let Some((_, ref existing)) = pages_object {
                        if let Ok(old_dict) = existing.as_dict() {
                            dictionary.extend(old_dict);
                        }
                    }
                    pages_object = Some((
                        pages_object.map(|(id, _)| id).unwrap_or(*object_id),
                        Object::Dictionary(dictionary),
                    ));
                }
            }
            "Page" | "Outlines" | "Outline" => {
                // Skip - handled separately
            }
            _ => {
                merged_doc.objects.insert(*object_id, object.clone());
            }
        }
    }

    // Verify we have required objects
    let pages_object = pages_object.ok_or_else(|| {
        PdfError::ParseError("No Pages object found in any document".to_string())
    })?;
    let catalog_object = catalog_object.ok_or_else(|| {
        PdfError::ParseError("No Catalog object found in any document".to_string())
    })?;

    // Update page parent references and insert pages
    for (object_id, object) in all_pages.iter() {
        if let Ok(dict) = object.as_dict() {
            let mut dict = dict.clone();
            dict.set("Parent", pages_object.0);
            merged_doc
                .objects
                .insert(*object_id, Object::Dictionary(dict));
        }
    }

    // Build new Pages object with all children
    if let Ok(dict) = pages_object.1.as_dict() {
        let mut dict = dict.clone();
        dict.set("Count", all_pages.len() as u32);
        dict.set(
            "Kids",
            all_pages
                .keys()
                .map(|&id| Object::Reference(id))
                .collect::<Vec<_>>(),
        );
        merged_doc
            .objects
            .insert(pages_object.0, Object::Dictionary(dict));
    }

    // Build new Catalog
    if let Ok(dict) = catalog_object.1.as_dict() {
        let mut dict = dict.clone();
        dict.set("Pages", pages_object.0);
        dict.remove(b"Outlines"); // Will be rebuilt
        merged_doc
            .objects
            .insert(catalog_object.0, Object::Dictionary(dict));
    }

    // Set trailer root
    merged_doc.trailer.set("Root", catalog_object.0);

    // Update max ID
    merged_doc.max_id = merged_doc.objects.len() as u32;

    // Renumber objects for clean output
    merged_doc.renumber_objects();

    // Fix bookmarks
    merged_doc.adjust_zero_pages();

    // Build outline tree
    if let Some(outline_id) = merged_doc.build_outline() {
        if let Ok(Object::Dictionary(dict)) = merged_doc.get_object_mut(catalog_object.0) {
            dict.set("Outlines", Object::Reference(outline_id));
        }
    }

    // Compress
    merged_doc.compress();

    // Write to bytes
    let mut output = Vec::new();
    merged_doc
        .save_to(&mut output)
        .map_err(|e| PdfError::WriteError(e.to_string()))?;

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to create a minimal valid PDF for testing
    fn create_test_pdf() -> Vec<u8> {
        use lopdf::content::{Content, Operation};
        use lopdf::Stream;

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

        let content = Content {
            operations: vec![
                Operation::new("BT", vec![]),
                Operation::new("Tf", vec!["F1".into(), 48.into()]),
                Operation::new("Td", vec![100.into(), 600.into()]),
                Operation::new("Tj", vec![Object::string_literal("Test Page")]),
                Operation::new("ET", vec![]),
            ],
        };

        let content_id = doc.add_object(Stream::new(dictionary! {}, content.encode().unwrap()));

        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
            "Resources" => resources_id,
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
        });

        let pages = dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
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

    #[test]
    fn test_merge_two_pdfs() {
        let pdf1 = create_test_pdf();
        let pdf2 = create_test_pdf();

        let result = merge_documents(&[&pdf1, &pdf2]);
        assert!(result.is_ok());

        let merged = result.unwrap();
        assert!(!merged.is_empty());

        // Verify merged document has 2 pages
        let cursor = Cursor::new(merged);
        let doc = Document::load_from(cursor).unwrap();
        assert_eq!(doc.get_pages().len(), 2);
    }

    #[test]
    fn test_merge_single_pdf() {
        let pdf = create_test_pdf();
        let result = merge_documents(&[&pdf]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_merge_empty() {
        let result = merge_documents(&[]);
        assert!(result.is_err());
    }
}
