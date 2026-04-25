
use serde::{Deserialize, Serialize};

/// Compression mode for PDF image optimization.
/// Also does not work cause I AM STUPIIDD
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum CompressionMode {
    Lossless,

    Quality(u8),

    TargetSize(u32),
}

impl CompressionMode {
    /// Create a new quality-based compression mode.
    /// Quality is clamped to 1-100.
    pub fn quality(percent: u8) -> Self {
        CompressionMode::Quality(percent.clamp(1, 100))
    }

    /// Create a new target size compression mode.
    pub fn target_size(bytes: u32) -> Self {
        CompressionMode::TargetSize(bytes)
    }

    /// Check if this is lossless mode.
    pub fn is_lossless(&self) -> bool {
        matches!(self, CompressionMode::Lossless)
    }
}

impl Default for CompressionMode {
    fn default() -> Self {
        CompressionMode::Quality(85)
    }
}

/// A range of pages to extract or manipulate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageRange {
    /// Start page (1-indexed, inclusive)
    pub start: u32,
    /// End page (1-indexed, inclusive)
    pub end: u32,
}

impl PageRange {
    /// Create a new page range.
    pub fn new(start: u32, end: u32) -> Self {
        Self {
            start: start.max(1),
            end: end.max(start),
        }
    }

    /// Create a single page range.
    pub fn single(page: u32) -> Self {
        Self::new(page, page)
    }

    /// Check if the range is valid for a document with the given page count.
    pub fn is_valid(&self, total_pages: u32) -> bool {
        self.start >= 1 && self.start <= self.end && self.end <= total_pages
    }

    /// Convert to 0-indexed page numbers.
    pub fn to_indices(&self) -> impl Iterator<Item = usize> {
        let start = (self.start - 1) as usize;
        let end = self.end as usize;
        start..end
    }

    /// Get the number of pages in this range.
    pub fn len(&self) -> u32 {
        self.end - self.start + 1
    }

    /// Check if the range is empty.
    pub fn is_empty(&self) -> bool {
        false // A range always has at least one page
    }
}

/// Selection of pages for extraction or removal.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PageSelection {
    /// A list of page ranges
    Ranges(Vec<PageRange>),

    /// A list of specific page numbers (1-indexed)
    Pages(Vec<u32>),

    /// All pages
    All,
}

impl PageSelection {
    /// Convert selection to a list of 0-indexed page indices.
    pub fn to_indices(&self, total_pages: u32) -> Vec<usize> {
        match self {
            PageSelection::Ranges(ranges) => {
                ranges
                    .iter()
                    .flat_map(|r| r.to_indices())
                    .filter(|&i| i < total_pages as usize)
                    .collect()
            }
            PageSelection::Pages(pages) => {
                pages
                    .iter()
                    .filter(|&&p| p >= 1 && p <= total_pages)
                    .map(|&p| (p - 1) as usize)
                    .collect()
            }
            PageSelection::All => (0..total_pages as usize).collect(),
        }
    }

    /// Create a selection from a single page.
    pub fn single(page: u32) -> Self {
        PageSelection::Pages(vec![page])
    }

    /// Create a selection from a range.
    pub fn range(start: u32, end: u32) -> Self {
        PageSelection::Ranges(vec![PageRange::new(start, end)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_range_indices() {
        let range = PageRange::new(1, 3);
        let indices: Vec<_> = range.to_indices().collect();
        assert_eq!(indices, vec![0, 1, 2]);
    }

    #[test]
    fn test_compression_mode_quality_clamp() {
        let mode = CompressionMode::quality(150);
        match mode {
            CompressionMode::Quality(q) => assert_eq!(q, 100),
            _ => panic!("Expected Quality variant"),
        }
    }

    #[test]
    fn test_page_selection_to_indices() {
        let selection = PageSelection::Pages(vec![1, 3, 5]);
        let indices = selection.to_indices(10);
        assert_eq!(indices, vec![0, 2, 4]);
    }
}
