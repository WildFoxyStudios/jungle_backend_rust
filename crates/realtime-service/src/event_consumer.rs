use crate::hub::{ConnectionHub, WsMessage};
use shared::events::{DomainEvent, EventBus};
use std::sync::Arc;
use tracing::{info, warn};

/// Spawns a background task that subscribes to NATS domain events
/// and relays them to connected WebSocket clients via the ConnectionHub.
pub async fn spawn_event_consumer(
    event_bus: Arc<dyn EventBus>,
    hub: ConnectionHub,
) {
    let mut subscription = match event_bus.subscribe("events.>").await {
        Ok(sub) => {
            info!("realtime-service: subscribed to NATS events.>");
            sub
        }
        Err(e) => {
            warn!("realtime-service: failed to subscribe to NATS events: {e}. WebSocket relay disabled.");
            return;
        }
    };

    loop {
        match subscription.next().await {
            Some((_subject, event)) => {
                relay_event_to_ws(&hub, &event);
            }
            None => {
                warn!("realtime-service: NATS subscription closed, exiting event consumer");
                break;
            }
        }
    }
}

/// Determine which users should receive a WebSocket message for this event
/// and push it through the ConnectionHub.
fn relay_event_to_ws(hub: &ConnectionHub, event: &DomainEvent) {
    match event {
        DomainEvent::MessageSent { sender_id, recipient_ids, conversation_id } => {
            let msg = WsMessage {
                event: "new_message".to_string(),
                data: serde_json::json!({
                    "conversation_id": conversation_id,
                    "sender_id": sender_id,
                }),
            };
            hub.send_to_users(recipient_ids, msg);
        }

        DomainEvent::MessageRead { conversation_id, user_id } => {
            let msg = WsMessage {
                event: "message_read".to_string(),
                data: serde_json::json!({
                    "conversation_id": conversation_id,
                    "user_id": user_id,
                }),
            };
            // All members of the conversation would ideally be notified;
            // for now, we broadcast to the user who triggered the read.
            hub.send_to_user(*user_id, msg);
        }

        DomainEvent::TypingStarted { conversation_id, user_id } => {
            let msg = WsMessage {
                event: "typing_start".to_string(),
                data: serde_json::json!({
                    "conversation_id": conversation_id,
                    "user_id": user_id,
                }),
            };
            hub.send_to_user(*user_id, msg);
        }

        DomainEvent::TypingStopped { conversation_id, user_id } => {
            let msg = WsMessage {
                event: "typing_stop".to_string(),
                data: serde_json::json!({
                    "conversation_id": conversation_id,
                    "user_id": user_id,
                }),
            };
            hub.send_to_user(*user_id, msg);
        }

        DomainEvent::NotificationCreated { recipient_id, notification_type, sender_id } => {
            let msg = WsMessage {
                event: "notification".to_string(),
                data: serde_json::json!({
                    "type": notification_type,
                    "sender_id": sender_id,
                }),
            };
            hub.send_to_user(*recipient_id, msg);
        }

        DomainEvent::CallStarted { caller_id, callee_id, call_id, call_type } => {
            let msg = WsMessage {
                event: "incoming_call".to_string(),
                data: serde_json::json!({
                    "call_id": call_id,
                    "caller_id": caller_id,
                    "call_type": call_type,
                }),
            };
            hub.send_to_user(*callee_id, msg);
        }

        DomainEvent::CallAnswered { call_id } => {
            let msg = WsMessage {
                event: "call_answered".to_string(),
                data: serde_json::json!({ "call_id": call_id }),
            };
            // Broadcast to all online users who might be in this call
            // In practice, you'd look up the call participants
            let _ = msg;
        }

        DomainEvent::CallEnded { call_id } => {
            let msg = WsMessage {
                event: "call_ended".to_string(),
                data: serde_json::json!({ "call_id": call_id }),
            };
            let _ = msg;
        }

        DomainEvent::PresenceOnline { user_id } => {
            let msg = WsMessage {
                event: "presence_online".to_string(),
                data: serde_json::json!({ "user_id": user_id }),
            };
            // Could broadcast to the user's friends/followers
            let _ = msg;
        }

        DomainEvent::PresenceOffline { user_id } => {
            let msg = WsMessage {
                event: "presence_offline".to_string(),
                data: serde_json::json!({ "user_id": user_id }),
            };
            let _ = msg;
        }

        DomainEvent::LiveStreamStarted { stream_id, user_id } => {
            let msg = WsMessage {
                event: "live_started".to_string(),
                data: serde_json::json!({
                    "stream_id": stream_id,
                    "user_id": user_id,
                }),
            };
            // Broadcast to followers would require DB lookup;
            // for now notify the streamer
            hub.send_to_user(*user_id, msg);
        }

        DomainEvent::LiveStreamEnded { stream_id, user_id } => {
            let msg = WsMessage {
                event: "live_ended".to_string(),
                data: serde_json::json!({
                    "stream_id": stream_id,
                    "user_id": user_id,
                }),
            };
            hub.send_to_user(*user_id, msg);
        }

        // Events that don't need WebSocket relay
        DomainEvent::UserCreated { .. }
        | DomainEvent::UserUpdated { .. }
        | DomainEvent::UserDeleted { .. }
        | DomainEvent::FollowCreated { .. }
        | DomainEvent::FollowDeleted { .. }
        | DomainEvent::UserBlocked { .. }
        | DomainEvent::PostCreated { .. }
        | DomainEvent::PostDeleted { .. }
        | DomainEvent::PostLiked { .. }
        | DomainEvent::CommentCreated { .. }
        | DomainEvent::GroupJoined { .. }
        | DomainEvent::GroupLeft { .. }
        | DomainEvent::PageLiked { .. }
        | DomainEvent::StoryCreated { .. }
        | DomainEvent::PaymentCompleted { .. }
        | DomainEvent::NewsletterQueued { .. } => {}

        DomainEvent::AdminNotice { text, target: _ } => {
            let msg = WsMessage {
                event: "admin_notice".to_string(),
                data: serde_json::json!({ "text": text }),
            };
            hub.broadcast(msg);
        }
    }
}
