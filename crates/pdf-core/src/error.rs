use thiserror::Error;

pub type PdfResult<T> = Result<T, PdfError>;

#[derive(Error, Debug)]
pub enum PdfError {
    /// Failed to parse PDF document
    #[error("Failed to parse PDF: {0}")]
    ParseError(String),

    /// Invalid page number requested
    #[error("Invalid page number: {page}. Document has {total} pages.")]
    InvalidPage { page: u32, total: u32 },

    /// Invalid page range
    #[error("Invalid page range: {0}")]
    InvalidRange(String),

    /// No pages to process
    #[error("No pages to process")]
    NoPages,

    /// Failed to write PDF
    #[error("Failed to write PDF: {0}")]
    WriteError(String),

    /// Image processing error
    #[error("Image processing error: {0}")]
    ImageError(String),

    /// Unsupported image format
    #[error("Unsupported image format: {0}")]
    UnsupportedFormat(String),

    /// Compression error
    #[error("Compression error: {0}")]
    CompressionError(String),

    /// Rendering error
    #[error("Rendering error: {0}")]
    RenderError(String),

    /// Generic internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<std::io::Error> for PdfError {
    fn from(err: std::io::Error) -> Self {
        PdfError::WriteError(err.to_string())
    }
}
