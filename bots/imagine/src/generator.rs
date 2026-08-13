//! The image generator: a provider abstraction.
//!
//! Bots depend on this enum, never on a provider crate directly. Each
//! variant wraps a provider client; [`Generator::generate`] normalizes the
//! provider's output to a validated [`GeneratedImage`], so swapping
//! providers only touches this module and the `Ctx` wiring. The generated
//! image also knows how to produce a compact JPEG copy for long-term
//! storage (the record log), keeping the full-resolution PNG for delivery.

use std::io::Cursor;

use anyhow::{Context, Result};
use cloudflare_ai::{CloudflareAiClient, Model};
use image::{DynamicImage, GenericImageView, ImageFormat, Rgb, RgbImage, imageops::FilterType};

/// Longest edge of the compact copy stored in the record log.
const COMPACT_MAX_EDGE: u32 = 512;

/// A generated image, normalized to bytes for sending via Telegram.
#[derive(Debug, Clone)]
pub struct GeneratedImage {
    pub bytes: Vec<u8>,
}

impl GeneratedImage {
    /// Wrap provider bytes, validating that they decode as an image.
    pub fn new(bytes: Vec<u8>) -> Result<Self> {
        image::load_from_memory(&bytes).context("generated image is not a valid image")?;
        Ok(Self { bytes })
    }

    /// A downscaled JPEG copy for long-term storage: the longest edge is
    /// capped at [`COMPACT_MAX_EDGE`] and any transparency is flattened
    /// onto white, since JPEG has no alpha channel.
    pub fn compact(&self) -> Result<Vec<u8>> {
        let img =
            image::load_from_memory(&self.bytes).context("failed to decode generated image")?;
        let img = Self::downscale(img);
        let flat = Self::flatten_alpha(&img);
        let mut out = Vec::new();
        flat.write_to(&mut Cursor::new(&mut out), ImageFormat::Jpeg)
            .context("failed to encode compact JPEG")?;
        Ok(out)
    }

    /// Shrink an image so its longest edge is at most [`COMPACT_MAX_EDGE`],
    /// keeping the aspect ratio. Smaller images pass through unchanged.
    fn downscale(img: DynamicImage) -> DynamicImage {
        let (w, h) = img.dimensions();
        let max = w.max(h);
        if max <= COMPACT_MAX_EDGE {
            return img;
        }
        let tw = (w * COMPACT_MAX_EDGE / max).max(1);
        let th = (h * COMPACT_MAX_EDGE / max).max(1);
        img.resize(tw, th, FilterType::Lanczos3)
    }

    /// Composite an image onto a white background, producing an opaque RGB
    /// image suitable for JPEG encoding.
    fn flatten_alpha(img: &DynamicImage) -> RgbImage {
        let rgba = img.to_rgba8();
        let (w, h) = rgba.dimensions();
        let mut out = RgbImage::new(w, h);
        for (x, y, px) in rgba.enumerate_pixels() {
            let [r, g, b, a] = px.0;
            let t = a as f32 / 255.0;
            out.put_pixel(
                x,
                y,
                Rgb([
                    (r as f32 * t + 255.0 * (1.0 - t)) as u8,
                    (g as f32 * t + 255.0 * (1.0 - t)) as u8,
                    (b as f32 * t + 255.0 * (1.0 - t)) as u8,
                ]),
            );
        }
        out
    }
}

/// The configured image provider.
#[derive(Clone)]
pub enum Generator {
    Cloudflare(CloudflareAiClient),
}

impl Generator {
    pub fn cloudflare(account_id: String, api_token: String) -> Result<Self> {
        Ok(Self::Cloudflare(CloudflareAiClient::new(
            account_id, api_token,
        )?))
    }

    /// Generate an image from `prompt` using `model`.
    pub async fn generate(&self, model: Model, prompt: &str) -> Result<GeneratedImage> {
        match self {
            Generator::Cloudflare(client) => {
                let image = client.generate_image(model, prompt).await?;
                GeneratedImage::new(image.bytes)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use image::{ImageFormat, Rgb, RgbImage};

    use super::*;

    fn solid_png(w: u32, h: u32, rgb: [u8; 3]) -> Vec<u8> {
        let mut out = Vec::new();
        RgbImage::from_pixel(w, h, Rgb(rgb))
            .write_to(&mut Cursor::new(&mut out), ImageFormat::Png)
            .unwrap();
        out
    }

    /// A 64x64 PNG: top-left quarter fully transparent, the rest opaque red.
    fn transparent_png() -> Vec<u8> {
        let mut img = image::RgbaImage::new(64, 64);
        for (x, y, px) in img.enumerate_pixels_mut() {
            *px = if x < 32 && y < 32 {
                image::Rgba([10, 20, 30, 0])
            } else {
                image::Rgba([200, 100, 50, 255])
            };
        }
        let mut out = Vec::new();
        img.write_to(&mut Cursor::new(&mut out), ImageFormat::Png)
            .unwrap();
        out
    }

    #[test]
    fn new_rejects_garbage() {
        assert!(GeneratedImage::new(b"not an image".to_vec()).is_err());
    }

    #[test]
    fn new_accepts_valid_png() {
        assert!(GeneratedImage::new(solid_png(4, 4, [1, 2, 3])).is_ok());
    }

    #[test]
    fn compact_encodes_jpeg() {
        let img = GeneratedImage::new(solid_png(8, 8, [200, 100, 50])).unwrap();
        let jpeg = img.compact().unwrap();
        assert_eq!(&jpeg[..3], &[0xFF, 0xD8, 0xFF]); // JPEG magic
        let decoded = image::load_from_memory(&jpeg).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (8, 8));
    }

    #[test]
    fn compact_does_not_upscale_small_images() {
        let img = GeneratedImage::new(solid_png(8, 8, [1, 2, 3])).unwrap();
        let jpeg = img.compact().unwrap();
        let decoded = image::load_from_memory(&jpeg).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (8, 8));
    }

    #[test]
    fn compact_downscales_large_images() {
        let img = GeneratedImage::new(solid_png(1024, 768, [1, 2, 3])).unwrap();
        let jpeg = img.compact().unwrap();
        let decoded = image::load_from_memory(&jpeg).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (512, 384));
    }

    #[test]
    fn compact_flattens_transparency_to_white() {
        let img = GeneratedImage::new(transparent_png()).unwrap();
        let jpeg = img.compact().unwrap();
        let decoded = image::load_from_memory(&jpeg).unwrap().to_rgb8();
        // JPEG is lossy, so compare with a small tolerance.
        let close = |a: [u8; 3], b: [u8; 3]| a.into_iter().zip(b).all(|(x, y)| x.abs_diff(y) <= 15);
        let transparent = decoded.get_pixel(16, 16).0;
        assert!(
            close(transparent, [255, 255, 255]),
            "transparent area should flatten to white, got {transparent:?}"
        );
        let opaque = decoded.get_pixel(48, 48).0;
        assert!(
            close(opaque, [200, 100, 50]),
            "opaque area should keep its color, got {opaque:?}"
        );
    }
}
