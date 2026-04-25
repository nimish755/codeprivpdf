/// AI genned comments.

use std::collections::{BTreeMap, HashSet};
use std::io::Cursor;

use lopdf::{dictionary, Document, Object, ObjectId};
use pdf_core::{collect_object_tree, update_object_references, PdfError, PdfResult};

/// Remove specific pages from a PDF.
pub fn remove_pages_from_pdf(pdf_bytes: &[u8], pages_to_remove: &[u32]) -> PdfResult<Vec<u8>> {
    let cursor = Cursor::new(pdf_bytes);
    let doc = Document::load_from(cursor).map_err(|e| PdfError::ParseError(e.to_string()))?;

    let all_pages: Vec<(u32, ObjectId)> = doc.get_pages().into_iter().collect();
    let page_count = all_pages.len() as u32;

    if page_count == 0 {
        return Err(PdfError::NoPages);
    }

    // Convert to set for efficient lookup
    let remove_set: HashSet<u32> = pages_to_remove.iter().cloned().collect();

    // Validate page numbers
    for &page_num in pages_to_remove {
        if page_num < 1 || page_num > page_count {
            return Err(PdfError::InvalidPage {
                page: page_num,
                total: page_count,
            });
        }
    }

    // Determine which pages to keep
    let pages_to_keep: Vec<u32> = (1..=page_count)
        .filter(|p| !remove_set.contains(p))
        .collect();

    if pages_to_keep.is_empty() {
        return Err(PdfError::NoPages);
    }

    // Extract the pages we want to keep
    extract_pages_from_pdf(pdf_bytes, &pages_to_keep)
}

/// Extract specific pages from a PDF into a new document.
pub fn extract_pages_from_pdf(pdf_bytes: &[u8], page_numbers: &[u32]) -> PdfResult<Vec<u8>> {
    let cursor = Cursor::new(pdf_bytes);
    let doc = Document::load_from(cursor).map_err(|e| PdfError::ParseError(e.to_string()))?;

    let all_pages: Vec<(u32, ObjectId)> = doc.get_pages().into_iter().collect();
    let page_count = all_pages.len() as u32;

    if page_count == 0 {
        return Err(PdfError::NoPages);
    }

    // Validate and collect page object IDs
    let mut page_ids_to_extract: Vec<ObjectId> = Vec::with_capacity(page_numbers.len());
    for &page_num in page_numbers {
        if page_num < 1 || page_num > page_count {
            return Err(PdfError::InvalidPage {
                page: page_num,
                total: page_count,
            });
        }
        // Pages are 1-indexed in the input, but our array is 0-indexed
        let (_, page_id) = all_pages[(page_num - 1) as usize];
        page_ids_to_extract.push(page_id);
    }

    if page_ids_to_extract.is_empty() {
        return Err(PdfError::NoPages);
    }

    // Build new document with selected pages
    build_document_with_pages(&doc, &page_ids_to_extract)
}

/// Reorder pages in a PDF.
pub fn reorder_pages_in_pdf(pdf_bytes: &[u8], new_order: &[u32]) -> PdfResult<Vec<u8>> {
    let cursor = Cursor::new(pdf_bytes);
    let doc = Document::load_from(cursor).map_err(|e| PdfError::ParseError(e.to_string()))?;

    let all_pages: Vec<(u32, ObjectId)> = doc.get_pages().into_iter().collect();
    let page_count = all_pages.len() as u32;

    if page_count == 0 {
        return Err(PdfError::NoPages);
    }

    // Validate new order
    if new_order.len() != page_count as usize {
        return Err(PdfError::InvalidRange(format!(
            "New order has {} entries but document has {} pages",
            new_order.len(),
            page_count
        )));
    }

    // Check for duplicates and valid page numbers
    let mut seen: HashSet<u32> = HashSet::new();
    for &page_num in new_order {
        if page_num < 1 || page_num > page_count {
            return Err(PdfError::InvalidPage {
                page: page_num,
                total: page_count,
            });
        }
        if !seen.insert(page_num) {
            return Err(PdfError::InvalidRange(format!(
                "Duplicate page number in new order: {}",
                page_num
            )));
        }
    }

    // Get page IDs in new order
    let page_ids_in_order: Vec<ObjectId> = new_order
        .iter()
        .map(|&page_num| all_pages[(page_num - 1) as usize].1)
        .collect();

    build_document_with_pages(&doc, &page_ids_in_order)
}

/// Build a new PDF document containing only the specified pages.
fn build_document_with_pages(source_doc: &Document, page_ids: &[ObjectId]) -> PdfResult<Vec<u8>> {
    let mut new_doc = Document::with_version("1.5");
    let _new_pages_id = new_doc.new_object_id();

    // Collect all objects needed for the selected pages
    let mut objects_to_copy: BTreeMap<ObjectId, Object> = BTreeMap::new();

    for &page_id in page_ids {
        collect_object_tree(source_doc, page_id, &mut objects_to_copy)?;
    }

    // Create ID mapping for new document
    let mut id_mapping: BTreeMap<ObjectId, ObjectId> = BTreeMap::new();
    let mut next_id = 1u32;

    for &old_id in objects_to_copy.keys() {
        id_mapping.insert(old_id, (next_id, 0));
        next_id += 1;
    }

    // Reserve ID for Pages object
    let actual_pages_id = (next_id, 0);
    next_id += 1;

    // Copy objects with updated references
    for (old_id, object) in &objects_to_copy {
        let new_id = id_mapping[old_id];
        let mut new_object = object.clone();

        update_object_references(&mut new_object, &id_mapping);

        // Update Parent reference for Page objects
        if let Ok(dict) = new_object.as_dict_mut() {
            let is_page = dict.get(b"Type")
                .ok()
                .and_then(|t| t.as_name().ok())
                .map(|n| n == b"Page")
                .unwrap_or(false);
            if is_page {
                dict.set("Parent", actual_pages_id);
            }
        }

        new_doc.objects.insert(new_id, new_object);
    }

    // Create Pages object
    let page_refs: Vec<Object> = page_ids
        .iter()
        .filter_map(|&old_id| id_mapping.get(&old_id).map(|&new_id| Object::Reference(new_id)))
        .collect();

    let pages_dict = dictionary! {
        "Type" => "Pages",
        "Kids" => page_refs,
        "Count" => page_ids.len() as u32,
    };
    new_doc
        .objects
        .insert(actual_pages_id, Object::Dictionary(pages_dict));

    // Create Catalog
    let catalog_id = (next_id, 0);
    let catalog = dictionary! {
        "Type" => "Catalog",
        "Pages" => actual_pages_id,
    };
    new_doc.objects.insert(catalog_id, Object::Dictionary(catalog));

    // Set trailer
    new_doc.trailer.set("Root", catalog_id);
    new_doc.max_id = next_id;

    // Compress
    new_doc.compress();

    // Write output
    let mut output = Vec::new();
    new_doc
        .save_to(&mut output)
        .map_err(|e| PdfError::WriteError(e.to_string()))?;

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdf_core::testutils::create_multi_page_pdf;

    #[test]
    fn test_remove_pages() {
        let pdf = create_multi_page_pdf(5);
        let result = remove_pages_from_pdf(&pdf, &[2, 4]).unwrap();

        let cursor = Cursor::new(result);
        let doc = Document::load_from(cursor).unwrap();
        assert_eq!(doc.get_pages().len(), 3);
    }

    #[test]
    fn test_extract_pages() {
        let pdf = create_multi_page_pdf(5);
        let result = extract_pages_from_pdf(&pdf, &[1, 3, 5]).unwrap();

        let cursor = Cursor::new(result);
        let doc = Document::load_from(cursor).unwrap();
        assert_eq!(doc.get_pages().len(), 3);
    }

    #[test]
    fn test_reorder_pages() {
        let pdf = create_multi_page_pdf(3);
        let result = reorder_pages_in_pdf(&pdf, &[3, 1, 2]).unwrap();

        let cursor = Cursor::new(result);
        let doc = Document::load_from(cursor).unwrap();
        assert_eq!(doc.get_pages().len(), 3);
    }

    #[test]
    fn test_remove_all_pages_error() {
        let pdf = create_multi_page_pdf(3);
        let result = remove_pages_from_pdf(&pdf, &[1, 2, 3]);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_page_number() {
        let pdf = create_multi_page_pdf(3);
        let result = extract_pages_from_pdf(&pdf, &[5]);
        assert!(result.is_err());
    }
}
