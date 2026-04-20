//! Pluggable AI provider abstraction.
//!
//! Supports OpenAI, Anthropic, and Google Gemini. A factory selects the
//! primary provider and falls back to the next configured provider on failure.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub mod anthropic;
pub mod factory;
pub mod gemini;
pub mod openai;

pub use factory::ProviderRegistry;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKind {
    Openai,
    Anthropic,
    Gemini,
}

impl ProviderKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProviderKind::Openai => "openai",
            ProviderKind::Anthropic => "anthropic",
            ProviderKind::Gemini => "gemini",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "openai" => Some(ProviderKind::Openai),
            "anthropic" | "claude" => Some(ProviderKind::Anthropic),
            "gemini" | "google" => Some(ProviderKind::Gemini),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Capability {
    Text,
    Image,
    Both,
}

#[derive(Debug, Clone, Default)]
pub struct GenOpts {
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub system_prompt: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ImgOpts {
    pub n: u32,
    pub size: Option<String>,       // e.g. "1024x1024"
    pub quality: Option<String>,    // standard | hd
    pub style: Option<String>,      // vivid | natural
}

#[derive(Debug, Clone, Serialize)]
pub struct GenResult {
    pub content: String,
    pub tokens_used: u32,
    pub provider: String,
    pub model: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImgResult {
    pub urls: Vec<String>,
    pub provider: String,
    pub model: String,
}

#[derive(Debug, thiserror::Error)]
pub enum AiError {
    #[error("Provider {0} not configured")]
    NotConfigured(String),

    #[error("Provider HTTP error ({provider}): {status}: {message}")]
    HttpError {
        provider: String,
        status: u16,
        message: String,
    },

    #[error("Network error ({provider}): {message}")]
    NetworkError { provider: String, message: String },

    #[error("Parsing error ({provider}): {message}")]
    ParseError { provider: String, message: String },

    #[error("Capability {0} not supported by this provider")]
    CapabilityNotSupported(String),
}

#[async_trait]
pub trait AiProvider: Send + Sync {
    fn kind(&self) -> ProviderKind;

    fn capability(&self) -> Capability;

    fn model_text(&self) -> &str;

    fn model_image(&self) -> &str {
        ""
    }

    async fn generate_text(&self, prompt: &str, opts: &GenOpts) -> Result<GenResult, AiError>;

    async fn generate_image(&self, prompt: &str, opts: &ImgOpts) -> Result<ImgResult, AiError> {
        let _ = (prompt, opts);
        Err(AiError::CapabilityNotSupported("image".into()))
    }

    /// Approximate token count for budgeting.
    fn estimate_tokens(&self, text: &str) -> u32 {
        // Rough heuristic: ~4 chars per token for English, ~3 for other languages.
        (text.chars().count() as f32 / 3.5).ceil() as u32
    }
}
