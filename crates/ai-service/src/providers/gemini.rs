//! Google Gemini provider — Gemini 1.5 Flash for text, Imagen 3 for images.

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::{AiError, AiProvider, Capability, GenOpts, GenResult, ImgOpts, ImgResult, ProviderKind};

#[derive(Clone)]
pub struct GeminiProvider {
    client: Client,
    api_key: String,
    model_text: String,
    model_image: String,
}

impl GeminiProvider {
    pub fn new(client: Client, api_key: String, model_text: String, model_image: String) -> Self {
        Self {
            client,
            api_key,
            model_text: if model_text.is_empty() {
                "gemini-1.5-flash".into()
            } else {
                model_text
            },
            model_image: if model_image.is_empty() {
                "imagen-3.0-generate-001".into()
            } else {
                model_image
            },
        }
    }
}

#[derive(Serialize)]
struct GenerateContentRequest<'a> {
    contents: Vec<GeminiContent<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<GeminiContent<'a>>,
    generation_config: GenerationConfig,
}

#[derive(Serialize)]
struct GeminiContent<'a> {
    parts: Vec<GeminiPart<'a>>,
}

#[derive(Serialize)]
struct GeminiPart<'a> {
    text: &'a str,
}

#[derive(Serialize)]
struct GenerationConfig {
    #[serde(rename = "maxOutputTokens")]
    max_output_tokens: u32,
    temperature: f32,
}

#[derive(Deserialize)]
struct GenerateContentResponse {
    candidates: Vec<Candidate>,
    #[serde(rename = "usageMetadata")]
    usage_metadata: Option<UsageMetadata>,
}

#[derive(Deserialize)]
struct Candidate {
    content: Option<ContentOut>,
}

#[derive(Deserialize)]
struct ContentOut {
    parts: Vec<PartOut>,
}

#[derive(Deserialize)]
struct PartOut {
    text: Option<String>,
}

#[derive(Deserialize)]
struct UsageMetadata {
    #[serde(rename = "totalTokenCount")]
    total_token_count: u32,
}

#[derive(Serialize)]
struct ImagenRequest {
    instances: Vec<ImagenInstance>,
    parameters: ImagenParameters,
}

#[derive(Serialize)]
struct ImagenInstance {
    prompt: String,
}

#[derive(Serialize)]
struct ImagenParameters {
    #[serde(rename = "sampleCount")]
    sample_count: u32,
}

#[derive(Deserialize)]
struct ImagenResponse {
    predictions: Vec<ImagenPrediction>,
}

#[derive(Deserialize)]
struct ImagenPrediction {
    #[serde(rename = "bytesBase64Encoded")]
    bytes_base64_encoded: Option<String>,
}

#[async_trait]
impl AiProvider for GeminiProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Gemini
    }

    fn capability(&self) -> Capability {
        Capability::Both
    }

    fn model_text(&self) -> &str {
        &self.model_text
    }

    fn model_image(&self) -> &str {
        &self.model_image
    }

    async fn generate_text(&self, prompt: &str, opts: &GenOpts) -> Result<GenResult, AiError> {
        if self.api_key.is_empty() {
            return Err(AiError::NotConfigured("gemini".into()));
        }

        let body = GenerateContentRequest {
            contents: vec![GeminiContent {
                parts: vec![GeminiPart { text: prompt }],
            }],
            system_instruction: opts.system_prompt.as_deref().map(|s| GeminiContent {
                parts: vec![GeminiPart { text: s }],
            }),
            generation_config: GenerationConfig {
                max_output_tokens: opts.max_tokens.unwrap_or(1024),
                temperature: opts.temperature.unwrap_or(0.7),
            },
        };

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model_text, self.api_key
        );

        let resp = self.client.post(&url).json(&body).send().await.map_err(|e| {
            AiError::NetworkError {
                provider: "gemini".into(),
                message: e.to_string(),
            }
        })?;

        let status = resp.status();
        if !status.is_success() {
            let message = resp.text().await.unwrap_or_default();
            return Err(AiError::HttpError {
                provider: "gemini".into(),
                status: status.as_u16(),
                message,
            });
        }

        let parsed: GenerateContentResponse = resp.json().await.map_err(|e| AiError::ParseError {
            provider: "gemini".into(),
            message: e.to_string(),
        })?;

        let content = parsed
            .candidates
            .into_iter()
            .filter_map(|c| c.content)
            .flat_map(|c| c.parts)
            .filter_map(|p| p.text)
            .collect::<Vec<_>>()
            .join("");

        Ok(GenResult {
            content,
            tokens_used: parsed.usage_metadata.map(|u| u.total_token_count).unwrap_or(0),
            provider: "gemini".into(),
            model: self.model_text.clone(),
        })
    }

    async fn generate_image(&self, prompt: &str, opts: &ImgOpts) -> Result<ImgResult, AiError> {
        if self.api_key.is_empty() {
            return Err(AiError::NotConfigured("gemini".into()));
        }

        let body = ImagenRequest {
            instances: vec![ImagenInstance {
                prompt: prompt.to_string(),
            }],
            parameters: ImagenParameters {
                sample_count: opts.n.clamp(1, 4),
            },
        };

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:predict?key={}",
            self.model_image, self.api_key
        );

        let resp = self.client.post(&url).json(&body).send().await.map_err(|e| {
            AiError::NetworkError {
                provider: "gemini".into(),
                message: e.to_string(),
            }
        })?;

        let status = resp.status();
        if !status.is_success() {
            let message = resp.text().await.unwrap_or_default();
            return Err(AiError::HttpError {
                provider: "gemini".into(),
                status: status.as_u16(),
                message,
            });
        }

        let parsed: ImagenResponse = resp.json().await.map_err(|e| AiError::ParseError {
            provider: "gemini".into(),
            message: e.to_string(),
        })?;

        // Imagen returns base64 — caller should upload to storage. Here we surface data URIs.
        let urls: Vec<String> = parsed
            .predictions
            .into_iter()
            .filter_map(|p| p.bytes_base64_encoded)
            .map(|b64| format!("data:image/png;base64,{}", b64))
            .collect();

        Ok(ImgResult {
            urls,
            provider: "gemini".into(),
            model: self.model_image.clone(),
        })
    }
}
