/// AI genned comments.

use std::collections::BTreeMap;
use std::io::Cursor;

use lopdf::{dictionary, Document, Object, ObjectId};
use pdf_core::{collect_object_tree, update_object_references, PageRange, PdfError, PdfResult};

/// Split a PDF into individual single-page PDFs.
pub fn split_into_pages(pdf_bytes: &[u8]) -> PdfResult<Vec<Vec<u8>>> {
    let cursor = Cursor::new(pdf_bytes);
    let doc = Document::load_from(cursor).map_err(|e| PdfError::ParseError(e.to_string()))?;

    let pages = doc.get_pages();
    let page_count = pages.len() as u32;

    if page_count == 0 {
        return Err(PdfError::NoPages);
    }

    // Create a range for each page
    let ranges: Vec<PageRange> = (1..=page_count).map(PageRange::single).collect();

    split_document_by_ranges(&doc, &ranges)
}

/// Split a PDF by custom page ranges.
pub fn split_by_ranges(pdf_bytes: &[u8], ranges: &[PageRange]) -> PdfResult<Vec<Vec<u8>>> {
    let cursor = Cursor::new(pdf_bytes);
    let doc = Document::load_from(cursor).map_err(|e| PdfError::ParseError(e.to_string()))?;

    split_document_by_ranges(&doc, ranges)
}

/// Split a loaded document by page ranges.
fn split_document_by_ranges(doc: &Document, ranges: &[PageRange]) -> PdfResult<Vec<Vec<u8>>> {
    let pages: Vec<(u32, ObjectId)> = doc.get_pages().into_iter().collect();
    let page_count = pages.len() as u32;

    if page_count == 0 {
        return Err(PdfError::NoPages);
    }

    // Validate all ranges
    for range in ranges {
        if !range.is_valid(page_count) {
            return Err(PdfError::InvalidRange(format!(
                "Range {}-{} is invalid for document with {} pages",
                range.start, range.end, page_count
            )));
        }
    }

    let mut results = Vec::with_capacity(ranges.len());

    for range in ranges {
        let extracted = extract_page_range(doc, &pages, range)?;
        results.push(extracted);
    }

    Ok(results)
}

/// Extract a single range of pages into a new PDF.
fn extract_page_range(
    source_doc: &Document,
    pages: &[(u32, ObjectId)],
    range: &PageRange,
) -> PdfResult<Vec<u8>> {
    let mut new_doc = Document::with_version("1.5");
    let new_pages_id = new_doc.new_object_id();

    // Collect objects that need to be copied
    let mut objects_to_copy: BTreeMap<ObjectId, Object> = BTreeMap::new();
    let mut new_page_ids: Vec<ObjectId> = Vec::new();

    // Get the page object IDs for the requested range
    let page_indices: Vec<usize> = range.to_indices().collect();

    for &page_idx in &page_indices {
        if page_idx >= pages.len() {
            continue;
        }

        let (_, page_id) = pages[page_idx];

        // Get the page object and all its dependencies
        collect_object_tree(source_doc, page_id, &mut objects_to_copy)?;
        new_page_ids.push(page_id);
    }

    if new_page_ids.is_empty() {
        return Err(PdfError::NoPages);
    }

    // Find resources that are shared (fonts, etc.)
    // Copy all collected objects, updating parent references
    let mut id_mapping: BTreeMap<ObjectId, ObjectId> = BTreeMap::new();
    let mut next_id = 1u32;

    // Create ID mapping
    for &old_id in objects_to_copy.keys() {
        let new_id = (next_id, 0);
        id_mapping.insert(old_id, new_id);
        next_id += 1;
    }

    // Map pages_id
    id_mapping.insert(new_pages_id, (next_id, 0));
    let actual_pages_id = (next_id, 0);
    next_id += 1;

    // Copy objects with updated references
    for (old_id, object) in objects_to_copy {
        let new_id = id_mapping[&old_id];
        let mut new_object = object.clone();

        // Update references in the object
        update_object_references(&mut new_object, &id_mapping);

        // If this is a page, update its Parent reference
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

    // Create new Pages object
    let new_page_refs: Vec<Object> = new_page_ids
        .iter()
        .filter_map(|&old_id| id_mapping.get(&old_id).map(|&new_id| Object::Reference(new_id)))
        .collect();

    let pages_dict = dictionary! {
        "Type" => "Pages",
        "Kids" => new_page_refs,
        "Count" => page_indices.len() as u32,
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

    // Write to bytes
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
    fn test_split_into_pages() {
        let pdf = create_multi_page_pdf(5);
        let pages = split_into_pages(&pdf).unwrap();

        assert_eq!(pages.len(), 5);

        // Each result should be a valid PDF with 1 page
        for page_pdf in pages {
            let cursor = Cursor::new(page_pdf);
            let doc = Document::load_from(cursor).unwrap();
            assert_eq!(doc.get_pages().len(), 1);
        }
    }

    #[test]
    fn test_split_by_ranges() {
        let pdf = create_multi_page_pdf(10);
        let ranges = vec![
            PageRange::new(1, 3),  // Pages 1-3
            PageRange::new(5, 7),  // Pages 5-7
            PageRange::new(10, 10), // Page 10
        ];

        let parts = split_by_ranges(&pdf, &ranges).unwrap();

        assert_eq!(parts.len(), 3);

        // Verify page counts
        let cursor1 = Cursor::new(&parts[0]);
        let doc1 = Document::load_from(cursor1).unwrap();
        assert_eq!(doc1.get_pages().len(), 3);

        let cursor2 = Cursor::new(&parts[1]);
        let doc2 = Document::load_from(cursor2).unwrap();
        assert_eq!(doc2.get_pages().len(), 3);

        let cursor3 = Cursor::new(&parts[2]);
        let doc3 = Document::load_from(cursor3).unwrap();
        assert_eq!(doc3.get_pages().len(), 1);
    }

    #[test]
    fn test_invalid_range() {
        let pdf = create_multi_page_pdf(5);
        let ranges = vec![PageRange::new(1, 10)];

        let result = split_by_ranges(&pdf, &ranges);
        assert!(result.is_err());
    }
}
