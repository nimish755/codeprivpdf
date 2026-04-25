mod decoder;
mod encoder;

pub use decoder::{decode_image, ImageFilter};
pub use encoder::{
    encode_jpeg, encode_png, encode_smart, downscale_image, is_photo_like, CompressionQuality
};

/// Supported image formats in PDFs. full AI gened this part. i got no idea what happens here
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    /// JPEG image (DCTDecode)
    Jpeg,
    /// PNG-like compressed image (FlateDecode with predictor)
    Png,
    /// Raw uncompressed image
    Raw,
    /// JPEG2000 image (JPXDecode)
    Jpeg2000,
    /// JBIG2 compressed image
    Jbig2,
    /// CCITT fax compression (Group 3/4)
    Ccitt,
    /// Unknown or unsupported format
    Unknown,
}

impl ImageFormat {
    /// Check if this format is supported for decoding.
    pub fn is_supported(&self) -> bool {
        match self {
            ImageFormat::Jpeg | ImageFormat::Png | ImageFormat::Raw => true,
            #[cfg(feature = "jpeg2000")]
            ImageFormat::Jpeg2000 => true,
            #[cfg(feature = "jbig2")]
            ImageFormat::Jbig2 => true,
            _ => false,
        }
    }

    /// Get the format from a PDF filter name.
    pub fn from_filter_name(name: &[u8]) -> Self {
        match name {
            b"DCTDecode" | b"DCT" => ImageFormat::Jpeg,
            b"FlateDecode" | b"Fl" => ImageFormat::Png,
            b"JPXDecode" => ImageFormat::Jpeg2000,
            b"JBIG2Decode" => ImageFormat::Jbig2,
            b"CCITTFaxDecode" | b"CCF" => ImageFormat::Ccitt,
            b"ASCIIHexDecode" | b"AHx" => ImageFormat::Raw,
            b"ASCII85Decode" | b"A85" => ImageFormat::Raw,
            b"LZWDecode" | b"LZW" => ImageFormat::Png, // Similar handling
            b"RunLengthDecode" | b"RL" => ImageFormat::Raw,
            _ => ImageFormat::Unknown,
        }
    }
}
