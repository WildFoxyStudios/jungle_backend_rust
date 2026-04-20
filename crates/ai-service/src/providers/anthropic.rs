//! Anthropic Claude provider — text only.
//!
//! Uses the Messages API (https://docs.anthropic.com/en/api/messages).

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::{AiError, AiProvider, Capability, GenOpts, GenResult, ProviderKind};

#[derive(Clone)]
pub struct AnthropicProvider {
    client: Client,
    api_key: String,
    model_text: String,
}

impl AnthropicProvider {
    pub fn new(client: Client, api_key: String, model_text: String) -> Self {
        Self {
            client,
            api_key,
            model_text: if model_text.is_empty() {
                "claude-3-5-sonnet-20241022".into()
            } else {
                model_text
            },
        }
    }
}

#[derive(Serialize)]
struct MessagesRequest<'a> {
    model: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<&'a str>,
    messages: Vec<MsgIn<'a>>,
    max_tokens: u32,
    temperature: f32,
}

#[derive(Serialize)]
struct MsgIn<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct MessagesResponse {
    content: Vec<ContentBlock>,
    usage: Option<AnthropicUsage>,
}

#[derive(Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    kind: String,
    text: Option<String>,
}

#[derive(Deserialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

#[async_trait]
impl AiProvider for AnthropicProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Anthropic
    }

    fn capability(&self) -> Capability {
        Capability::Text
    }

    fn model_text(&self) -> &str {
        &self.model_text
    }

    async fn generate_text(&self, prompt: &str, opts: &GenOpts) -> Result<GenResult, AiError> {
        if self.api_key.is_empty() {
            return Err(AiError::NotConfigured("anthropic".into()));
        }

        let body = MessagesRequest {
            model: &self.model_text,
            system: opts.system_prompt.as_deref(),
            messages: vec![MsgIn {
                role: "user",
                content: prompt,
            }],
            max_tokens: opts.max_tokens.unwrap_or(1024),
            temperature: opts.temperature.unwrap_or(0.7),
        };

        let resp = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .await
            .map_err(|e| AiError::NetworkError {
                provider: "anthropic".into(),
                message: e.to_string(),
            })?;

        let status = resp.status();
        if !status.is_success() {
            let message = resp.text().await.unwrap_or_default();
            return Err(AiError::HttpError {
                provider: "anthropic".into(),
                status: status.as_u16(),
                message,
            });
        }

        let parsed: MessagesResponse = resp.json().await.map_err(|e| AiError::ParseError {
            provider: "anthropic".into(),
            message: e.to_string(),
        })?;

        let content = parsed
            .content
            .into_iter()
            .filter(|c| c.kind == "text")
            .filter_map(|c| c.text)
            .collect::<Vec<_>>()
            .join("");

        let tokens_used = parsed
            .usage
            .map(|u| u.input_tokens + u.output_tokens)
            .unwrap_or(0);

        Ok(GenResult {
            content,
            tokens_used,
            provider: "anthropic".into(),
            model: self.model_text.clone(),
        })
    }
}
