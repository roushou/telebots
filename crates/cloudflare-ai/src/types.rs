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
/// [`ImageModel::path`] carries the `@cf/…` identifier used in the REST URL;
/// consumers deal in the short, human-friendly names of [`ImageModel::from_alias`]
/// and never hard-code provider paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum ImageModel {
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

impl ImageModel {
    /// Every supported model, in the order shown to users.
    pub const ALL: &[ImageModel] = &[
        ImageModel::Flux2Dev,
        ImageModel::Flux1Schnell,
        ImageModel::Flux2Klein4b,
        ImageModel::Flux2Klein9b,
        ImageModel::SdXlLightning,
        ImageModel::Dreamshaper8Lcm,
        ImageModel::SdXlBase1,
    ];

    /// The canonical short name — the primary token users type.
    pub fn as_str(self) -> &'static str {
        self.aliases()[0]
    }

    /// The Cloudflare `@cf/…` path used in the REST URL.
    pub fn path(self) -> &'static str {
        match self {
            ImageModel::Flux2Dev => "@cf/black-forest-labs/flux-2-dev",
            ImageModel::Flux1Schnell => "@cf/black-forest-labs/flux-1-schnell",
            ImageModel::Flux2Klein4b => "@cf/black-forest-labs/flux-2-klein-4b",
            ImageModel::Flux2Klein9b => "@cf/black-forest-labs/flux-2-klein-9b",
            ImageModel::SdXlLightning => "@cf/bytedance/stable-diffusion-xl-lightning",
            ImageModel::Dreamshaper8Lcm => "@cf/lykon/dreamshaper-8-lcm",
            ImageModel::SdXlBase1 => "@cf/stabilityai/stable-diffusion-xl-base-1.0",
        }
    }

    /// How the prompt must be encoded for this model.
    pub fn input(self) -> Input {
        match self {
            ImageModel::Flux2Dev | ImageModel::Flux2Klein4b | ImageModel::Flux2Klein9b => {
                Input::Multipart
            }
            ImageModel::Flux1Schnell
            | ImageModel::SdXlLightning
            | ImageModel::Dreamshaper8Lcm
            | ImageModel::SdXlBase1 => Input::Json,
        }
    }

    /// A short human description, for help text and menus.
    pub fn description(self) -> &'static str {
        match self {
            ImageModel::Flux2Dev => "Flux 2 Dev — best quality",
            ImageModel::Flux1Schnell => "Flux 1 Schnell — fast (default)",
            ImageModel::Flux2Klein4b => "Flux 2 Klein 4B — compact",
            ImageModel::Flux2Klein9b => "Flux 2 Klein 9B — compact",
            ImageModel::SdXlLightning => "SDXL Lightning — fast",
            ImageModel::Dreamshaper8Lcm => "Dreamshaper 8 LCM — fast",
            ImageModel::SdXlBase1 => "SDXL Base 1.0",
        }
    }

    /// All accepted tokens, canonical name first.
    pub fn aliases(self) -> &'static [&'static str] {
        match self {
            ImageModel::Flux2Dev => &["flux-2-dev", "flux2dev"],
            ImageModel::Flux1Schnell => &["flux-1-schnell", "flux-schnell", "schnell"],
            ImageModel::Flux2Klein4b => &["flux-2-klein-4b", "klein-4b"],
            ImageModel::Flux2Klein9b => &["flux-2-klein-9b", "klein-9b"],
            ImageModel::SdXlLightning => &["sd-xl-lightning", "sdxl-lightning", "sd-lightning"],
            ImageModel::Dreamshaper8Lcm => &["dreamshaper-8-lcm", "dreamshaper", "lcm"],
            ImageModel::SdXlBase1 => &["sd-xl-base", "sdxl-base", "sd-base"],
        }
    }

    /// Match user input against [`ImageModel::aliases`], case-insensitively.
    pub fn from_alias(s: &str) -> Option<Self> {
        let s = s.trim();
        Self::ALL
            .iter()
            .copied()
            .find(|m| m.aliases().iter().any(|a| s.eq_ignore_ascii_case(a)))
    }
}

impl fmt::Display for ImageModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ImageModel {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_alias(s).ok_or_else(|| Error::InvalidModel(s.trim().to_string()))
    }
}

/// The role a message plays in a chat conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Role {
    /// Instructions that set the assistant's behavior.
    System,
    /// A message from the user.
    User,
    /// A prior assistant reply, kept for context.
    Assistant,
}

impl Role {
    /// The role name as encoded in the Cloudflare request body.
    pub fn as_str(self) -> &'static str {
        match self {
            Role::System => "system",
            Role::User => "user",
            Role::Assistant => "assistant",
        }
    }
}

/// One message in a text-generation (chat) request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
}

impl ChatMessage {
    /// A message with the given role and content.
    pub fn new(role: Role, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
        }
    }

    /// A `system` message.
    pub fn system(content: impl Into<String>) -> Self {
        Self::new(Role::System, content)
    }

    /// A `user` message.
    pub fn user(content: impl Into<String>) -> Self {
        Self::new(Role::User, content)
    }

    /// An `assistant` message.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new(Role::Assistant, content)
    }
}

/// Token usage reported by a text-generation response.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Usage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
}

/// A completed text generation, normalized to plain text so consumers never
/// deal with provider transport details.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatCompletion {
    pub text: String,
    /// Token usage, when the provider reported it.
    pub usage: Option<Usage>,
}

/// A Cloudflare Workers AI text-generation model.
///
/// [`TextModel::path`] carries the `@cf/…` identifier used in the REST URL;
/// consumers deal in the short, human-friendly names of
/// [`TextModel::from_alias`] and never hard-code provider paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum TextModel {
    /// Llama 3.1 8B Instruct (fast) — a good balance, the default.
    #[default]
    Llama318b,
    /// Llama 3.2 3B Instruct — smallest and fastest.
    Llama323b,
    /// Llama 3.3 70B Instruct (fp8 fast) — highest quality.
    Llama3370b,
    /// DeepSeek R1 Distill Qwen 32B — reasoning.
    DeepseekR132b,
}

impl TextModel {
    /// Every supported model, in the order shown to users.
    pub const ALL: &[TextModel] = &[
        TextModel::Llama318b,
        TextModel::Llama323b,
        TextModel::Llama3370b,
        TextModel::DeepseekR132b,
    ];

    /// The canonical short name — the primary token users type.
    pub fn as_str(self) -> &'static str {
        self.aliases()[0]
    }

    /// The Cloudflare `@cf/…` path used in the REST URL.
    pub fn path(self) -> &'static str {
        match self {
            TextModel::Llama318b => "@cf/meta/llama-3.1-8b-instruct-fast",
            TextModel::Llama323b => "@cf/meta/llama-3.2-3b-instruct",
            TextModel::Llama3370b => "@cf/meta/llama-3.3-70b-instruct-fp8-fast",
            TextModel::DeepseekR132b => "@cf/deepseek-ai/deepseek-r1-distill-qwen-32b",
        }
    }

    /// A short human description, for help text and menus.
    pub fn description(self) -> &'static str {
        match self {
            TextModel::Llama318b => "Llama 3.1 8B — fast (default)",
            TextModel::Llama323b => "Llama 3.2 3B — fastest",
            TextModel::Llama3370b => "Llama 3.3 70B — best quality",
            TextModel::DeepseekR132b => "DeepSeek R1 32B — reasoning",
        }
    }

    /// All accepted tokens, canonical name first.
    pub fn aliases(self) -> &'static [&'static str] {
        match self {
            TextModel::Llama318b => &["llama-3.1-8b", "llama-8b", "8b"],
            TextModel::Llama323b => &["llama-3.2-3b", "llama-3b", "3b"],
            TextModel::Llama3370b => &["llama-3.3-70b", "llama-70b", "70b"],
            TextModel::DeepseekR132b => &["deepseek-r1", "deepseek-32b", "r1"],
        }
    }

    /// Match user input against [`TextModel::aliases`], case-insensitively.
    pub fn from_alias(s: &str) -> Option<Self> {
        let s = s.trim();
        Self::ALL
            .iter()
            .copied()
            .find(|m| m.aliases().iter().any(|a| s.eq_ignore_ascii_case(a)))
    }
}

impl fmt::Display for TextModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for TextModel {
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
        assert_eq!(ImageModel::default(), ImageModel::Flux1Schnell);
        assert_eq!(
            ImageModel::default().path(),
            "@cf/black-forest-labs/flux-1-schnell"
        );
    }

    #[test]
    fn from_alias_matches_canonical_and_aliases_case_insensitively() {
        assert_eq!(
            ImageModel::from_alias("flux-2-dev"),
            Some(ImageModel::Flux2Dev)
        );
        assert_eq!(
            ImageModel::from_alias("FLUX2DEV"),
            Some(ImageModel::Flux2Dev)
        );
        assert_eq!(
            ImageModel::from_alias("schnell"),
            Some(ImageModel::Flux1Schnell)
        );
        assert_eq!(
            ImageModel::from_alias("sd-xl-base"),
            Some(ImageModel::SdXlBase1)
        );
    }

    #[test]
    fn from_alias_rejects_unknown_tokens() {
        assert_eq!(ImageModel::from_alias("flux"), None);
        assert_eq!(ImageModel::from_alias(""), None);
    }

    #[test]
    fn from_str_reports_the_bad_token() {
        let err = "flux".parse::<ImageModel>().unwrap_err();
        assert!(err.to_string().contains("flux"));
    }

    #[test]
    fn display_uses_canonical_name() {
        assert_eq!(ImageModel::Flux2Dev.to_string(), "flux-2-dev");
        assert_eq!(ImageModel::SdXlBase1.to_string(), "sd-xl-base");
    }

    #[test]
    fn input_encoding_by_model() {
        assert_eq!(ImageModel::Flux2Dev.input(), Input::Multipart);
        assert_eq!(ImageModel::Flux2Klein4b.input(), Input::Multipart);
        assert_eq!(ImageModel::Flux2Klein9b.input(), Input::Multipart);
        assert_eq!(ImageModel::Flux1Schnell.input(), Input::Json);
        assert_eq!(ImageModel::SdXlLightning.input(), Input::Json);
        assert_eq!(ImageModel::Dreamshaper8Lcm.input(), Input::Json);
        assert_eq!(ImageModel::SdXlBase1.input(), Input::Json);
    }
}

#[cfg(test)]
mod text_model_tests {
    use super::*;

    #[test]
    fn default_is_llama_3_1_8b() {
        assert_eq!(TextModel::default(), TextModel::Llama318b);
        assert_eq!(
            TextModel::default().path(),
            "@cf/meta/llama-3.1-8b-instruct-fast"
        );
    }

    #[test]
    fn paths_are_cloudflare_identifiers() {
        assert_eq!(
            TextModel::Llama323b.path(),
            "@cf/meta/llama-3.2-3b-instruct"
        );
        assert_eq!(
            TextModel::Llama3370b.path(),
            "@cf/meta/llama-3.3-70b-instruct-fp8-fast"
        );
        assert_eq!(
            TextModel::DeepseekR132b.path(),
            "@cf/deepseek-ai/deepseek-r1-distill-qwen-32b"
        );
    }

    #[test]
    fn from_alias_matches_case_insensitively() {
        assert_eq!(
            TextModel::from_alias("llama-3.1-8b"),
            Some(TextModel::Llama318b)
        );
        assert_eq!(TextModel::from_alias("8B"), Some(TextModel::Llama318b));
        assert_eq!(TextModel::from_alias("r1"), Some(TextModel::DeepseekR132b));
        assert_eq!(
            TextModel::from_alias("llama-70b"),
            Some(TextModel::Llama3370b)
        );
    }

    #[test]
    fn from_alias_rejects_unknown_tokens() {
        assert_eq!(TextModel::from_alias("llama"), None);
        assert_eq!(TextModel::from_alias(""), None);
    }

    #[test]
    fn from_str_reports_the_bad_token() {
        let err = "gpt-4".parse::<TextModel>().unwrap_err();
        assert!(err.to_string().contains("gpt-4"));
    }

    #[test]
    fn display_uses_canonical_name() {
        assert_eq!(TextModel::Llama318b.to_string(), "llama-3.1-8b");
        assert_eq!(TextModel::DeepseekR132b.to_string(), "deepseek-r1");
    }

    #[test]
    fn roles_encode_to_wire_names() {
        assert_eq!(Role::System.as_str(), "system");
        assert_eq!(Role::User.as_str(), "user");
        assert_eq!(Role::Assistant.as_str(), "assistant");
    }

    #[test]
    fn chat_message_constructors() {
        assert_eq!(
            ChatMessage::user("hi"),
            ChatMessage {
                role: Role::User,
                content: "hi".into()
            }
        );
        assert_eq!(
            ChatMessage::system("be brief"),
            ChatMessage {
                role: Role::System,
                content: "be brief".into()
            }
        );
        assert_eq!(
            ChatMessage::assistant("sure"),
            ChatMessage {
                role: Role::Assistant,
                content: "sure".into()
            }
        );
    }
}
