//! Factory that loads provider configurations from DB, decrypts API keys
//! and builds a priority-ordered fallback chain.

use reqwest::Client;
use sqlx::PgPool;
use std::sync::Arc;

use super::{
    anthropic::AnthropicProvider, gemini::GeminiProvider, openai::OpenAiProvider, AiError,
    AiProvider, Capability, ProviderKind,
};
use crate::crypto;

#[derive(sqlx::FromRow, Debug)]
struct ProviderRow {
    provider_type: String,
    capability: String,
    api_key_encrypted: String,
    model_text: Option<String>,
    model_image: Option<String>,
    priority: i32,
}

pub struct ProviderRegistry {
    db: PgPool,
    client: Client,
    enc_key: Vec<u8>,
    /// Fallback if DB has no configured providers. Reads env vars.
    env_fallback: Vec<Arc<dyn AiProvider>>,
}

impl ProviderRegistry {
    pub fn new(db: PgPool, client: Client, enc_key: Vec<u8>) -> Self {
        let env_fallback = Self::build_env_providers(&client);
        Self {
            db,
            client,
            enc_key,
            env_fallback,
        }
    }

    fn build_env_providers(client: &Client) -> Vec<Arc<dyn AiProvider>> {
        let mut providers: Vec<Arc<dyn AiProvider>> = Vec::new();

        if let Ok(key) = std::env::var("OPENAI_API_KEY")
            && !key.is_empty()
        {
            providers.push(Arc::new(OpenAiProvider::new(
                client.clone(),
                key,
                std::env::var("OPENAI_MODEL").unwrap_or_default(),
                std::env::var("OPENAI_IMAGE_MODEL").unwrap_or_default(),
            )));
        }

        if let Ok(key) = std::env::var("ANTHROPIC_API_KEY")
            && !key.is_empty()
        {
            providers.push(Arc::new(AnthropicProvider::new(
                client.clone(),
                key,
                std::env::var("ANTHROPIC_MODEL").unwrap_or_default(),
            )));
        }

        if let Ok(key) = std::env::var("GEMINI_API_KEY")
            && !key.is_empty()
        {
            providers.push(Arc::new(GeminiProvider::new(
                client.clone(),
                key,
                std::env::var("GEMINI_MODEL").unwrap_or_default(),
                std::env::var("GEMINI_IMAGE_MODEL").unwrap_or_default(),
            )));
        }

        providers
    }

    /// Load providers from DB ordered by priority (lower first) for the requested capability.
    /// Falls back to env-based providers if DB is empty.
    pub async fn chain_for(&self, capability: Capability) -> Vec<Arc<dyn AiProvider>> {
        let cap_str = match capability {
            Capability::Text => "text",
            Capability::Image => "image",
            Capability::Both => "both",
        };

        let rows = sqlx::query_as::<_, ProviderRow>(
            r#"
            SELECT provider_type, capability, api_key_encrypted,
                   model_text, model_image, priority
              FROM ai_provider_config
             WHERE enabled = TRUE
               AND (capability = $1 OR capability = 'both')
          ORDER BY priority ASC
            "#,
        )
        .bind(cap_str)
        .fetch_all(&self.db)
        .await;

        let rows = match rows {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!(error = %e, "failed to load ai_provider_config, using env fallback");
                return self.env_fallback.clone();
            }
        };

        if rows.is_empty() {
            return self.env_fallback.clone();
        }

        let mut chain: Vec<Arc<dyn AiProvider>> = Vec::new();
        for row in rows {
            // Defensive filter — the SQL already filters, but validate the capability
            // column against the requested capability in case of DB drift.
            if !capability_matches(&row.capability, capability) {
                tracing::debug!(
                    provider = %row.provider_type,
                    db_capability = %row.capability,
                    requested = ?capability,
                    "skipping provider: capability mismatch"
                );
                continue;
            }

            let api_key = match crypto::decrypt(&self.enc_key, &row.api_key_encrypted) {
                Ok(k) => k,
                Err(e) => {
                    tracing::error!(error = %e, provider = %row.provider_type, "failed to decrypt api key");
                    continue;
                }
            };

            let kind = match ProviderKind::from_str(&row.provider_type) {
                Some(k) => k,
                None => {
                    tracing::warn!(
                        provider_type = %row.provider_type,
                        priority = row.priority,
                        "unknown provider_type, skipping"
                    );
                    continue;
                }
            };

            tracing::debug!(
                provider = %row.provider_type,
                capability = %row.capability,
                priority = row.priority,
                "adding provider to chain"
            );

            let provider: Arc<dyn AiProvider> = match kind {
                ProviderKind::Openai => Arc::new(OpenAiProvider::new(
                    self.client.clone(),
                    api_key,
                    row.model_text.unwrap_or_default(),
                    row.model_image.unwrap_or_default(),
                )),
                ProviderKind::Anthropic => Arc::new(AnthropicProvider::new(
                    self.client.clone(),
                    api_key,
                    row.model_text.unwrap_or_default(),
                )),
                ProviderKind::Gemini => Arc::new(GeminiProvider::new(
                    self.client.clone(),
                    api_key,
                    row.model_text.unwrap_or_default(),
                    row.model_image.unwrap_or_default(),
                )),
            };

            chain.push(provider);
        }

        if chain.is_empty() {
            self.env_fallback.clone()
        } else {
            chain
        }
    }

    /// Health snapshot for admin dashboards — returns `(provider_type, capability, priority, enabled)`
    /// tuples sorted by priority, without decrypting secrets.
    pub async fn health_snapshot(&self) -> Vec<(String, String, i32)> {
        let rows = sqlx::query_as::<_, ProviderRow>(
            r#"
            SELECT provider_type, capability, api_key_encrypted,
                   model_text, model_image, priority
              FROM ai_provider_config
             WHERE enabled = TRUE
          ORDER BY priority ASC
            "#,
        )
        .fetch_all(&self.db)
        .await
        .unwrap_or_default();

        rows.into_iter()
            .map(|r| (r.provider_type, r.capability, r.priority))
            .collect()
    }
}

/// Check whether a DB `capability` value (`text` | `image` | `both`) satisfies
/// a requested `capability`.
fn capability_matches(db_cap: &str, requested: Capability) -> bool {
    match (db_cap, requested) {
        ("both", _) => true,
        ("text", Capability::Text) => true,
        ("image", Capability::Image) => true,
        (_, Capability::Both) => true, // loading the full catalog
        _ => false,
    }
}

/// Iterate the chain calling generate_text, returning on first success.
pub async fn try_text(
    chain: &[Arc<dyn AiProvider>],
    prompt: &str,
    opts: &super::GenOpts,
) -> Result<super::GenResult, AiError> {
    let mut last_err: Option<AiError> = None;
    for p in chain {
        if matches!(p.capability(), Capability::Text | Capability::Both) {
            match p.generate_text(prompt, opts).await {
                Ok(res) => return Ok(res),
                Err(e) => {
                    tracing::warn!(provider = %p.kind().as_str(), error = %e, "text generation failed, trying next");
                    last_err = Some(e);
                }
            }
        }
    }
    Err(last_err.unwrap_or_else(|| AiError::NotConfigured("any text provider".into())))
}

pub async fn try_image(
    chain: &[Arc<dyn AiProvider>],
    prompt: &str,
    opts: &super::ImgOpts,
) -> Result<super::ImgResult, AiError> {
    let mut last_err: Option<AiError> = None;
    for p in chain {
        if matches!(p.capability(), Capability::Image | Capability::Both) {
            match p.generate_image(prompt, opts).await {
                Ok(res) => return Ok(res),
                Err(e) => {
                    tracing::warn!(provider = %p.kind().as_str(), error = %e, "image generation failed, trying next");
                    last_err = Some(e);
                }
            }
        }
    }
    Err(last_err.unwrap_or_else(|| AiError::NotConfigured("any image provider".into())))
}
