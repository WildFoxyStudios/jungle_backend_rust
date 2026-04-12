use crate::dispatch::{NotificationDispatcher, NotificationPayload};
use shared::events::{DomainEvent, EventBus};
use sqlx::PgPool;
use std::sync::Arc;
use tracing::{error, info, warn};

/// Spawns a background task that subscribes to NATS domain events
/// and auto-creates notifications via the multi-channel dispatcher.
pub async fn spawn_event_consumer(
    event_bus: Arc<dyn EventBus>,
    db: PgPool,
) {
    let dispatcher = NotificationDispatcher::new(db);

    // Subscribe to all domain events via wildcard
    let mut subscription = match event_bus.subscribe("events.>").await {
        Ok(sub) => {
            info!("notification-service: subscribed to NATS events.>");
            sub
        }
        Err(e) => {
            warn!("notification-service: failed to subscribe to NATS events: {e}. Notifications will only be created via direct API calls.");
            return;
        }
    };

    loop {
        match subscription.next().await {
            Some((_subject, event)) => {
                if let Some(payload) = event_to_notification(&event)
                    && let Err(e) = dispatcher.dispatch(payload).await {
                        error!(error = %e, "Failed to dispatch notification");
                    }
            }
            None => {
                warn!("notification-service: NATS subscription closed, exiting event consumer");
                break;
            }
        }
    }
}

/// Maps a DomainEvent to a NotificationPayload if it should generate a notification.
fn event_to_notification(event: &DomainEvent) -> Option<NotificationPayload> {
    match event {
        DomainEvent::FollowCreated { follower_id, following_id } => Some(NotificationPayload {
            recipient_id: *following_id,
            sender_id: Some(*follower_id),
            notification_type: "following".to_string(),
            target_type: Some("user".to_string()),
            target_id: Some(*follower_id),
            text: "started following you".to_string(),
        }),

        DomainEvent::PostLiked { user_id, author_id, post_id, reaction_type } => {
            if user_id == author_id {
                return None;
            }
            Some(NotificationPayload {
                recipient_id: *author_id,
                sender_id: Some(*user_id),
                notification_type: format!("reaction_{}", reaction_type),
                target_type: Some("post".to_string()),
                target_id: Some(*post_id),
                text: format!("reacted to your post with {}", reaction_type),
            })
        }

        DomainEvent::CommentCreated { user_id, author_id, post_id, comment_id: _ } => {
            if user_id == author_id {
                return None;
            }
            Some(NotificationPayload {
                recipient_id: *author_id,
                sender_id: Some(*user_id),
                notification_type: "comment".to_string(),
                target_type: Some("post".to_string()),
                target_id: Some(*post_id),
                text: "commented on your post".to_string(),
            })
        }

        DomainEvent::GroupJoined { group_id, user_id } => Some(NotificationPayload {
            recipient_id: *user_id,
            sender_id: None,
            notification_type: "group_joined".to_string(),
            target_type: Some("group".to_string()),
            target_id: Some(*group_id),
            text: "joined the group".to_string(),
        }),

        DomainEvent::PageLiked { page_id, user_id } => Some(NotificationPayload {
            recipient_id: *user_id,
            sender_id: None,
            notification_type: "page_liked".to_string(),
            target_type: Some("page".to_string()),
            target_id: Some(*page_id),
            text: "liked the page".to_string(),
        }),

        DomainEvent::CallStarted { caller_id, callee_id, call_id, call_type } => {
            Some(NotificationPayload {
                recipient_id: *callee_id,
                sender_id: Some(*caller_id),
                notification_type: format!("{}_call", call_type),
                target_type: Some("call".to_string()),
                target_id: Some(*call_id),
                text: format!("incoming {} call", call_type),
            })
        }

        DomainEvent::LiveStreamStarted { stream_id, user_id } => Some(NotificationPayload {
            recipient_id: *user_id,
            sender_id: Some(*user_id),
            notification_type: "live_stream".to_string(),
            target_type: Some("live".to_string()),
            target_id: Some(*stream_id),
            text: "started a live stream".to_string(),
        }),

        DomainEvent::PaymentCompleted { user_id, transaction_id, amount, tx_type } => {
            Some(NotificationPayload {
                recipient_id: *user_id,
                sender_id: None,
                notification_type: format!("payment_{}", tx_type),
                target_type: Some("payment".to_string()),
                target_id: Some(*transaction_id),
                text: format!("Payment of {} completed", amount),
            })
        }

        // Events that don't generate user-facing notifications
        DomainEvent::UserCreated { .. }
        | DomainEvent::UserUpdated { .. }
        | DomainEvent::UserDeleted { .. }
        | DomainEvent::FollowDeleted { .. }
        | DomainEvent::UserBlocked { .. }
        | DomainEvent::PostCreated { .. }
        | DomainEvent::PostDeleted { .. }
        | DomainEvent::MessageSent { .. }
        | DomainEvent::MessageRead { .. }
        | DomainEvent::TypingStarted { .. }
        | DomainEvent::TypingStopped { .. }
        | DomainEvent::GroupLeft { .. }
        | DomainEvent::StoryCreated { .. }
        | DomainEvent::CallAnswered { .. }
        | DomainEvent::CallEnded { .. }
        | DomainEvent::LiveStreamEnded { .. }
        | DomainEvent::NotificationCreated { .. }
        | DomainEvent::PresenceOnline { .. }
        | DomainEvent::PresenceOffline { .. }
        | DomainEvent::AdminNotice { .. }
        | DomainEvent::NewsletterQueued { .. } => None,
    }
}
