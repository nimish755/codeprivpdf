mod error;
mod types;
pub mod utils;

pub use error::{PdfError, PdfResult};
pub use types::{CompressionMode, PageRange, PageSelection};
pub use utils::{collect_object_tree, collect_references, get_page_count, update_object_references};

#[cfg(feature = "testutils")]
pub use utils::testutils;
