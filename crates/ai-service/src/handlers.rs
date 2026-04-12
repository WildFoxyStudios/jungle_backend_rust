use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Clone)]
pub struct AiState {
    pub http: reqwest::Client,
    pub _redis: redis::aio::ConnectionManager,
    pub openai_key: String,
    pub openai_model: String,
}

// ── Chat Completion ──

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub prompt: String,
    pub system_prompt: Option<String>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

#[derive(Debug, Serialize)]
struct OpenAiRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
    max_tokens: u32,
    temperature: f32,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAiMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenAiResponse {
    choices: Vec<OpenAiChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: OpenAiMessage,
}

pub async fn chat_completion(
    State(state): State<AiState>,
    Json(req): Json<ChatRequest>,
) -> Result<Json<Value>, (axum::http::StatusCode, Json<Value>)> {
    if state.openai_key.is_empty() {
        return Err((
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error": "OpenAI API key not configured"})),
        ));
    }

    let mut messages = Vec::new();
    if let Some(sys) = &req.system_prompt {
        messages.push(OpenAiMessage {
            role: "system".into(),
            content: sys.clone(),
        });
    }
    messages.push(OpenAiMessage {
        role: "user".into(),
        content: req.prompt.clone(),
    });

    let body = OpenAiRequest {
        model: state.openai_model.clone(),
        messages,
        max_tokens: req.max_tokens.unwrap_or(1024),
        temperature: req.temperature.unwrap_or(0.7),
    };

    let response = state
        .http
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", state.openai_key))
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::BAD_GATEWAY,
                Json(json!({"error": e.to_string()})),
            )
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err((
            axum::http::StatusCode::BAD_GATEWAY,
            Json(json!({"error": format!("OpenAI API error ({}): {}", status, text)})),
        ));
    }

    let ai_resp: OpenAiResponse = response.json().await.map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
    })?;

    let reply = ai_resp
        .choices
        .first()
        .map(|c| c.message.content.clone())
        .unwrap_or_default();

    Ok(Json(json!({ "data": { "reply": reply } })))
}

// ── Text Generation (Post Suggestions) ──

#[derive(Debug, Deserialize)]
pub struct SuggestRequest {
    pub context: Option<String>,
    pub content_type: Option<String>,
}

pub async fn suggest_post(
    State(state): State<AiState>,
    Json(req): Json<SuggestRequest>,
) -> Result<Json<Value>, (axum::http::StatusCode, Json<Value>)> {
    let system = format!(
        "You are a social media post writer. Generate an engaging {} post. Be creative, concise, and use appropriate emojis.",
        req.content_type.as_deref().unwrap_or("general")
    );
    let prompt = req
        .context
        .unwrap_or_else(|| "Write an interesting social media post".into());

    chat_completion(
        State(state),
        Json(ChatRequest {
            prompt,
            system_prompt: Some(system),
            max_tokens: Some(300),
            temperature: Some(0.9),
        }),
    )
    .await
}

// ── Image Description (Accessibility) ──

#[derive(Debug, Deserialize)]
pub struct DescribeImageRequest {
    pub image_url: String,
}

pub async fn describe_image(
    State(state): State<AiState>,
    Json(req): Json<DescribeImageRequest>,
) -> Result<Json<Value>, (axum::http::StatusCode, Json<Value>)> {
    if state.openai_key.is_empty() {
        return Err((
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error": "OpenAI API key not configured"})),
        ));
    }

    let body = json!({
        "model": "gpt-4o-mini",
        "messages": [
            {
                "role": "user",
                "content": [
                    {"type": "text", "text": "Describe this image concisely for accessibility alt-text (max 200 chars)."},
                    {"type": "image_url", "image_url": {"url": req.image_url}}
                ]
            }
        ],
        "max_tokens": 100
    });

    let response = state
        .http
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", state.openai_key))
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::BAD_GATEWAY,
                Json(json!({"error": e.to_string()})),
            )
        })?;

    let ai_resp: OpenAiResponse = response.json().await.map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
    })?;

    let description = ai_resp
        .choices
        .first()
        .map(|c| c.message.content.clone())
        .unwrap_or_default();

    Ok(Json(json!({ "data": { "description": description } })))
}

// ── Health ──

pub async fn health_check() -> Json<Value> {
    Json(json!({ "status": "healthy", "service": "ai-service" }))
}
