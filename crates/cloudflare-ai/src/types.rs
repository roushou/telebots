//! Public Cloudflare AI data types.

/// A generated image, normalized to raw bytes so consumers never deal with
/// provider transport details (binary vs base64 vs URL).
#[derive(Debug, Clone)]
pub struct GeneratedImage {
    pub bytes: Vec<u8>,
    pub mime: String,
}
