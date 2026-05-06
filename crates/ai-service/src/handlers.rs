//! HTTP handlers for ai-service.
//!
//! Endpoints:
//! - `/v1/ai/chat`, `/v1/ai/suggest-post`, `/v1/ai/describe-image` (legacy, still supported)
//! - `/v1/ai/generate-post`, `/v1/ai/generate-blog`, `/v1/ai/generate-images`
//! - `/v1/ai/balance/words`, `/v1/ai/balance/images`
//! - `/v1/admin/ai/providers` (list/CRUD)
//! - `/v1/admin/ai/providers/{id}/test`

use axum::{
    Json,
    extract::{FromRef, State},
};
use serde::Deserialize;
use serde_json::{Value, json};
use shared::auth::{AppState, AuthUser};
use shared::errors::ApiError;
use shared::permissions::Permission;
use sqlx::PgPool;
use std::sync::Arc;

use crate::providers::cloudflare_ai;
use crate::providers::ocrspace;
use crate::providers::openai;

use crate::credits::{self, CreditKind};
use crate::providers::{GenOpts, ImgOpts, ProviderRegistry, factory};
use shared::moderation;

#[derive(Clone)]
pub struct AiState {
    pub app: AppState,
    pub http: reqwest::Client,
    pub registry: Arc<ProviderRegistry>,
    pub enc_key: Vec<u8>,
}

impl AiState {
    pub fn db(&self) -> &PgPool {
        &self.app.db
    }
}

impl FromRef<AiState> for AppState {
    fn from_ref(state: &AiState) -> Self {
        state.app.clone()
    }
}

// ═══════════════════════════════════════════════════════════════════
// Generate Post
// ═══════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct GeneratePostRequest {
    pub prompt: Option<String>,
    pub topic: Option<String>,
    pub tone: Option<String>,
    pub max_tokens: Option<u32>,
}

pub async fn generate_post(
    State(state): State<AiState>,
    auth: AuthUser,
    Json(req): Json<GeneratePostRequest>,
) -> Result<Json<Value>, ApiError> {
    let words_to_estimate = req.max_tokens.unwrap_or(300) as i32;
    credits::deduct(
        state.db(),
        auth.user_id,
        CreditKind::Words(words_to_estimate),
    )
    .await?;

    let system = format!(
        "You are a social media post writer. Write a concise, engaging post in {} tone. \
         Use relevant emojis sparingly. Avoid hashtag stuffing. Max 280 characters.",
        req.tone.as_deref().unwrap_or("casual"),
    );
    let prompt = req.prompt.or(req.topic).unwrap_or_else(|| {
        "Write an interesting, engaging social media post about something trending.".into()
    });

    let chain = state
        .registry
        .chain_for(crate::providers::Capability::Text)
        .await;
    let opts = GenOpts {
        max_tokens: Some(req.max_tokens.unwrap_or(300)),
        temperature: Some(0.9),
        system_prompt: Some(system),
    };

    match factory::try_text(&chain, &prompt, &opts).await {
        Ok(res) => {
            credits::log_usage(
                state.db(),
                credits::UsageLog {
                    user_id: auth.user_id,
                    provider: &res.provider,
                    kind: "post",
                    tokens_used: res.tokens_used as i32,
                    images_generated: 0,
                    cost_cents: 0,
                    success: true,
                    error_message: None,
                },
            )
            .await;
            Ok(Json(json!({
                "data": {
                    "content": res.content,
                    "provider": res.provider,
                    "model": res.model,
                    "tokens_used": res.tokens_used,
                }
            })))
        }
        Err(e) => {
            credits::log_usage(
                state.db(),
                credits::UsageLog {
                    user_id: auth.user_id,
                    provider: "unknown",
                    kind: "post",
                    tokens_used: 0,
                    images_generated: 0,
                    cost_cents: 0,
                    success: false,
                    error_message: Some(&e.to_string()),
                },
            )
            .await;
            Err(ApiError::Internal(format!("AI generation failed: {}", e)))
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// Generate Blog
// ═══════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct GenerateBlogRequest {
    pub topic: String,
    pub keywords: Option<Vec<String>>,
    pub tone: Option<String>,
    pub length: Option<String>, // short | medium | long
}

pub async fn generate_blog(
    State(state): State<AiState>,
    auth: AuthUser,
    Json(req): Json<GenerateBlogRequest>,
) -> Result<Json<Value>, ApiError> {
    let (max_tokens, est_words) = match req.length.as_deref().unwrap_or("medium") {
        "short" => (800, 600),
        "long" => (3000, 2000),
        _ => (1800, 1200),
    };

    credits::deduct(state.db(), auth.user_id, CreditKind::Words(est_words)).await?;

    let keywords_hint = req
        .keywords
        .as_ref()
        .map(|k| format!(" Include these keywords naturally: {}.", k.join(", ")))
        .unwrap_or_default();

    let system = format!(
        "You are a professional blog writer. Write a well-structured article in {} tone with:\n\
         - A compelling H1 title (markdown # heading)\n\
         - An engaging introduction\n\
         - 3-5 H2 sections (markdown ## headings)\n\
         - A conclusion\n\
         - Markdown formatting.{}",
        req.tone.as_deref().unwrap_or("informative"),
        keywords_hint,
    );

    let chain = state
        .registry
        .chain_for(crate::providers::Capability::Text)
        .await;
    let opts = GenOpts {
        max_tokens: Some(max_tokens),
        temperature: Some(0.7),
        system_prompt: Some(system),
    };

    match factory::try_text(&chain, &req.topic, &opts).await {
        Ok(res) => {
            credits::log_usage(
                state.db(),
                credits::UsageLog {
                    user_id: auth.user_id,
                    provider: &res.provider,
                    kind: "blog",
                    tokens_used: res.tokens_used as i32,
                    images_generated: 0,
                    cost_cents: 0,
                    success: true,
                    error_message: None,
                },
            )
            .await;
            // Extract title if present
            let title = res
                .content
                .lines()
                .find(|l| l.starts_with("# "))
                .map(|l| l.trim_start_matches("# ").to_string())
                .unwrap_or_else(|| req.topic.clone());

            Ok(Json(json!({
                "data": {
                    "title": title,
                    "content": res.content,
                    "provider": res.provider,
                    "model": res.model,
                    "tokens_used": res.tokens_used,
                }
            })))
        }
        Err(e) => {
            credits::log_usage(
                state.db(),
                credits::UsageLog {
                    user_id: auth.user_id,
                    provider: "unknown",
                    kind: "blog",
                    tokens_used: 0,
                    images_generated: 0,
                    cost_cents: 0,
                    success: false,
                    error_message: Some(&e.to_string()),
                },
            )
            .await;
            Err(ApiError::Internal(format!(
                "AI blog generation failed: {}",
                e
            )))
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// Generate Images
// ═══════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct GenerateImagesRequest {
    pub prompt: String,
    pub n: Option<u32>,
    pub size: Option<String>,
    pub quality: Option<String>,
    pub style: Option<String>,
}

pub async fn generate_images(
    State(state): State<AiState>,
    auth: AuthUser,
    Json(req): Json<GenerateImagesRequest>,
) -> Result<Json<Value>, ApiError> {
    let n = req.n.unwrap_or(1).clamp(1, 4) as i32;
    credits::deduct(state.db(), auth.user_id, CreditKind::Images(n)).await?;

    let chain = state
        .registry
        .chain_for(crate::providers::Capability::Image)
        .await;
    let opts = ImgOpts {
        n: n as u32,
        size: req.size.clone(),
        quality: req.quality.clone(),
        style: req.style.clone(),
    };

    match factory::try_image(&chain, &req.prompt, &opts).await {
        Ok(res) => {
            credits::log_usage(
                state.db(),
                credits::UsageLog {
                    user_id: auth.user_id,
                    provider: &res.provider,
                    kind: "images",
                    tokens_used: 0,
                    images_generated: res.urls.len() as i32,
                    cost_cents: 0,
                    success: true,
                    error_message: None,
                },
            )
            .await;
            Ok(Json(json!({
                "data": {
                    "urls": res.urls,
                    "provider": res.provider,
                    "model": res.model,
                }
            })))
        }
        Err(e) => {
            credits::log_usage(
                state.db(),
                credits::UsageLog {
                    user_id: auth.user_id,
                    provider: "unknown",
                    kind: "images",
                    tokens_used: 0,
                    images_generated: 0,
                    cost_cents: 0,
                    success: false,
                    error_message: Some(&e.to_string()),
                },
            )
            .await;
            Err(ApiError::Internal(format!(
                "AI image generation failed: {}",
                e
            )))
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// Balance endpoints
// ═══════════════════════════════════════════════════════════════════

pub async fn get_balance_words(
    State(state): State<AiState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let credits = credits::get_or_init(state.db(), auth.user_id).await?;
    Ok(Json(json!({
        "data": {
            "remaining": credits.words_remaining,
            "limit": credits.words_limit,
            "plan": credits.plan,
            "reset_at": credits.reset_at,
        }
    })))
}

pub async fn get_balance_images(
    State(state): State<AiState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let credits = credits::get_or_init(state.db(), auth.user_id).await?;
    Ok(Json(json!({
        "data": {
            "remaining": credits.images_remaining,
            "limit": credits.images_limit,
            "plan": credits.plan,
            "reset_at": credits.reset_at,
        }
    })))
}

// ═══════════════════════════════════════════════════════════════════
// Legacy endpoints kept for backwards compatibility
// ═══════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub prompt: String,
    pub system_prompt: Option<String>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

pub async fn chat_completion(
    State(state): State<AiState>,
    auth: AuthUser,
    Json(req): Json<ChatRequest>,
) -> Result<Json<Value>, ApiError> {
    let est = req.max_tokens.unwrap_or(1024) as i32;
    credits::deduct(state.db(), auth.user_id, CreditKind::Words(est)).await?;

    let chain = state
        .registry
        .chain_for(crate::providers::Capability::Text)
        .await;
    let opts = GenOpts {
        max_tokens: req.max_tokens,
        temperature: req.temperature,
        system_prompt: req.system_prompt.clone(),
    };

    match factory::try_text(&chain, &req.prompt, &opts).await {
        Ok(res) => {
            credits::log_usage(
                state.db(),
                credits::UsageLog {
                    user_id: auth.user_id,
                    provider: &res.provider,
                    kind: "chat",
                    tokens_used: res.tokens_used as i32,
                    images_generated: 0,
                    cost_cents: 0,
                    success: true,
                    error_message: None,
                },
            )
            .await;
            Ok(Json(
                json!({"data": {"reply": res.content, "provider": res.provider}}),
            ))
        }
        Err(e) => Err(ApiError::Internal(format!("AI chat failed: {}", e))),
    }
}

#[derive(Debug, Deserialize)]
pub struct SuggestRequest {
    pub context: Option<String>,
    pub content_type: Option<String>,
}

pub async fn suggest_post(
    State(state): State<AiState>,
    auth: AuthUser,
    Json(req): Json<SuggestRequest>,
) -> Result<Json<Value>, ApiError> {
    generate_post(
        State(state),
        auth,
        Json(GeneratePostRequest {
            prompt: req.context,
            topic: None,
            tone: req.content_type,
            max_tokens: Some(300),
        }),
    )
    .await
}

// ═══════════════════════════════════════════════════════════════════
// Chat Suggestions (smart reply chips)
// ═══════════════════════════════════════════════════════════════════
//
// Returns 3-5 short reply suggestions ("smart reply chips") that the
// composer can render above the input. The contract is intentionally
// lean: the caller forwards the last few messages from the active
// thread and we ask the configured text provider for a tiny JSON
// array of replies.
//
// The endpoint is rate-limited via the existing word-credit budget
// (50 words estimate), so spammy clients can't drain quota by typing
// a single character.

#[derive(Debug, Deserialize)]
pub struct ChatSuggestionsRequest {
    /// Most recent thread messages. Order: oldest -> newest. The last
    /// entry is what we want to reply to. We cap at 6 entries on the
    /// server to keep prompts cheap regardless of what the client sends.
    pub messages: Vec<ChatSuggestionMessage>,
    /// Optional locale hint (e.g. "es"). Falls back to English.
    pub locale: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ChatSuggestionMessage {
    /// "me" = the local user, "them" = the other party. Anything else
    /// is treated as "them" so callers can pass usernames safely.
    pub role: String,
    pub text: String,
}

pub async fn chat_suggestions(
    State(state): State<AiState>,
    auth: AuthUser,
    Json(req): Json<ChatSuggestionsRequest>,
) -> Result<Json<Value>, ApiError> {
    if req.messages.is_empty() {
        return Ok(Json(
            json!({"data": {"suggestions": Vec::<String>::new(), "provider": "none"}}),
        ));
    }
    credits::deduct(state.db(), auth.user_id, CreditKind::Words(50)).await?;

    let locale = req.locale.as_deref().unwrap_or("en");
    let mut transcript = String::new();
    for m in req.messages.iter().rev().take(6).rev() {
        let speaker = if m.role == "me" { "Me" } else { "Them" };
        // 240 chars per turn keeps the prompt < ~1.5kB even with 6 turns.
        let text = if m.text.len() > 240 {
            &m.text[..240]
        } else {
            m.text.as_str()
        };
        transcript.push_str(&format!("{speaker}: {text}\n"));
    }

    let system = format!(
        "You generate short, natural reply suggestions for a chat app. \
         Return ONLY a JSON array of 3 to 5 strings, no prose, no markdown. \
         Each reply must be <= 40 characters, friendly, locale={locale}. \
         Do not include emoji-only replies. Do not repeat the user's last message.",
    );

    let chain = state
        .registry
        .chain_for(crate::providers::Capability::Text)
        .await;
    let opts = GenOpts {
        max_tokens: Some(150),
        temperature: Some(0.7),
        system_prompt: Some(system),
    };

    match factory::try_text(&chain, &transcript, &opts).await {
        Ok(res) => {
            let suggestions = parse_suggestion_array(&res.content);
            credits::log_usage(
                state.db(),
                credits::UsageLog {
                    user_id: auth.user_id,
                    provider: &res.provider,
                    kind: "chat_suggestions",
                    tokens_used: res.tokens_used as i32,
                    images_generated: 0,
                    cost_cents: 0,
                    success: true,
                    error_message: None,
                },
            )
            .await;
            Ok(Json(
                json!({"data": {"suggestions": suggestions, "provider": res.provider}}),
            ))
        }
        Err(e) => Err(ApiError::Internal(format!(
            "AI chat-suggestions failed: {}",
            e
        ))),
    }
}

/// Defensive parser: providers occasionally return prose around the
/// array, so we extract the first JSON array we see and fall back to
/// splitting by newline if parsing fails.
fn parse_suggestion_array(raw: &str) -> Vec<String> {
    let trimmed = raw.trim();
    if let (Some(start), Some(end)) = (trimmed.find('['), trimmed.rfind(']'))
        && start < end
    {
        let slice = &trimmed[start..=end];
        if let Ok(arr) = serde_json::from_str::<Vec<String>>(slice) {
            return arr
                .into_iter()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .take(5)
                .collect();
        }
    }
    trimmed
        .lines()
        .map(|l| {
            l.trim()
                .trim_start_matches(|c: char| {
                    c == '-' || c == '*' || c.is_ascii_digit() || c == '.'
                })
                .trim()
                .trim_matches('"')
                .to_string()
        })
        .filter(|s| !s.is_empty())
        .take(5)
        .collect()
}

#[derive(Debug, Deserialize)]
pub struct DescribeImageRequest {
    pub image_url: String,
}

pub async fn describe_image(
    State(state): State<AiState>,
    auth: AuthUser,
    Json(req): Json<DescribeImageRequest>,
) -> Result<Json<Value>, ApiError> {
    credits::deduct(state.db(), auth.user_id, CreditKind::Words(100)).await?;

    // Describe-image currently uses OpenAI vision only (no fallback).
    let key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
    if key.is_empty() {
        return Err(ApiError::Internal(
            "describe-image requires OPENAI_API_KEY".into(),
        ));
    }

    // Prefer a dedicated vision model env var and fall back to the generic
    // OPENAI_MODEL; the baked-in default is the cheapest vision-capable model.
    let vision_model = std::env::var("OPENAI_VISION_MODEL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| {
            std::env::var("OPENAI_MODEL")
                .ok()
                .filter(|s| !s.trim().is_empty())
        })
        .unwrap_or_else(|| "gpt-4o-mini".to_string());

    let body = json!({
        "model": vision_model,
        "messages": [{
            "role": "user",
            "content": [
                {"type": "text", "text": "Describe this image concisely for accessibility alt-text (max 200 chars)."},
                {"type": "image_url", "image_url": {"url": req.image_url}}
            ]
        }],
        "max_tokens": 100
    });

    let resp = state
        .http
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(&key)
        .json(&body)
        .send()
        .await
        .map_err(|e| ApiError::Internal(format!("OpenAI request failed: {}", e)))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(ApiError::Internal(format!(
            "OpenAI error {}: {}",
            status, text
        )));
    }

    let body: Value = resp
        .json()
        .await
        .map_err(|e| ApiError::Internal(format!("OpenAI parse error: {}", e)))?;

    let description = body["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or_default()
        .to_string();

    Ok(Json(json!({"data": {"description": description}})))
}

// ═══════════════════════════════════════════════════════════════════
// Admin: provider config CRUD
// ═══════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct CreateProviderRequest {
    pub name: String,
    pub provider_type: String,
    pub capability: String,
    pub api_key: String,
    pub model_text: Option<String>,
    pub model_image: Option<String>,
    pub enabled: Option<bool>,
    pub priority: Option<i32>,
}

pub async fn admin_list_providers(
    State(state): State<AiState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    if !auth.is_admin {
        return Err(ApiError::Forbidden("Admin only".into()));
    }

    type ProviderRow = (
        i64,
        String,
        String,
        String,
        String,
        Option<String>,
        Option<String>,
        bool,
        i32,
    );
    let rows: Vec<ProviderRow> = sqlx::query_as(
        r#"SELECT id, name, provider_type, capability, api_key_encrypted,
                  model_text, model_image, enabled, priority
             FROM ai_provider_config
         ORDER BY priority ASC"#,
    )
    .fetch_all(state.db())
    .await?;

    let data: Vec<Value> = rows
        .into_iter()
        .map(
            |(id, name, ptype, cap, key_enc, mt, mi, enabled, priority)| {
                let masked = crate::crypto::decrypt(&state.enc_key, &key_enc)
                    .map(|k| shared::crypto::mask_secret(&k))
                    .unwrap_or_else(|_| "****".to_string());
                json!({
                    "id": id,
                    "name": name,
                    "provider_type": ptype,
                    "capability": cap,
                    "api_key_masked": masked,
                    "model_text": mt,
                    "model_image": mi,
                    "enabled": enabled,
                    "priority": priority,
                })
            },
        )
        .collect();

    Ok(Json(json!({ "data": data })))
}

pub async fn admin_create_provider(
    State(state): State<AiState>,
    auth: AuthUser,
    Json(req): Json<CreateProviderRequest>,
) -> Result<Json<Value>, ApiError> {
    if !auth.is_admin {
        return Err(ApiError::Forbidden("Admin only".into()));
    }
    if req.api_key.trim().is_empty() {
        return Err(ApiError::BadRequest("api_key required".into()));
    }

    let encrypted = crate::crypto::encrypt(&state.enc_key, &req.api_key)
        .map_err(|e| ApiError::Internal(format!("encryption failed: {}", e)))?;

    let id: i64 = sqlx::query_scalar(
        r#"INSERT INTO ai_provider_config
            (name, provider_type, capability, api_key_encrypted,
             model_text, model_image, enabled, priority)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING id"#,
    )
    .bind(&req.name)
    .bind(&req.provider_type)
    .bind(&req.capability)
    .bind(&encrypted)
    .bind(&req.model_text)
    .bind(&req.model_image)
    .bind(req.enabled.unwrap_or(true))
    .bind(req.priority.unwrap_or(100))
    .fetch_one(state.db())
    .await?;

    Ok(Json(json!({ "data": { "id": id } })))
}

#[derive(Debug, Deserialize)]
pub struct UpdateProviderRequest {
    pub api_key: Option<String>,
    pub model_text: Option<String>,
    pub model_image: Option<String>,
    pub enabled: Option<bool>,
    pub priority: Option<i32>,
}

pub async fn admin_update_provider(
    State(state): State<AiState>,
    auth: AuthUser,
    axum::extract::Path(id): axum::extract::Path<i64>,
    Json(req): Json<UpdateProviderRequest>,
) -> Result<Json<Value>, ApiError> {
    if !auth.is_admin {
        return Err(ApiError::Forbidden("Admin only".into()));
    }

    if let Some(key) = &req.api_key
        && !key.is_empty()
    {
        let encrypted = crate::crypto::encrypt(&state.enc_key, key)
            .map_err(|e| ApiError::Internal(format!("encryption failed: {}", e)))?;
        sqlx::query("UPDATE ai_provider_config SET api_key_encrypted = $1, updated_at = NOW() WHERE id = $2")
            .bind(encrypted)
            .bind(id)
            .execute(state.db())
            .await?;
    }

    if let Some(mt) = &req.model_text {
        sqlx::query(
            "UPDATE ai_provider_config SET model_text = $1, updated_at = NOW() WHERE id = $2",
        )
        .bind(mt)
        .bind(id)
        .execute(state.db())
        .await?;
    }

    if let Some(mi) = &req.model_image {
        sqlx::query(
            "UPDATE ai_provider_config SET model_image = $1, updated_at = NOW() WHERE id = $2",
        )
        .bind(mi)
        .bind(id)
        .execute(state.db())
        .await?;
    }

    if let Some(enabled) = req.enabled {
        sqlx::query("UPDATE ai_provider_config SET enabled = $1, updated_at = NOW() WHERE id = $2")
            .bind(enabled)
            .bind(id)
            .execute(state.db())
            .await?;
    }

    if let Some(priority) = req.priority {
        sqlx::query(
            "UPDATE ai_provider_config SET priority = $1, updated_at = NOW() WHERE id = $2",
        )
        .bind(priority)
        .bind(id)
        .execute(state.db())
        .await?;
    }

    Ok(Json(json!({ "data": { "updated": true } })))
}

pub async fn admin_delete_provider(
    State(state): State<AiState>,
    auth: AuthUser,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<Json<Value>, ApiError> {
    if !auth.is_admin {
        return Err(ApiError::Forbidden("Admin only".into()));
    }
    sqlx::query("DELETE FROM ai_provider_config WHERE id = $1")
        .bind(id)
        .execute(state.db())
        .await?;
    Ok(Json(json!({ "data": { "deleted": true } })))
}

pub async fn admin_test_provider(
    State(state): State<AiState>,
    auth: AuthUser,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<Json<Value>, ApiError> {
    if !auth.is_admin {
        return Err(ApiError::Forbidden("Admin only".into()));
    }

    let row: (String, String, String, Option<String>, Option<String>) = sqlx::query_as(
        r#"SELECT provider_type, capability, api_key_encrypted, model_text, model_image
             FROM ai_provider_config WHERE id = $1"#,
    )
    .bind(id)
    .fetch_optional(state.db())
    .await?
    .ok_or(ApiError::NotFound("provider not found".into()))?;

    let api_key = crate::crypto::decrypt(&state.enc_key, &row.2)
        .map_err(|e| ApiError::Internal(format!("decryption: {}", e)))?;

    let kind = row.0.parse::<crate::providers::ProviderKind>()
        .map_err(|_| ApiError::BadRequest("unknown provider_type".into()))?;

    let model_text = row.3.unwrap_or_default();
    let model_image = row.4.unwrap_or_default();

    // Each match arm is mutually exclusive so moving is OK; the compiler validates this.
    let provider: std::sync::Arc<dyn crate::providers::AiProvider> = match kind {
        crate::providers::ProviderKind::Openai => {
            std::sync::Arc::new(crate::providers::openai::OpenAiProvider::new(
                state.http.clone(),
                api_key,
                model_text,
                model_image,
            ))
        }
        crate::providers::ProviderKind::Anthropic => {
            std::sync::Arc::new(crate::providers::anthropic::AnthropicProvider::new(
                state.http.clone(),
                api_key,
                model_text,
            ))
        }
        crate::providers::ProviderKind::Gemini => {
            std::sync::Arc::new(crate::providers::gemini::GeminiProvider::new(
                state.http.clone(),
                api_key,
                model_text,
                model_image,
            ))
        }
    };

    // Tiny probe
    let probe = provider
        .generate_text(
            "Reply with the single word: OK",
            &GenOpts {
                max_tokens: Some(5),
                temperature: Some(0.0),
                system_prompt: None,
            },
        )
        .await;

    match probe {
        Ok(res) => Ok(Json(json!({
            "data": { "ok": true, "reply": res.content, "provider": res.provider }
        }))),
        Err(e) => Ok(Json(json!({
            "data": { "ok": false, "error": e.to_string() }
        }))),
    }
}

// ── Health ──

pub async fn health_check() -> Json<Value> {
    Json(json!({ "status": "healthy", "service": "ai-service" }))
}

/// GET /v1/ai/admin/providers/health
///
/// Returns a sanitized snapshot of configured providers (no secrets) useful for
/// admin dashboards to display the resolution order + which capabilities are
/// covered.
pub async fn admin_providers_health(
    State(state): State<AiState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    auth.require_permission(Permission::ManageAi, &state.app).await?;
    let snapshot = state.registry.health_snapshot().await;

    let text_covered = snapshot
        .iter()
        .any(|(_, cap, _)| cap == "text" || cap == "both");
    let image_covered = snapshot
        .iter()
        .any(|(_, cap, _)| cap == "image" || cap == "both");

    let providers: Vec<Value> = snapshot
        .into_iter()
        .map(|(ptype, capability, priority)| {
            json!({
                "provider_type": ptype,
                "capability": capability,
                "priority": priority,
            })
        })
        .collect();

    Ok(Json(json!({
        "data": {
            "providers": providers,
            "coverage": {
                "text": text_covered,
                "image": image_covered,
            },
        }
    })))
}

// ═══════════════════════════════════════════════════════════════════
// Moderation
// ═══════════════════════════════════════════════════════════════════

#[derive(Deserialize)]
pub struct ModerateRequest {
    pub items: Vec<ModerateItem>,
}

#[derive(Deserialize)]
pub struct ModerateItem {
    pub text: Option<String>,
    pub image_url: Option<String>,
    pub kind: String, // "post", "comment", "message", "image", "profile"
    pub target_id: i64,
    pub user_id: i64,
}

pub async fn moderate_content(
    State(state): State<AiState>,
    Json(body): Json<ModerateRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let api_key = state.app.config.openai_api_key.as_deref().unwrap_or("");
    if api_key.is_empty() {
        return Err(ApiError::Internal("OpenAI API key not configured".into()));
    }

    let client = &reqwest::Client::new();
    let mut results = Vec::new();

    for item in &body.items {
        let text = item.text.as_deref().unwrap_or("");
        let image_url = item.image_url.as_deref();

        // Skip items with no content to moderate
        if text.is_empty() && image_url.is_none() {
            continue;
        }

        // Moderate text if present, otherwise moderate image
        let moderation_result = if !text.is_empty() {
            openai::moderate_text(client, api_key, text).await
        } else if let Some(url) = image_url {
            openai::moderate_text(client, api_key, url).await
        } else {
            continue;
        };

        match moderation_result {
            Ok(result) => {
                let max_score = result
                    .category_scores
                    .values()
                    .cloned()
                    .fold(0.0f64, f64::max);

                let action = moderation::decide_action(result.flagged, max_score);

                // Store result in moderation_queue with correct column names
                let _ = sqlx::query(
                    "INSERT INTO moderation_queue (target_type, target_id, submitted_by_user_id, content_text, content_image_url, openai_flagged, openai_categories, openai_scores, auto_action, status, created_at)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NOW())
                     ON CONFLICT (target_type, target_id)
                     DO UPDATE SET content_text = EXCLUDED.content_text, content_image_url = EXCLUDED.content_image_url,
                                   openai_flagged = EXCLUDED.openai_flagged, openai_categories = EXCLUDED.openai_categories,
                                   openai_scores = EXCLUDED.openai_scores, auto_action = EXCLUDED.auto_action",
                )
                .bind(&item.kind)
                .bind(item.target_id)
                .bind(item.user_id)
                .bind(text)
                .bind(image_url)
                .bind(result.flagged)
                .bind(serde_json::to_value(&result.categories).unwrap_or_default())
                .bind(serde_json::to_value(&result.category_scores).unwrap_or_default())
                .bind(action)
                .bind(action)
                .execute(state.db())
                .await
                .map_err(|e| {
                    tracing::error!(error = %e);
                    ApiError::Internal("DB error".into())
                })?;

                // Apply the moderation action
                let _ = moderation::apply_moderation_action(
                    state.db(),
                    &item.kind,
                    item.target_id,
                    action,
                )
                .await;

                results.push(serde_json::json!({
                    "target_id": item.target_id,
                    "kind": item.kind,
                    "flagged": result.flagged,
                    "action": action,
                    "max_score": max_score,
                }));
            }
            Err(e) => {
                tracing::error!(error = %e, "Moderation API error");
                results.push(serde_json::json!({
                    "target_id": item.target_id,
                    "kind": item.kind,
                    "error": e,
                }));
            }
        }
    }

    Ok(Json(serde_json::json!({ "results": results })))
}

// ═══════════════════════════════════════════════════════════════════
// OCR (OCR.space)
// ═══════════════════════════════════════════════════════════════════

#[derive(Deserialize)]
pub struct OcrRequest {
    pub image_url: String,
}

pub async fn ocr_extract(
    State(_state): State<AppState>,
    Json(body): Json<OcrRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let api_key = std::env::var("OCR_SPACE_API_KEY").unwrap_or_default();
    if api_key.is_empty() {
        return Err(ApiError::Internal("OCR API key not configured".into()));
    }

    let client = &reqwest::Client::new();
    match ocrspace::extract_text(client, &api_key, &body.image_url).await {
        Ok(result) => Ok(Json(serde_json::json!({
            "extracted_text": result.extracted_text,
            "confidence": result.confidence,
        }))),
        Err(e) => Err(ApiError::Internal(format!("OCR failed: {}", e))),
    }
}

// ═══════════════════════════════════════════════════════════════════
// Audio Transcription (Cloudflare Workers AI)
// ═══════════════════════════════════════════════════════════════════

#[derive(Deserialize)]
pub struct TranscribeRequest {
    pub audio_url: String,
}

pub async fn transcribe_audio(
    State(_state): State<AppState>,
    Json(body): Json<TranscribeRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let account_id = std::env::var("CLOUDFLARE_ACCOUNT_ID").unwrap_or_default();
    let api_token = std::env::var("CLOUDFLARE_AI_TOKEN").unwrap_or_default();

    if account_id.is_empty() || api_token.is_empty() {
        return Err(ApiError::Internal("Cloudflare AI not configured".into()));
    }

    let client = &reqwest::Client::new();
    match cloudflare_ai::transcribe(client, &account_id, &api_token, &body.audio_url).await {
        Ok(transcript) => Ok(Json(serde_json::json!({
            "transcript": transcript,
        }))),
        Err(e) => Err(ApiError::Internal(format!("Transcription failed: {}", e))),
    }
}
