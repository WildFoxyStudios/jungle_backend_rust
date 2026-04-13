use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use shared::auth::AppState;

use crate::hub::{ConnectionHub, WsMessage};

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
                .unwrap()
                .into_response();
        }
    };

    ws.on_upgrade(move |socket| handle_socket(socket, user_id, hub))
}

async fn handle_socket(socket: WebSocket, user_id: i64, hub: ConnectionHub) {
    let (mut sender, mut receiver) = socket.split();

    // Subscribe to user's channel
    let mut rx = hub.subscribe(user_id);

    tracing::info!(user_id, "WebSocket connected");

    // Broadcast join event
    hub.send_to_user(
        user_id,
        WsMessage {
            event: "connected".into(),
            data: serde_json::json!({ "user_id": user_id }),
        },
    );

    // Task: forward hub messages → WebSocket
    let hub_clone = hub.clone();
    let send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            let text = serde_json::to_string(&msg).unwrap_or_default();
            if sender.send(Message::Text(text.into())).await.is_err() {
                break;
            }
        }
    });

    // Task: read WebSocket messages → process
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    if let Ok(ws_msg) = serde_json::from_str::<WsMessage>(&text) {
                        process_client_message(user_id, &hub_clone, ws_msg).await;
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    // Wait for either task to finish
    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    }

    hub.unsubscribe(user_id);
    tracing::info!(user_id, "WebSocket disconnected");
}

async fn process_client_message(user_id: i64, hub: &ConnectionHub, msg: WsMessage) {
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
        "call_offer" | "call_answer" | "call_ice_candidate" | "call_end" => {
            // WebRTC signaling relay
            if let Some(to_id) = msg.data["to_user_id"].as_i64() {
                hub.send_to_user(
                    to_id,
                    WsMessage {
                        event: msg.event.clone(),
                        data: serde_json::json!({
                            "from_user_id": user_id,
                            "payload": msg.data.get("payload")
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
    use jsonwebtoken::{decode, DecodingKey, Validation};
    use shared::auth::Claims;

    let key = DecodingKey::from_secret(state.config.jwt_secret.as_bytes());
    let mut validation = Validation::default();
    validation.set_required_spec_claims(&["sub", "exp", "iat"]);

    decode::<Claims>(token, &key, &validation)
        .ok()
        .map(|td| td.claims.sub)
}
