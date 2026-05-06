use crate::proxy::GatewayState;
use axum::{
    extract::{Query, State, WebSocketUpgrade, ws::WebSocket},
    response::{IntoResponse, Response},
};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct WsQuery {
    pub token: String,
}

#[derive(Deserialize)]
pub struct WsLiveNativeQuery {
    pub token: String,
    pub room_id: String,
}

pub async fn ws_proxy_handler(
    ws: WebSocketUpgrade,
    State(state): State<GatewayState>,
    Query(query): Query<WsQuery>,
) -> Response {
    let upstream_base = match state.services.resolve("/ws") {
        Some(url) => url.to_string(),
        None => {
            return (axum::http::StatusCode::BAD_GATEWAY, "No upstream for /ws").into_response();
        }
    };

    // Convert http:// to ws://
    let ws_base = upstream_base
        .replace("http://", "ws://")
        .replace("https://", "wss://");
    let upstream_url = format!("{}/ws?token={}", ws_base, query.token);

    ws.on_upgrade(move |socket| bridge_websocket(socket, upstream_url))
}

pub async fn ws_live_native_proxy_handler(
    ws: WebSocketUpgrade,
    State(state): State<GatewayState>,
    Query(query): Query<WsLiveNativeQuery>,
) -> Response {
    let upstream_base = match state.services.resolve("/ws/live-native") {
        Some(url) => url.to_string(),
        None => {
            return (
                axum::http::StatusCode::BAD_GATEWAY,
                "No upstream for /ws/live-native",
            )
                .into_response();
        }
    };

    let ws_base = upstream_base
        .replace("http://", "ws://")
        .replace("https://", "wss://");
    let upstream_url = format!(
        "{}/ws/live-native?token={}&room_id={}",
        ws_base, query.token, query.room_id
    );

    ws.on_upgrade(move |socket| bridge_websocket(socket, upstream_url))
}

async fn bridge_websocket(client_socket: WebSocket, upstream_url: String) {
    use tokio_tungstenite::tungstenite::Message as TungMsg;

    // Connect to upstream realtime-service
    let upstream = match tokio_tungstenite::connect_async(&upstream_url).await {
        Ok((stream, _)) => stream,
        Err(e) => {
            tracing::error!(error = %e, url = %upstream_url, "Failed to connect to upstream WebSocket");
            return;
        }
    };

    let (mut client_tx, mut client_rx) = client_socket.split();
    let (mut upstream_tx, mut upstream_rx) = upstream.split();

    // Forward: client → upstream
    let client_to_upstream = tokio::spawn(async move {
        while let Some(Ok(msg)) = client_rx.next().await {
            let tung_msg = match msg {
                axum::extract::ws::Message::Text(t) => TungMsg::Text(t.to_string()),
                axum::extract::ws::Message::Binary(b) => TungMsg::Binary(b.to_vec()),
                axum::extract::ws::Message::Ping(p) => TungMsg::Ping(p.to_vec()),
                axum::extract::ws::Message::Pong(p) => TungMsg::Pong(p.to_vec()),
                axum::extract::ws::Message::Close(_) => {
                    let _ = upstream_tx.close().await;
                    break;
                }
            };
            if upstream_tx.send(tung_msg).await.is_err() {
                break;
            }
        }
    });

    // Forward: upstream → client
    let upstream_to_client = tokio::spawn(async move {
        while let Some(Ok(msg)) = upstream_rx.next().await {
            let axum_msg = match msg {
                TungMsg::Text(t) => axum::extract::ws::Message::Text(t.into()),
                TungMsg::Binary(b) => axum::extract::ws::Message::Binary(b.into()),
                TungMsg::Ping(p) => axum::extract::ws::Message::Ping(p.into()),
                TungMsg::Pong(p) => axum::extract::ws::Message::Pong(p.into()),
                TungMsg::Close(_) => {
                    let _ = client_tx.close().await;
                    break;
                }
                TungMsg::Frame(_) => continue,
            };
            if client_tx.send(axum_msg).await.is_err() {
                break;
            }
        }
    });

    // Wait for either direction to finish, then abort the other
    tokio::select! {
        _ = client_to_upstream => {},
        _ = upstream_to_client => {},
    }
}
