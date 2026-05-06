use axum::{
    Json, Router,
    extract::{
        Path, Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
};
use futures::{SinkExt, StreamExt};
use http::header::AUTHORIZATION;
use jsonwebtoken::{DecodingKey, Validation, decode};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use shared::{
    auth::{AppState, Claims},
    errors::ApiError,
};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use time::OffsetDateTime;
use tokio::sync::{RwLock, broadcast};

#[derive(Clone)]
pub struct LiveState {
    rooms: Arc<RwLock<HashMap<String, Room>>>,
    channels: Arc<RwLock<HashMap<String, broadcast::Sender<SignalEnvelope>>>>,
}

impl Default for LiveState {
    fn default() -> Self {
        Self::new()
    }
}

impl LiveState {
    pub fn new() -> Self {
        Self {
            rooms: Arc::new(RwLock::new(HashMap::new())),
            channels: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoomKind {
    Live,
    AudioCall,
    VideoCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Room {
    id: String,
    owner_id: i64,
    title: String,
    kind: RoomKind,
    max_participants: i32,
    created_at: OffsetDateTime,
    participants: HashSet<i64>,
}

#[derive(Debug, Deserialize)]
struct CreateRoomRequest {
    title: String,
    kind: RoomKind,
    max_participants: Option<i32>,
}

#[derive(Debug, Serialize)]
struct RoomSummary {
    id: String,
    owner_id: i64,
    title: String,
    kind: RoomKind,
    max_participants: i32,
    participants_count: usize,
    created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SignalEnvelope {
    pub room_id: String,
    pub from_user_id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_user_id: Option<i64>,
    pub kind: String,
    pub payload: Value,
    pub sent_at: String,
}

#[derive(Debug, Deserialize)]
struct WsQuery {
    token: String,
    room_id: String,
}

#[derive(Debug, Deserialize)]
struct ClientSignal {
    kind: String,
    #[serde(default)]
    target_user_id: Option<i64>,
    #[serde(default)]
    payload: Value,
}

#[derive(Clone)]
pub struct ServiceState {
    pub app: AppState,
    pub live: LiveState,
}

pub fn create_router(state: ServiceState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/v1/live-native/rooms", post(create_room).get(list_rooms))
        .route("/v1/live-native/rooms/{room_id}", get(get_room))
        .route("/v1/live-native/rooms/{room_id}/join", post(join_room))
        .route("/v1/live-native/rooms/{room_id}/leave", post(leave_room))
        .route("/v1/live-native/ice-config", get(ice_config))
        .route("/ws/live-native", get(ws_live))
        .with_state(state)
}

async fn health() -> Json<Value> {
    Json(json!({ "status": "healthy", "service": "live-service" }))
}

fn user_id_from_headers(app: &AppState, headers: &HeaderMap) -> Result<i64, ApiError> {
    let auth_header = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or(ApiError::Unauthorized)?;
    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(ApiError::Unauthorized)?;
    let key = DecodingKey::from_secret(app.config.jwt_secret.as_bytes());
    let mut validation = Validation::default();
    validation.set_required_spec_claims(&["exp", "iat"]);
    decode::<Claims>(token, &key, &validation)
        .map(|d| d.claims.sub)
        .map_err(|_| ApiError::Unauthorized)
}

fn to_summary(room: &Room) -> RoomSummary {
    RoomSummary {
        id: room.id.clone(),
        owner_id: room.owner_id,
        title: room.title.clone(),
        kind: room.kind.clone(),
        max_participants: room.max_participants,
        participants_count: room.participants.len(),
        created_at: room.created_at.to_string(),
    }
}

async fn create_room(
    State(state): State<ServiceState>,
    headers: HeaderMap,
    Json(req): Json<CreateRoomRequest>,
) -> Result<Json<Value>, ApiError> {
    let user_id = user_id_from_headers(&state.app, &headers)?;
    if req.title.trim().is_empty() {
        return Err(ApiError::BadRequest("title is required".into()));
    }
    let room_id = format!("room_{}", OffsetDateTime::now_utc().unix_timestamp_nanos());
    let mut participants = HashSet::new();
    participants.insert(user_id);
    let room = Room {
        id: room_id.clone(),
        owner_id: user_id,
        title: req.title.trim().to_string(),
        kind: req.kind,
        max_participants: req.max_participants.unwrap_or(50).clamp(2, 10000),
        created_at: OffsetDateTime::now_utc(),
        participants,
    };
    state
        .live
        .rooms
        .write()
        .await
        .insert(room_id.clone(), room.clone());
    let (tx, _) = broadcast::channel::<SignalEnvelope>(512);
    state
        .live
        .channels
        .write()
        .await
        .insert(room_id.clone(), tx);
    Ok(Json(json!({ "data": to_summary(&room) })))
}

async fn list_rooms(
    State(state): State<ServiceState>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let _ = user_id_from_headers(&state.app, &headers)?;
    let rooms = state.live.rooms.read().await;
    let data: Vec<RoomSummary> = rooms.values().map(to_summary).collect();
    Ok(Json(json!({ "data": data })))
}

async fn get_room(
    State(state): State<ServiceState>,
    headers: HeaderMap,
    Path(room_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let _ = user_id_from_headers(&state.app, &headers)?;
    let rooms = state.live.rooms.read().await;
    let room = rooms
        .get(&room_id)
        .ok_or_else(|| ApiError::NotFound("Room not found".into()))?;
    Ok(Json(json!({ "data": to_summary(room) })))
}

async fn join_room(
    State(state): State<ServiceState>,
    headers: HeaderMap,
    Path(room_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let user_id = user_id_from_headers(&state.app, &headers)?;
    let mut rooms = state.live.rooms.write().await;
    let room = rooms
        .get_mut(&room_id)
        .ok_or_else(|| ApiError::NotFound("Room not found".into()))?;
    if room.participants.len() >= room.max_participants as usize {
        return Err(ApiError::BadRequest("room is full".into()));
    }
    room.participants.insert(user_id);
    Ok(Json(
        json!({ "data": { "joined": true, "room_id": room_id } }),
    ))
}

async fn leave_room(
    State(state): State<ServiceState>,
    headers: HeaderMap,
    Path(room_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let user_id = user_id_from_headers(&state.app, &headers)?;
    let mut rooms = state.live.rooms.write().await;
    let room = rooms
        .get_mut(&room_id)
        .ok_or_else(|| ApiError::NotFound("Room not found".into()))?;
    room.participants.remove(&user_id);
    Ok(Json(
        json!({ "data": { "left": true, "room_id": room_id } }),
    ))
}

async fn ice_config(
    State(state): State<ServiceState>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let _ = user_id_from_headers(&state.app, &headers)?;
    let rows = sqlx::query_as::<_, (String, String)>(
        "SELECT key, value FROM site_config WHERE category = 'live' ORDER BY key",
    )
    .fetch_all(&state.app.db)
    .await
    .unwrap_or_default();
    let mut m = HashMap::<String, String>::new();
    for (k, v) in rows {
        m.insert(k, v);
    }
    let stun_url = m
        .get("stun_server_url")
        .cloned()
        .unwrap_or_else(|| "stun:stun.l.google.com:19302".to_string());
    let turn_url = m.get("turn_server_url").cloned().unwrap_or_default();
    let turn_user = m.get("turn_username").cloned().unwrap_or_default();
    let turn_pass = m.get("turn_password").cloned().unwrap_or_default();

    let mut ice_servers = vec![json!({ "urls": [stun_url] })];
    if !turn_url.is_empty() {
        ice_servers.push(json!({
            "urls": [turn_url],
            "username": turn_user,
            "credential": turn_pass
        }));
    }
    Ok(Json(json!({ "data": { "ice_servers": ice_servers } })))
}

async fn ws_live(
    ws: WebSocketUpgrade,
    State(state): State<ServiceState>,
    Query(q): Query<WsQuery>,
) -> impl IntoResponse {
    let user_id = {
        let key = DecodingKey::from_secret(state.app.config.jwt_secret.as_bytes());
        let mut validation = Validation::default();
        validation.set_required_spec_claims(&["exp", "iat"]);
        match decode::<Claims>(&q.token, &key, &validation) {
            Ok(td) => td.claims.sub,
            Err(_) => {
                return axum::response::Response::builder()
                    .status(StatusCode::UNAUTHORIZED)
                    .body(axum::body::Body::from("Unauthorized"))
                    .unwrap_or_else(|e| {
                        axum::response::Response::builder()
                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                            .body(axum::body::Body::from(format!("Error: {}", e)))
                            .unwrap()
                    })
                    .into_response();
            }
        }
    };
    ws.on_upgrade(move |socket| handle_ws(socket, state.live.clone(), q.room_id, user_id))
}

async fn handle_ws(socket: WebSocket, live: LiveState, room_id: String, user_id: i64) {
    let tx = {
        let mut channels = live.channels.write().await;
        channels
            .entry(room_id.clone())
            .or_insert_with(|| {
                let (tx, _) = broadcast::channel::<SignalEnvelope>(512);
                tx
            })
            .clone()
    };
    let mut rx = tx.subscribe();
    {
        let mut rooms = live.rooms.write().await;
        if let Some(room) = rooms.get_mut(&room_id) {
            room.participants.insert(user_id);
        }
    }

    let (mut sender, mut receiver) = socket.split();
    let send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if (msg.target_user_id.is_none()
                || msg.target_user_id == Some(user_id)
                || msg.from_user_id == user_id)
                && sender
                    .send(Message::Text(
                        serde_json::to_string(&msg).unwrap_or_default().into(),
                    ))
                    .await
                    .is_err()
                {
                    break;
                }
        }
    });

    while let Some(Ok(msg)) = receiver.next().await {
        if let Message::Text(text) = msg
            && let Ok(in_msg) = serde_json::from_str::<ClientSignal>(&text) {
                let _ = tx.send(SignalEnvelope {
                    room_id: room_id.clone(),
                    from_user_id: user_id,
                    target_user_id: in_msg.target_user_id,
                    kind: in_msg.kind,
                    payload: in_msg.payload,
                    sent_at: OffsetDateTime::now_utc().to_string(),
                });
            }
    }

    send_task.abort();
    let mut rooms = live.rooms.write().await;
    if let Some(room) = rooms.get_mut(&room_id) {
        room.participants.remove(&user_id);
    }
}
