use crate::hub::{ConnectionHub, WsMessage};
use shared::events::{DomainEvent, EventBus};
use sqlx::PgPool;
use std::sync::Arc;
use tracing::{info, warn};

/// Spawns a background task that subscribes to NATS domain events
/// and relays them to connected WebSocket clients via the ConnectionHub.
pub async fn spawn_event_consumer(event_bus: Arc<dyn EventBus>, hub: ConnectionHub, pool: PgPool) {
    let mut subscription = match event_bus.subscribe("events.>").await {
        Ok(sub) => {
            info!("realtime-service: subscribed to NATS events.>");
            sub
        }
        Err(e) => {
            warn!(
                "realtime-service: failed to subscribe to NATS events: {e}. WebSocket relay disabled."
            );
            return;
        }
    };

    loop {
        match subscription.next().await {
            Some((_subject, event)) => {
                relay_event_to_ws(&hub, &event, &pool).await;
            }
            None => {
                warn!("realtime-service: NATS subscription closed, exiting event consumer");
                break;
            }
        }
    }
}

/// Load the active followers of a user (those who should see presence/name changes).
async fn load_followers(pool: &PgPool, user_id: i64) -> Vec<i64> {
    match sqlx::query_scalar::<_, i64>(
        "SELECT follower_id FROM follows WHERE following_id = $1 AND status = 'active'",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            warn!(user_id, error = %e, "failed to load followers for fan-out");
            Vec::new()
        }
    }
}

/// Load the caller/callee pair for a call id.
/// Active conversation members except `exclude_user_id` (for typing fan-out).
/// Active members (for read-receipt fan-out to every participant + multi-device sync).
async fn conversation_member_ids(pool: &PgPool, conversation_id: i64) -> Vec<i64> {
    match sqlx::query_scalar::<_, i64>(
        r#"
        SELECT user_id FROM conversation_members
        WHERE conversation_id = $1 AND is_active = TRUE
        "#,
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            warn!(
                conversation_id,
                error = %e,
                "failed to load conversation members for message.seen"
            );
            Vec::new()
        }
    }
}

async fn conversation_peers_excluding(
    pool: &PgPool,
    conversation_id: i64,
    exclude_user_id: i64,
) -> Vec<i64> {
    match sqlx::query_scalar::<_, i64>(
        r#"
        SELECT user_id FROM conversation_members
        WHERE conversation_id = $1 AND is_active = TRUE AND user_id <> $2
        "#,
    )
    .bind(conversation_id)
    .bind(exclude_user_id)
    .fetch_all(pool)
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            warn!(
                conversation_id,
                error = %e,
                "failed to load conversation peers for typing relay"
            );
            Vec::new()
        }
    }
}

async fn load_call_participants(pool: &PgPool, call_id: i64) -> Option<(i64, i64)> {
    match sqlx::query_as::<_, (i64, i64)>("SELECT caller_id, callee_id FROM calls WHERE id = $1")
        .bind(call_id)
        .fetch_optional(pool)
        .await
    {
        Ok(Some(pair)) => Some(pair),
        Ok(None) => None,
        Err(e) => {
            warn!(call_id, error = %e, "failed to resolve call participants");
            None
        }
    }
}

/// Determine which users should receive a WebSocket message for this event
/// and push it through the ConnectionHub.
async fn relay_event_to_ws(hub: &ConnectionHub, event: &DomainEvent, pool: &PgPool) {
    match event {
        DomainEvent::MessageSent {
            sender_id,
            recipient_ids,
            conversation_id,
        } => {
            tracing::info!(
                conversation_id,
                sender_id,
                recipients = recipient_ids.len(),
                "ws relay message.new"
            );
            let msg = WsMessage {
                event: "message.new".to_string(),
                data: serde_json::json!({
                    "conversation_id": conversation_id,
                    "sender_id": sender_id,
                }),
            };
            hub.send_to_users(recipient_ids, msg);
        }

        DomainEvent::MessageRead {
            conversation_id,
            user_id,
        } => {
            let recipients = conversation_member_ids(pool, *conversation_id).await;
            let msg = WsMessage {
                event: "message.seen".to_string(),
                data: serde_json::json!({
                    "conversation_id": conversation_id,
                    "user_id": user_id,
                }),
            };
            hub.send_to_users(&recipients, msg);
        }

        DomainEvent::TypingStarted {
            conversation_id,
            user_id,
        } => {
            let msg = WsMessage {
                event: "message.typing.start".to_string(),
                data: serde_json::json!({
                    "conversation_id": conversation_id,
                    "user_id": user_id,
                }),
            };
            let peers =
                conversation_peers_excluding(pool, *conversation_id, *user_id).await;
            hub.send_to_users(&peers, msg);
        }

        DomainEvent::TypingStopped {
            conversation_id,
            user_id,
        } => {
            let msg = WsMessage {
                event: "message.typing.stop".to_string(),
                data: serde_json::json!({
                    "conversation_id": conversation_id,
                    "user_id": user_id,
                }),
            };
            let peers =
                conversation_peers_excluding(pool, *conversation_id, *user_id).await;
            hub.send_to_users(&peers, msg);
        }

        DomainEvent::NotificationCreated {
            recipient_id,
            notification_type,
            sender_id,
        } => {
            let msg = WsMessage {
                event: "notification.new".to_string(),
                data: serde_json::json!({
                    "type": notification_type,
                    "sender_id": sender_id,
                }),
            };
            hub.send_to_user(*recipient_id, msg);
        }

        DomainEvent::CallStarted {
            caller_id,
            callee_id,
            call_id,
            call_type,
        } => {
            let enrich = sqlx::query_as::<_, (String, String, String, String)>(
                r#"
                SELECT c.room_name, u.first_name, u.last_name, u.avatar
                FROM calls c
                JOIN users u ON u.id = c.caller_id
                WHERE c.id = $1
                "#,
            )
            .bind(call_id)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten();
            let (room_name, first_name, last_name, avatar) = enrich.unwrap_or_else(|| {
                (
                    format!("call-{call_id}"),
                    String::new(),
                    String::new(),
                    String::new(),
                )
            });
            let msg = WsMessage {
                event: "call.incoming".to_string(),
                data: serde_json::json!({
                    "call_id": call_id,
                    "caller_id": caller_id,
                    "callee_id": callee_id,
                    "call_type": call_type,
                    "room_name": room_name,
                    "caller": {
                        "first_name": first_name,
                        "last_name": last_name,
                        "avatar": avatar,
                    },
                }),
            };
            hub.send_to_user(*callee_id, msg.clone());
            hub.send_to_user(*caller_id, msg);
        }

        DomainEvent::CallAnswered { call_id } => {
            let msg = WsMessage {
                event: "call.answered".to_string(),
                data: serde_json::json!({ "call_id": call_id }),
            };
            if let Some((caller_id, callee_id)) = load_call_participants(pool, *call_id).await {
                hub.send_to_users(&[caller_id, callee_id], msg);
            }
        }

        DomainEvent::CallEnded { call_id } => {
            let msg = WsMessage {
                event: "call.ended".to_string(),
                data: serde_json::json!({ "call_id": call_id }),
            };
            if let Some((caller_id, callee_id)) = load_call_participants(pool, *call_id).await {
                hub.send_to_users(&[caller_id, callee_id], msg);
            }
        }

        DomainEvent::PresenceOnline { user_id } => {
            let msg = WsMessage {
                event: "user.presence".to_string(),
                data: serde_json::json!({ "user_id": user_id, "status": "online" }),
            };
            let followers = load_followers(pool, *user_id).await;
            hub.send_to_users(&followers, msg.clone());
            hub.send_to_user(*user_id, msg);
        }

        DomainEvent::PresenceOffline { user_id } => {
            let msg = WsMessage {
                event: "user.presence".to_string(),
                data: serde_json::json!({ "user_id": user_id, "status": "offline" }),
            };
            let followers = load_followers(pool, *user_id).await;
            hub.send_to_users(&followers, msg.clone());
            hub.send_to_user(*user_id, msg);
        }

        DomainEvent::AvatarChanged { user_id, url } => {
            let msg = WsMessage {
                event: "user.avatar_changed".to_string(),
                data: serde_json::json!({ "user_id": user_id, "url": url }),
            };
            let followers = load_followers(pool, *user_id).await;
            hub.send_to_users(&followers, msg.clone());
            hub.send_to_user(*user_id, msg);
        }

        DomainEvent::NameChanged {
            user_id,
            first_name,
            last_name,
        } => {
            let msg = WsMessage {
                event: "user.name_changed".to_string(),
                data: serde_json::json!({
                    "user_id": user_id,
                    "first_name": first_name,
                    "last_name": last_name,
                }),
            };
            let followers = load_followers(pool, *user_id).await;
            hub.send_to_users(&followers, msg.clone());
            hub.send_to_user(*user_id, msg);
        }

        DomainEvent::FollowRequestCreated {
            recipient_id,
            requester_id,
        } => {
            let msg = WsMessage {
                event: "follow.requested".to_string(),
                data: serde_json::json!({
                    "recipient_id": recipient_id,
                    "requester_id": requester_id,
                }),
            };
            hub.send_to_user(*recipient_id, msg);
        }

        DomainEvent::FollowRequestRemoved {
            recipient_id,
            requester_id,
        } => {
            let msg = WsMessage {
                event: "follow.request_cancelled".to_string(),
                data: serde_json::json!({
                    "recipient_id": recipient_id,
                    "requester_id": requester_id,
                }),
            };
            hub.send_to_user(*recipient_id, msg);
        }

        DomainEvent::UnreadCountChanged {
            user_id,
            messages,
            notifications,
        } => {
            let msg = WsMessage {
                event: "notification.counter".to_string(),
                data: serde_json::json!({
                    "messages": messages,
                    "notifications": notifications,
                }),
            };
            hub.send_to_user(*user_id, msg);
        }

        DomainEvent::ChatColorChanged {
            conversation_id,
            user_id,
            color,
        } => {
            let msg = WsMessage {
                event: "conversation.color_changed".to_string(),
                data: serde_json::json!({
                    "conversation_id": conversation_id,
                    "user_id": user_id,
                    "color": color,
                }),
            };
            // Fan-out to all participants of the conversation.
            let participants = match sqlx::query_scalar::<_, i64>(
                "SELECT user_id FROM conversation_members WHERE conversation_id = $1",
            )
            .bind(conversation_id)
            .fetch_all(pool)
            .await
            {
                Ok(rows) => rows,
                Err(_) => vec![*user_id],
            };
            hub.send_to_users(&participants, msg);
        }

        DomainEvent::ReactionRegistered {
            post_id,
            user_id,
            reaction,
        } => {
            let msg = WsMessage {
                event: "post.reaction.added".to_string(),
                data: serde_json::json!({
                    "post_id": post_id,
                    "user_id": user_id,
                    "reaction": reaction,
                }),
            };
            // Notify the post author so the UI can refresh its counters/badge.
            if let Ok(Some(author_id)) =
                sqlx::query_scalar::<_, i64>("SELECT user_id FROM posts WHERE id = $1")
                    .bind(post_id)
                    .fetch_optional(pool)
                    .await
            {
                hub.send_to_user(author_id, msg.clone());
            }
            hub.send_to_user(*user_id, msg);
        }

        DomainEvent::NewPostsAvailable { feed_scope, count } => {
            let msg = WsMessage {
                event: "post.new".to_string(),
                data: serde_json::json!({
                    "feed_scope": feed_scope,
                    "count": count,
                }),
            };
            // Use topic-based fanout so only clients subscribed to this feed
            // scope (e.g. "feed:home", "feed:explore") receive the notification.
            hub.send_to_topic(&format!("feed:{}", feed_scope), msg);
        }

        DomainEvent::CommentReplyCreated {
            parent_comment_id,
            comment_id,
            post_id,
        } => {
            let msg = WsMessage {
                event: "comment.new".to_string(),
                data: serde_json::json!({
                    "parent_comment_id": parent_comment_id,
                    "comment_id": comment_id,
                    "post_id": post_id,
                }),
            };
            if let Ok(Some(parent_author)) =
                sqlx::query_scalar::<_, i64>("SELECT user_id FROM comments WHERE id = $1")
                    .bind(parent_comment_id)
                    .fetch_optional(pool)
                    .await
            {
                hub.send_to_user(parent_author, msg);
            }
        }

        DomainEvent::LiveStreamStarted { stream_id, user_id } => {
            let msg = WsMessage {
                event: "live.started".to_string(),
                data: serde_json::json!({
                    "stream_id": stream_id,
                    "user_id": user_id,
                }),
            };
            let followers = load_followers(pool, *user_id).await;
            hub.send_to_users(&followers, msg.clone());
            hub.send_to_user(*user_id, msg);
        }

        DomainEvent::LiveStreamEnded { stream_id, user_id } => {
            let msg = WsMessage {
                event: "live.ended".to_string(),
                data: serde_json::json!({
                    "stream_id": stream_id,
                    "user_id": user_id,
                }),
            };
            let followers = load_followers(pool, *user_id).await;
            hub.send_to_users(&followers, msg.clone());
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
        | DomainEvent::UserMentionedInPost { .. }
        | DomainEvent::GroupJoined { .. }
        | DomainEvent::GroupLeft { .. }
        | DomainEvent::PageLiked { .. }
        | DomainEvent::StoryCreated { .. }
        | DomainEvent::PaymentCompleted { .. }
        | DomainEvent::NewsletterQueued { .. }
        | DomainEvent::JobApplicationSubmitted { .. }
        | DomainEvent::ApplicationStatusChanged { .. }
        | DomainEvent::OrderCreated { .. }
        | DomainEvent::OrderStatusChanged { .. }
        | DomainEvent::FundingDonation { .. }
        | DomainEvent::FundingGoalReached { .. }
        | DomainEvent::ProductReviewCreated { .. } => {}

        DomainEvent::AdminNotice { text, target: _ } => {
            let msg = WsMessage {
                event: "admin.notice".to_string(),
                data: serde_json::json!({ "text": text }),
            };
            hub.broadcast(msg);
        }
    }
}
