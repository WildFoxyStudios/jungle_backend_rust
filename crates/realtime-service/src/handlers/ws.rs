use axum::{
    extract::{
        Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use shared::{auth::AppState, metrics::REALTIME_WS_LIFECYCLE};
use sqlx::FromRow;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::hub::{ConnectionHub, WsMessage};

#[derive(Debug, FromRow)]
struct CallerInfo {
    username: String,
    first_name: String,
    last_name: String,
    avatar: String,
}

#[derive(Debug, Deserialize)]
pub struct WsQuery {
    pub token: String,
}

type WsState = (AppState, ConnectionHub);

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State((state, hub)): State<WsState>,
    Query(query): Query<WsQuery>,
) -> impl IntoResponse {
    // Verify JWT from query param
    let user_id = match verify_ws_token(&state, &query.token) {
        Some(uid) => uid,
        None => {
            return axum::response::Response::builder()
                .status(401)
                .body(axum::body::Body::from("Unauthorized"))
                .unwrap_or_else(|e| {
                    axum::response::Response::builder()
                        .status(500)
                        .body(axum::body::Body::from(format!("Error: {}", e)))
                        .unwrap()
                })
                .into_response();
        }
    };

    ws.on_upgrade(move |socket| handle_socket(socket, user_id, state, hub))
}

async fn handle_socket(socket: WebSocket, user_id: i64, state: AppState, hub: ConnectionHub) {
    let (sender, mut receiver) = socket.split();
    let sender = Arc::new(Mutex::new(sender));

    // Auto-subscribe to the user's personal channel.
    let mut rx_user = hub.subscribe(user_id);

    tracing::info!(user_id, "WebSocket connected");

    // Greeting frame.
    hub.send_to_user(
        user_id,
        WsMessage {
            event: "connected".into(),
            data: serde_json::json!({ "user_id": user_id }),
        },
    );

    // Track topics this socket is subscribed to so we can clean up on disconnect.
    let topics: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));

    // Forward user-channel messages → WebSocket.
    let sender_user = sender.clone();
    let user_send_task = tokio::spawn(async move {
        while let Ok(msg) = rx_user.recv().await {
            let text = serde_json::to_string(&msg).unwrap_or_default();
            let mut s = sender_user.lock().await;
            if s.send(Message::Text(text.into())).await.is_err() {
                break;
            }
        }
    });

    // Read frames → process subscribe/unsubscribe + relay events.
    let hub_clone = hub.clone();
    let state_clone = state.clone();
    let topics_clone = topics.clone();
    let sender_recv = sender.clone();
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    let Ok(ws_msg) = serde_json::from_str::<WsMessage>(&text) else {
                        continue;
                    };
                    match ws_msg.event.as_str() {
                        "subscribe" => {
                            let Some(topic) = ws_msg.data.get("topic").and_then(|v| v.as_str())
                            else {
                                continue;
                            };
                            if !topic_is_allowed(user_id, topic) {
                                continue;
                            }
                            let already = {
                                let mut t = topics_clone.lock().await;
                                !t.insert(topic.to_string())
                            };
                            if already {
                                continue;
                            }
                            let mut rx_topic = hub_clone.subscribe_topic(topic);
                            let sender_topic = sender_recv.clone();
                            let topic_owned = topic.to_string();
                            tokio::spawn(async move {
                                while let Ok(m) = rx_topic.recv().await {
                                    let text = serde_json::to_string(&m).unwrap_or_default();
                                    let mut s = sender_topic.lock().await;
                                    if s.send(Message::Text(text.into())).await.is_err() {
                                        break;
                                    }
                                }
                                tracing::trace!(topic = %topic_owned, "topic forwarder ended");
                            });
                        }
                        "unsubscribe" => {
                            if let Some(topic) = ws_msg.data.get("topic").and_then(|v| v.as_str()) {
                                let mut t = topics_clone.lock().await;
                                t.remove(topic);
                                // Best-effort cleanup; the spawned forwarder
                                // will exit naturally when the broadcast
                                // channel closes (no senders / no receivers).
                                hub_clone.maybe_drop_topic(topic);
                            }
                        }
                        _ => {
                            process_client_message(user_id, &state_clone, &hub_clone, ws_msg).await;
                        }
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    tokio::select! {
        _ = user_send_task => {},
        _ = recv_task => {},
    }

    let topic_names: Vec<String> = topics.lock().await.drain().collect();
    for t in &topic_names {
        hub.maybe_drop_topic(t);
    }

    hub.unsubscribe(user_id);
    REALTIME_WS_LIFECYCLE.with_label_values(&["disconnect"]).inc();
    tracing::info!(user_id, "WebSocket disconnected");
}

/// Topic ACL: limit which topics a given user is allowed to subscribe to.
/// `user:N` only allowed for the same user. `feed:home` is the user's own
/// feed; everything else is currently allowed (live rooms, group walls, …).
fn topic_is_allowed(user_id: i64, topic: &str) -> bool {
    if let Some(rest) = topic.strip_prefix("user:") {
        return rest.parse::<i64>().ok() == Some(user_id);
    }
    if topic == "feed:home" || topic.starts_with("feed:") {
        return true;
    }
    if topic.starts_with("live:")
        || topic.starts_with("group:")
        || topic.starts_with("page:")
        || topic.starts_with("post:")
        || topic.starts_with("conversation:")
    {
        return true;
    }
    tracing::debug!(user_id, topic, "rejecting subscribe to unknown topic");
    false
}

async fn process_client_message(
    user_id: i64,
    state: &AppState,
    hub: &ConnectionHub,
    msg: WsMessage,
) {
    match msg.event.as_str() {
        "typing" | "typing.start" => {
            if let Some(to_id) = msg.data["to_user_id"].as_i64() {
                let conversation_id = msg.data["conversation_id"].as_i64().unwrap_or(0);
                hub.send_to_user(
                    to_id,
                    WsMessage {
                        event: "typing.start".into(),
                        data: serde_json::json!({ "user_id": user_id, "conversation_id": conversation_id }),
                    },
                );
            }
        }
        "stop_typing" | "typing.stop" => {
            if let Some(to_id) = msg.data["to_user_id"].as_i64() {
                let conversation_id = msg.data["conversation_id"].as_i64().unwrap_or(0);
                hub.send_to_user(
                    to_id,
                    WsMessage {
                        event: "typing.stop".into(),
                        data: serde_json::json!({ "user_id": user_id, "conversation_id": conversation_id }),
                    },
                );
            }
        }
        "ping" => {
            hub.send_to_user(
                user_id,
                WsMessage {
                    event: "pong".into(),
                    data: serde_json::json!({}),
                },
            );
        }
        "message" | "message.new" => {
            if let Some(to_id) = msg.data["to_user_id"].as_i64() {
                hub.send_to_user(
                    to_id,
                    WsMessage {
                        event: "message.new".into(),
                        data: serde_json::json!({
                            "from_user_id": user_id,
                            "message": msg.data.get("message")
                        }),
                    },
                );
            }
        }
        // WebRTC signaling
        "call_offer" => {
            // Caller announces a new call. We enrich with the caller's public
            // identity so the callee UI can render name + avatar directly.
            if let Some(to_id) = msg.data["to_user_id"].as_i64() {
                let room = msg.data.get("room").cloned().unwrap_or_default();
                let sdp = msg.data.get("sdp").cloned().unwrap_or_default();
                let audio_only = msg
                    .data
                    .get("audio_only")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let caller = sqlx::query_as::<_, CallerInfo>(
                    "SELECT username, first_name, last_name, avatar FROM users WHERE id = $1",
                )
                .bind(user_id)
                .fetch_optional(&state.db)
                .await
                .ok()
                .flatten();

                let caller_json = caller
                    .map(|c| {
                        serde_json::json!({
                            "id": user_id,
                            "username": c.username,
                            "first_name": c.first_name,
                            "last_name": c.last_name,
                            "avatar": c.avatar,
                        })
                    })
                    .unwrap_or_else(|| serde_json::json!({ "id": user_id }));

                hub.send_to_user(
                    to_id,
                    WsMessage {
                        event: "call.incoming".into(),
                        data: serde_json::json!({
                            "room": room,
                            "caller": caller_json,
                            "audio_only": audio_only,
                            "sdp": sdp,
                        }),
                    },
                );
            }
        }
        "call_answer" | "call_ice_candidate" | "call_end" => {
            // Simple relay: the callee/caller echo back negotiation messages.
            if let Some(to_id) = msg.data["to_user_id"].as_i64() {
                hub.send_to_user(
                    to_id,
                    WsMessage {
                        event: msg.event.clone(),
                        data: serde_json::json!({
                            "from_user_id": user_id,
                            "room": msg.data.get("room"),
                            "sdp": msg.data.get("sdp"),
                            "candidate": msg.data.get("candidate"),
                        }),
                    },
                );
            }
        }
        _ => {
            tracing::debug!(user_id, event = %msg.event, "Unknown WS event");
        }
    }
}

fn verify_ws_token(state: &AppState, token: &str) -> Option<i64> {
    use jsonwebtoken::{DecodingKey, Validation, decode};
    use shared::auth::Claims;

    let key = DecodingKey::from_secret(state.config.jwt_secret.as_bytes());
    let mut validation = Validation::default();
    validation.set_required_spec_claims(&["exp", "iat"]);

    decode::<Claims>(token, &key, &validation)
        .ok()
        .map(|td| td.claims.sub)
}
