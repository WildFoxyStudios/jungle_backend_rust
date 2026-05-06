//! OpenAI provider — GPT-4o-mini for text, DALL-E 3 for images.

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::{
    AiError, AiProvider, Capability, GenOpts, GenResult, ImgOpts, ImgResult, ProviderKind,
};

#[derive(Clone)]
pub struct OpenAiProvider {
    client: Client,
    api_key: String,
    model_text: String,
    model_image: String,
}

impl OpenAiProvider {
    pub fn new(client: Client, api_key: String, model_text: String, model_image: String) -> Self {
        Self {
            client,
            api_key,
            model_text: if model_text.is_empty() {
                "gpt-4o-mini".into()
            } else {
                model_text
            },
            model_image: if model_image.is_empty() {
                "dall-e-3".into()
            } else {
                model_image
            },
        }
    }
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
    max_tokens: u32,
    temperature: f32,
}

#[derive(Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
    usage: Option<ChatUsage>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatResponseMessage,
}

#[derive(Deserialize)]
struct ChatResponseMessage {
    content: String,
}

#[derive(Deserialize)]
struct ChatUsage {
    total_tokens: u32,
}

#[derive(Serialize)]
struct ImageRequest<'a> {
    model: &'a str,
    prompt: &'a str,
    n: u32,
    size: &'a str,
    quality: &'a str,
    style: &'a str,
    response_format: &'a str,
}

#[derive(Deserialize)]
struct ImageResponse {
    data: Vec<ImageData>,
}

#[derive(Deserialize)]
struct ImageData {
    url: Option<String>,
    b64_json: Option<String>,
}

#[async_trait]
impl AiProvider for OpenAiProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Openai
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
            return Err(AiError::NotConfigured("openai".into()));
        }

        let mut messages: Vec<ChatMessage> = Vec::new();
        if let Some(sys) = opts.system_prompt.as_deref() {
            messages.push(ChatMessage {
                role: "system",
                content: sys,
            });
        }
        messages.push(ChatMessage {
            role: "user",
            content: prompt,
        });

        let body = ChatRequest {
            model: &self.model_text,
            messages,
            max_tokens: opts.max_tokens.unwrap_or(1024),
            temperature: opts.temperature.unwrap_or(0.7),
        };

        let resp = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| AiError::NetworkError {
                provider: "openai".into(),
                message: e.to_string(),
            })?;

        let status = resp.status();
        if !status.is_success() {
            let message = resp.text().await.unwrap_or_default();
            return Err(AiError::HttpError {
                provider: "openai".into(),
                status: status.as_u16(),
                message,
            });
        }

        let parsed: ChatResponse = resp.json().await.map_err(|e| AiError::ParseError {
            provider: "openai".into(),
            message: e.to_string(),
        })?;

        let content = parsed
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .unwrap_or_default();

        Ok(GenResult {
            content,
            tokens_used: parsed.usage.map(|u| u.total_tokens).unwrap_or(0),
            provider: "openai".into(),
            model: self.model_text.clone(),
        })
    }

    async fn generate_image(&self, prompt: &str, opts: &ImgOpts) -> Result<ImgResult, AiError> {
        if self.api_key.is_empty() {
            return Err(AiError::NotConfigured("openai".into()));
        }

        let body = ImageRequest {
            model: &self.model_image,
            prompt,
            n: opts.n.clamp(1, 10),
            size: opts.size.as_deref().unwrap_or("1024x1024"),
            quality: opts.quality.as_deref().unwrap_or("standard"),
            style: opts.style.as_deref().unwrap_or("vivid"),
            response_format: "url",
        };

        let resp = self
            .client
            .post("https://api.openai.com/v1/images/generations")
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| AiError::NetworkError {
                provider: "openai".into(),
                message: e.to_string(),
            })?;

        let status = resp.status();
        if !status.is_success() {
            let message = resp.text().await.unwrap_or_default();
            return Err(AiError::HttpError {
                provider: "openai".into(),
                status: status.as_u16(),
                message,
            });
        }

        let parsed: ImageResponse = resp.json().await.map_err(|e| AiError::ParseError {
            provider: "openai".into(),
            message: e.to_string(),
        })?;

        let urls: Vec<String> = parsed
            .data
            .into_iter()
            .filter_map(|d| d.url.or(d.b64_json))
            .collect();

        Ok(ImgResult {
            urls,
            provider: "openai".into(),
            model: self.model_image.clone(),
        })
    }
}

// ═══════════════════════════════════════════════════════════════════
// Moderation
// ═══════════════════════════════════════════════════════════════════

#[derive(Debug, serde::Deserialize)]
pub struct ModerationResult {
    pub flagged: bool,
    pub categories: std::collections::HashMap<String, bool>,
    pub category_scores: std::collections::HashMap<String, f64>,
}

pub async fn moderate_text(
    client: &reqwest::Client,
    api_key: &str,
    text: &str,
) -> Result<ModerationResult, String> {
    let body = serde_json::json!({
        "model": "omni-moderation-latest",
        "input": text,
    });

    let resp = client
        .post("https://api.openai.com/v1/moderations")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("OpenAI moderation request failed: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("OpenAI moderation error {}: {}", status, body));
    }

    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let result = &json["results"][0];

    let categories: std::collections::HashMap<String, bool> = result["categories"]
        .as_object()
        .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.as_bool().unwrap_or(false))).collect())
        .unwrap_or_default();

    let category_scores: std::collections::HashMap<String, f64> = result["category_scores"]
        .as_object()
        .map(|obj| obj.iter().filter_map(|(k, v)| v.as_f64().map(|s| (k.clone(), s))).collect())
        .unwrap_or_default();

    Ok(ModerationResult {
        flagged: result["flagged"].as_bool().unwrap_or(false),
        categories,
        category_scores,
    })
}
