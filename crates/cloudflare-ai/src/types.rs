//! Public Cloudflare AI data types.

use std::{fmt, str::FromStr};

use crate::error::Error;

/// A generated image, normalized to raw bytes so consumers never deal with
/// provider transport details (binary vs base64 vs URL).
#[derive(Debug, Clone)]
pub struct GeneratedImage {
    pub bytes: Vec<u8>,
    pub mime: String,
}

/// How a model's prompt must be encoded on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Input {
    /// `application/json` body — `{"prompt": "..."}`.
    Json,
    /// `multipart/form-data` body with a `prompt` field.
    Multipart,
}

/// A Cloudflare Workers AI text-to-image model.
///
/// [`Model::path`] carries the `@cf/…` identifier used in the REST URL;
/// consumers deal in the short, human-friendly names of [`Model::from_alias`]
/// and never hard-code provider paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum Model {
    /// Flux 2 Dev — highest-quality Flux.
    Flux2Dev,
    /// Flux 1 Schnell — fast Flux, the default.
    #[default]
    Flux1Schnell,
    /// Flux 2 Klein 4B — smaller Flux 2.
    Flux2Klein4b,
    /// Flux 2 Klein 9B — larger of the two Klein variants.
    Flux2Klein9b,
    /// Stable Diffusion XL Lightning (ByteDance).
    SdXlLightning,
    /// Dreamshaper 8 LCM (Lykon).
    Dreamshaper8Lcm,
    /// Stable Diffusion XL Base 1.0 (Stability AI).
    SdXlBase1,
}

impl Model {
    /// Every supported model, in the order shown to users.
    pub const ALL: &[Model] = &[
        Model::Flux2Dev,
        Model::Flux1Schnell,
        Model::Flux2Klein4b,
        Model::Flux2Klein9b,
        Model::SdXlLightning,
        Model::Dreamshaper8Lcm,
        Model::SdXlBase1,
    ];

    /// The canonical short name — the primary token users type.
    pub fn as_str(self) -> &'static str {
        self.aliases()[0]
    }

    /// The Cloudflare `@cf/…` path used in the REST URL.
    pub fn path(self) -> &'static str {
        match self {
            Model::Flux2Dev => "@cf/black-forest-labs/flux-2-dev",
            Model::Flux1Schnell => "@cf/black-forest-labs/flux-1-schnell",
            Model::Flux2Klein4b => "@cf/black-forest-labs/flux-2-klein-4b",
            Model::Flux2Klein9b => "@cf/black-forest-labs/flux-2-klein-9b",
            Model::SdXlLightning => "@cf/bytedance/stable-diffusion-xl-lightning",
            Model::Dreamshaper8Lcm => "@cf/lykon/dreamshaper-8-lcm",
            Model::SdXlBase1 => "@cf/stabilityai/stable-diffusion-xl-base-1.0",
        }
    }

    /// How the prompt must be encoded for this model.
    pub fn input(self) -> Input {
        match self {
            Model::Flux2Dev | Model::Flux2Klein4b | Model::Flux2Klein9b => Input::Multipart,
            Model::Flux1Schnell
            | Model::SdXlLightning
            | Model::Dreamshaper8Lcm
            | Model::SdXlBase1 => Input::Json,
        }
    }

    /// A short human description, for help text and menus.
    pub fn description(self) -> &'static str {
        match self {
            Model::Flux2Dev => "Flux 2 Dev — best quality",
            Model::Flux1Schnell => "Flux 1 Schnell — fast (default)",
            Model::Flux2Klein4b => "Flux 2 Klein 4B — compact",
            Model::Flux2Klein9b => "Flux 2 Klein 9B — compact",
            Model::SdXlLightning => "SDXL Lightning — fast",
            Model::Dreamshaper8Lcm => "Dreamshaper 8 LCM — fast",
            Model::SdXlBase1 => "SDXL Base 1.0",
        }
    }

    /// All accepted tokens, canonical name first.
    pub fn aliases(self) -> &'static [&'static str] {
        match self {
            Model::Flux2Dev => &["flux-2-dev", "flux2dev"],
            Model::Flux1Schnell => &["flux-1-schnell", "flux-schnell", "schnell"],
            Model::Flux2Klein4b => &["flux-2-klein-4b", "klein-4b"],
            Model::Flux2Klein9b => &["flux-2-klein-9b", "klein-9b"],
            Model::SdXlLightning => &["sd-xl-lightning", "sdxl-lightning", "sd-lightning"],
            Model::Dreamshaper8Lcm => &["dreamshaper-8-lcm", "dreamshaper", "lcm"],
            Model::SdXlBase1 => &["sd-xl-base", "sdxl-base", "sd-base"],
        }
    }

    /// Match user input against [`Model::aliases`], case-insensitively.
    pub fn from_alias(s: &str) -> Option<Self> {
        let s = s.trim();
        Self::ALL
            .iter()
            .copied()
            .find(|m| m.aliases().iter().any(|a| s.eq_ignore_ascii_case(a)))
    }
}

impl fmt::Display for Model {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Model {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_alias(s).ok_or_else(|| Error::InvalidModel(s.trim().to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_flux_1_schnell() {
        assert_eq!(Model::default(), Model::Flux1Schnell);
        assert_eq!(
            Model::default().path(),
            "@cf/black-forest-labs/flux-1-schnell"
        );
    }

    #[test]
    fn from_alias_matches_canonical_and_aliases_case_insensitively() {
        assert_eq!(Model::from_alias("flux-2-dev"), Some(Model::Flux2Dev));
        assert_eq!(Model::from_alias("FLUX2DEV"), Some(Model::Flux2Dev));
        assert_eq!(Model::from_alias("schnell"), Some(Model::Flux1Schnell));
        assert_eq!(Model::from_alias("sd-xl-base"), Some(Model::SdXlBase1));
    }

    #[test]
    fn from_alias_rejects_unknown_tokens() {
        assert_eq!(Model::from_alias("flux"), None);
        assert_eq!(Model::from_alias(""), None);
    }

    #[test]
    fn from_str_reports_the_bad_token() {
        let err = "flux".parse::<Model>().unwrap_err();
        assert!(err.to_string().contains("flux"));
    }

    #[test]
    fn display_uses_canonical_name() {
        assert_eq!(Model::Flux2Dev.to_string(), "flux-2-dev");
        assert_eq!(Model::SdXlBase1.to_string(), "sd-xl-base");
    }

    #[test]
    fn input_encoding_by_model() {
        assert_eq!(Model::Flux2Dev.input(), Input::Multipart);
        assert_eq!(Model::Flux2Klein4b.input(), Input::Multipart);
        assert_eq!(Model::Flux2Klein9b.input(), Input::Multipart);
        assert_eq!(Model::Flux1Schnell.input(), Input::Json);
        assert_eq!(Model::SdXlLightning.input(), Input::Json);
        assert_eq!(Model::Dreamshaper8Lcm.input(), Input::Json);
        assert_eq!(Model::SdXlBase1.input(), Input::Json);
    }
}
