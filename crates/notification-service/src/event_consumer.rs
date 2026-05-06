use crate::dispatch::{NotificationDispatcher, NotificationPayload};
use shared::events::{DomainEvent, EventBus};
use sqlx::PgPool;
use std::sync::Arc;
use tracing::{error, info, warn};

/// Spawns a background task that subscribes to NATS domain events
/// and auto-creates notifications via the multi-channel dispatcher.
pub async fn spawn_event_consumer(event_bus: Arc<dyn EventBus>, db: PgPool) {
    let dispatcher = NotificationDispatcher::with_event_bus(db, Some(event_bus.clone()));

    // Subscribe to all domain events via wildcard
    let mut subscription = match event_bus.subscribe("events.>").await {
        Ok(sub) => {
            info!("notification-service: subscribed to NATS events.>");
            sub
        }
        Err(e) => {
            warn!(
                "notification-service: failed to subscribe to NATS events: {e}. Notifications will only be created via direct API calls."
            );
            return;
        }
    };

    loop {
        match subscription.next().await {
            Some((_subject, event)) => {
                match &event {
                    DomainEvent::MessageSent {
                        conversation_id,
                        sender_id,
                        recipient_ids,
                    } => {
                        dispatcher
                            .dispatch_new_chat_messages(
                                *conversation_id,
                                *sender_id,
                                recipient_ids.as_slice(),
                            )
                            .await;
                    }
                    _ => {
                        if let Some(payload) = event_to_notification(&event)
                            && let Err(e) = dispatcher.dispatch(payload).await
                        {
                            error!(error = %e, "Failed to dispatch notification");
                        }
                    }
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
        DomainEvent::FollowCreated {
            follower_id,
            following_id,
        } => Some(NotificationPayload {
            recipient_id: *following_id,
            sender_id: Some(*follower_id),
            notification_type: "following".to_string(),
            target_type: Some("user".to_string()),
            target_id: Some(*follower_id),
            text: "started following you".to_string(),
        }),

        DomainEvent::PostLiked {
            user_id,
            author_id,
            post_id,
            reaction_type,
        } => {
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

        DomainEvent::CommentCreated {
            user_id,
            author_id,
            post_id,
            comment_id: _,
        } => {
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

        DomainEvent::UserMentionedInPost {
            post_id,
            mentioner_id,
            mentioned_user_id,
        } => {
            if mentioner_id == mentioned_user_id {
                return None;
            }
            Some(NotificationPayload {
                recipient_id: *mentioned_user_id,
                sender_id: Some(*mentioner_id),
                notification_type: "mention".to_string(),
                target_type: Some("post".to_string()),
                target_id: Some(*post_id),
                text: "mentioned you in a post".to_string(),
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

        DomainEvent::CallStarted {
            caller_id,
            callee_id,
            call_id,
            call_type,
        } => Some(NotificationPayload {
            recipient_id: *callee_id,
            sender_id: Some(*caller_id),
            notification_type: format!("{}_call", call_type),
            target_type: Some("call".to_string()),
            target_id: Some(*call_id),
            text: format!("incoming {} call", call_type),
        }),

        DomainEvent::LiveStreamStarted { stream_id, user_id } => Some(NotificationPayload {
            recipient_id: *user_id,
            sender_id: Some(*user_id),
            notification_type: "live_stream".to_string(),
            target_type: Some("live".to_string()),
            target_id: Some(*stream_id),
            text: "started a live stream".to_string(),
        }),

        DomainEvent::PaymentCompleted {
            user_id,
            transaction_id,
            amount,
            tx_type,
        } => Some(NotificationPayload {
            recipient_id: *user_id,
            sender_id: None,
            notification_type: format!("payment_{}", tx_type),
            target_type: Some("payment".to_string()),
            target_id: Some(*transaction_id),
            text: format!("Payment of {} completed", amount),
        }),

        DomainEvent::JobApplicationSubmitted {
            job_id,
            applicant_id,
            employer_id,
        } => Some(NotificationPayload {
            recipient_id: *employer_id,
            sender_id: Some(*applicant_id),
            notification_type: "job_application".to_string(),
            target_type: Some("job".to_string()),
            target_id: Some(*job_id),
            text: "applied to your job".to_string(),
        }),

        DomainEvent::ApplicationStatusChanged {
            application_id: _,
            job_id,
            applicant_id,
            new_status,
        } => Some(NotificationPayload {
            recipient_id: *applicant_id,
            sender_id: None,
            notification_type: "application_status".to_string(),
            target_type: Some("job".to_string()),
            target_id: Some(*job_id),
            text: format!("Your application status changed to {}", new_status),
        }),

        DomainEvent::OrderCreated {
            order_id,
            buyer_id,
            seller_id,
        } => Some(NotificationPayload {
            recipient_id: *seller_id,
            sender_id: Some(*buyer_id),
            notification_type: "new_order".to_string(),
            target_type: Some("order".to_string()),
            target_id: Some(*order_id),
            text: "placed a new order".to_string(),
        }),

        DomainEvent::OrderStatusChanged {
            order_id,
            buyer_id,
            seller_id: _,
            new_status,
        } => Some(NotificationPayload {
            recipient_id: *buyer_id,
            sender_id: None,
            notification_type: "order_status".to_string(),
            target_type: Some("order".to_string()),
            target_id: Some(*order_id),
            text: format!("Your order status is now {}", new_status),
        }),

        DomainEvent::FundingDonation {
            funding_id,
            donor_id,
            creator_id,
            amount,
        } => Some(NotificationPayload {
            recipient_id: *creator_id,
            sender_id: Some(*donor_id),
            notification_type: "funding_donation".to_string(),
            target_type: Some("funding".to_string()),
            target_id: Some(*funding_id),
            text: format!("donated {} to your campaign", amount),
        }),

        DomainEvent::FundingGoalReached {
            funding_id,
            creator_id,
            goal_amount,
        } => Some(NotificationPayload {
            recipient_id: *creator_id,
            sender_id: None,
            notification_type: "funding_goal_reached".to_string(),
            target_type: Some("funding".to_string()),
            target_id: Some(*funding_id),
            text: format!("Your funding goal of {} has been reached!", goal_amount),
        }),

        DomainEvent::ProductReviewCreated {
            product_id,
            reviewer_id,
            seller_id,
        } => Some(NotificationPayload {
            recipient_id: *seller_id,
            sender_id: Some(*reviewer_id),
            notification_type: "product_review".to_string(),
            target_type: Some("product".to_string()),
            target_id: Some(*product_id),
            text: "reviewed your product".to_string(),
        }),

        // Events that don't generate user-facing persisted notifications
        // (most are realtime-only signals, handled by realtime-service directly).
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
        | DomainEvent::AvatarChanged { .. }
        | DomainEvent::NameChanged { .. }
        | DomainEvent::FollowRequestCreated { .. }
        | DomainEvent::FollowRequestRemoved { .. }
        | DomainEvent::UnreadCountChanged { .. }
        | DomainEvent::ChatColorChanged { .. }
        | DomainEvent::ReactionRegistered { .. }
        | DomainEvent::NewPostsAvailable { .. }
        | DomainEvent::CommentReplyCreated { .. }
        | DomainEvent::AdminNotice { .. }
        | DomainEvent::NewsletterQueued { .. } => None,
    }
}
