use async_trait::async_trait;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tracing::{error, info};

/// All domain events that flow through the event bus.
/// Services publish these; notification-service, realtime-service, and others subscribe.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", content = "data")]
pub enum DomainEvent {
    // ── User ──
    UserCreated {
        user_id: i64,
        username: String,
    },
    UserUpdated {
        user_id: i64,
        fields: Vec<String>,
    },
    UserDeleted {
        user_id: i64,
    },

    // ── Social ──
    FollowCreated {
        follower_id: i64,
        following_id: i64,
    },
    FollowDeleted {
        follower_id: i64,
        following_id: i64,
    },
    UserBlocked {
        blocker_id: i64,
        blocked_id: i64,
    },

    // ── Posts ──
    PostCreated {
        post_id: i64,
        user_id: i64,
        group_id: Option<i64>,
        page_id: Option<i64>,
    },
    PostDeleted {
        post_id: i64,
    },
    PostLiked {
        post_id: i64,
        user_id: i64,
        author_id: i64,
        reaction_type: String,
    },

    // ── Comments ──
    CommentCreated {
        comment_id: i64,
        post_id: i64,
        user_id: i64,
        author_id: i64,
    },
    /// `@username` in post/reel caption body (resolved to user ids server-side).
    UserMentionedInPost {
        post_id: i64,
        mentioner_id: i64,
        mentioned_user_id: i64,
    },

    // ── Messaging ──
    MessageSent {
        conversation_id: i64,
        sender_id: i64,
        recipient_ids: Vec<i64>,
    },
    MessageRead {
        conversation_id: i64,
        user_id: i64,
    },
    TypingStarted {
        conversation_id: i64,
        user_id: i64,
    },
    TypingStopped {
        conversation_id: i64,
        user_id: i64,
    },

    // ── Groups / Pages ──
    GroupJoined {
        group_id: i64,
        user_id: i64,
    },
    GroupLeft {
        group_id: i64,
        user_id: i64,
    },
    PageLiked {
        page_id: i64,
        user_id: i64,
    },

    // ── Stories ──
    StoryCreated {
        story_id: i64,
        user_id: i64,
    },

    // ── Calls ──
    CallStarted {
        call_id: i64,
        caller_id: i64,
        callee_id: i64,
        call_type: String,
    },
    CallAnswered {
        call_id: i64,
    },
    CallEnded {
        call_id: i64,
    },

    // ── Payments ──
    PaymentCompleted {
        transaction_id: i64,
        user_id: i64,
        amount: String,
        tx_type: String,
    },

    // ── Live ──
    LiveStreamStarted {
        stream_id: i64,
        user_id: i64,
    },
    LiveStreamEnded {
        stream_id: i64,
        user_id: i64,
    },

    // ── Notifications ──
    NotificationCreated {
        recipient_id: i64,
        notification_type: String,
        sender_id: Option<i64>,
    },

    // ── Presence ──
    PresenceOnline {
        user_id: i64,
    },
    PresenceOffline {
        user_id: i64,
    },

    // ── Profile mutations (fan-out to followers/self sessions) ──
    AvatarChanged {
        user_id: i64,
        url: String,
    },
    NameChanged {
        user_id: i64,
        first_name: String,
        last_name: String,
    },

    // ── Social (follow requests lifecycle, distinct from FollowCreated) ──
    FollowRequestCreated {
        recipient_id: i64,
        requester_id: i64,
    },
    FollowRequestRemoved {
        recipient_id: i64,
        requester_id: i64,
    },

    // ── Counters ──
    UnreadCountChanged {
        user_id: i64,
        messages: i32,
        notifications: i32,
    },

    // ── Chat customisation ──
    ChatColorChanged {
        conversation_id: i64,
        user_id: i64,
        color: String,
    },

    // ── Feed domain granular ──
    ReactionRegistered {
        post_id: i64,
        user_id: i64,
        reaction: String,
    },
    NewPostsAvailable {
        feed_scope: String,
        count: i32,
    },
    CommentReplyCreated {
        parent_comment_id: i64,
        comment_id: i64,
        post_id: i64,
    },

    // ── Admin ──
    AdminNotice {
        text: String,
        target: String,
    },
    NewsletterQueued {
        subject: String,
        recipient_count: i64,
    },

    // ── Commerce ──
    JobApplicationSubmitted {
        job_id: i64,
        applicant_id: i64,
        employer_id: i64,
    },
    ApplicationStatusChanged {
        application_id: i64,
        job_id: i64,
        applicant_id: i64,
        new_status: String,
    },
    OrderCreated {
        order_id: i64,
        buyer_id: i64,
        seller_id: i64,
    },
    OrderStatusChanged {
        order_id: i64,
        buyer_id: i64,
        seller_id: i64,
        new_status: String,
    },
    FundingDonation {
        funding_id: i64,
        donor_id: i64,
        creator_id: i64,
        amount: String,
    },
    FundingGoalReached {
        funding_id: i64,
        creator_id: i64,
        goal_amount: String,
    },
    ProductReviewCreated {
        product_id: i64,
        reviewer_id: i64,
        seller_id: i64,
    },
}

impl DomainEvent {
    /// NATS subject for this event, e.g. `events.user.created`.
    pub fn subject(&self) -> &'static str {
        match self {
            Self::UserCreated { .. } => "events.user.created",
            Self::UserUpdated { .. } => "events.user.updated",
            Self::UserDeleted { .. } => "events.user.deleted",
            Self::FollowCreated { .. } => "events.follow.created",
            Self::FollowDeleted { .. } => "events.follow.deleted",
            Self::UserBlocked { .. } => "events.user.blocked",
            Self::PostCreated { .. } => "events.post.created",
            Self::PostDeleted { .. } => "events.post.deleted",
            Self::PostLiked { .. } => "events.post.liked",
            Self::CommentCreated { .. } => "events.post.commented",
            Self::UserMentionedInPost { .. } => "events.post.mention",
            Self::MessageSent { .. } => "events.message.sent",
            Self::MessageRead { .. } => "events.message.read",
            Self::TypingStarted { .. } => "events.typing.start",
            Self::TypingStopped { .. } => "events.typing.stop",
            Self::GroupJoined { .. } => "events.group.joined",
            Self::GroupLeft { .. } => "events.group.left",
            Self::PageLiked { .. } => "events.page.liked",
            Self::StoryCreated { .. } => "events.story.created",
            Self::CallStarted { .. } => "events.call.started",
            Self::CallAnswered { .. } => "events.call.answered",
            Self::CallEnded { .. } => "events.call.ended",
            Self::PaymentCompleted { .. } => "events.payment.completed",
            Self::LiveStreamStarted { .. } => "events.live.started",
            Self::LiveStreamEnded { .. } => "events.live.ended",
            Self::NotificationCreated { .. } => "events.notification.created",
            Self::PresenceOnline { .. } => "events.presence.online",
            Self::PresenceOffline { .. } => "events.presence.offline",
            Self::AvatarChanged { .. } => "events.user.avatar_changed",
            Self::NameChanged { .. } => "events.user.name_changed",
            Self::FollowRequestCreated { .. } => "events.follow.request_created",
            Self::FollowRequestRemoved { .. } => "events.follow.request_removed",
            Self::UnreadCountChanged { .. } => "events.user.unread_changed",
            Self::ChatColorChanged { .. } => "events.conversation.color_changed",
            Self::ReactionRegistered { .. } => "events.post.reaction_registered",
            Self::NewPostsAvailable { .. } => "events.feed.new_posts",
            Self::CommentReplyCreated { .. } => "events.comment.reply_created",
            Self::AdminNotice { .. } => "events.admin.notice",
            Self::NewsletterQueued { .. } => "events.admin.newsletter",
            Self::JobApplicationSubmitted { .. } => "events.commerce.job_application",
            Self::ApplicationStatusChanged { .. } => "events.commerce.application_status",
            Self::OrderCreated { .. } => "events.commerce.order_created",
            Self::OrderStatusChanged { .. } => "events.commerce.order_status",
            Self::FundingDonation { .. } => "events.commerce.funding_donation",
            Self::FundingGoalReached { .. } => "events.commerce.funding_goal_reached",
            Self::ProductReviewCreated { .. } => "events.commerce.product_review",
        }
    }
}

/// Abstraction over the event bus so we can swap implementations or use a no-op in tests.
#[async_trait]
pub trait EventBus: Send + Sync {
    async fn publish(&self, event: &DomainEvent) -> Result<(), EventBusError>;
    async fn subscribe(&self, subject: &str) -> Result<EventSubscription, EventBusError>;

    /// Publish an already-serialized payload on a raw subject. Used by the DLQ
    /// admin retry endpoint to resurrect a failed event on its original subject
    /// without re-parsing it into a `DomainEvent` (and thus tolerating schema
    /// drift between producers and consumers).
    async fn publish_raw(&self, subject: &str, payload: &[u8]) -> Result<(), EventBusError>;
}

#[derive(Debug, thiserror::Error)]
pub enum EventBusError {
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Connection error: {0}")]
    Connection(String),
    #[error("Publish error: {0}")]
    Publish(String),
    #[error("Subscribe error: {0}")]
    Subscribe(String),
}

/// Wraps an async-nats subscriber for consuming events.
pub struct EventSubscription {
    inner: async_nats::Subscriber,
}

impl EventSubscription {
    /// Receive the next event. Returns `None` if the subscription is closed.
    pub async fn next(&mut self) -> Option<(String, DomainEvent)> {
        let msg = self.inner.next().await?;
        let subject = msg.subject.to_string();
        match serde_json::from_slice::<DomainEvent>(&msg.payload) {
            Ok(event) => Some((subject, event)),
            Err(e) => {
                error!(subject, error = %e, "Failed to deserialize event");
                None
            }
        }
    }
}

/// NATS-backed event bus implementation.
#[derive(Clone)]
pub struct NatsEventBus {
    client: async_nats::Client,
}

impl NatsEventBus {
    /// Connect to NATS using the provided URL.
    pub async fn connect(nats_url: &str) -> Result<Self, EventBusError> {
        let client = async_nats::connect(nats_url)
            .await
            .map_err(|e| EventBusError::Connection(e.to_string()))?;
        info!(url = nats_url, "Connected to NATS");
        Ok(Self { client })
    }

    pub fn client(&self) -> &async_nats::Client {
        &self.client
    }
}

#[async_trait]
impl EventBus for NatsEventBus {
    async fn publish(&self, event: &DomainEvent) -> Result<(), EventBusError> {
        let payload =
            serde_json::to_vec(event).map_err(|e| EventBusError::Serialization(e.to_string()))?;
        let subject = event.subject();

        // Retry up to 3 times with exponential backoff
        for attempt in 0u32..3 {
            match self.client.publish(subject, payload.clone().into()).await {
                Ok(_) => return Ok(()),
                Err(e) if attempt < 2 => {
                    let delay = std::time::Duration::from_millis(100 * 2u64.pow(attempt));
                    tracing::warn!(subject, attempt, error = %e, "Retrying publish after {:?}", delay);
                    tokio::time::sleep(delay).await;
                }
                Err(e) => {
                    // Dead letter queue
                    let dlq_subject: String = format!("dlq.{}", subject);
                    let _ = self.client.publish(dlq_subject, payload.into()).await;
                    error!(subject, "Message sent to DLQ after 3 failures");
                    return Err(EventBusError::Publish(e.to_string()));
                }
            }
        }
        Ok(())
    }

    async fn subscribe(&self, subject: &str) -> Result<EventSubscription, EventBusError> {
        let owned_subject: String = subject.to_owned();
        let subscriber = self
            .client
            .subscribe(owned_subject)
            .await
            .map_err(|e| EventBusError::Subscribe(e.to_string()))?;
        Ok(EventSubscription { inner: subscriber })
    }

    async fn publish_raw(&self, subject: &str, payload: &[u8]) -> Result<(), EventBusError> {
        self.client
            .publish(subject.to_owned(), payload.to_vec().into())
            .await
            .map_err(|e| EventBusError::Publish(e.to_string()))
    }
}

/// Connect to NATS (or return a `NoopEventBus` fallback). Crashes when
/// `NATS_REQUIRED=true` and the connection fails — use this in production
/// to prevent silent event loss.
pub async fn connect_event_bus(nats_url: &str) -> std::sync::Arc<dyn EventBus> {
    match NatsEventBus::connect(nats_url).await {
        Ok(bus) => std::sync::Arc::new(bus),
        Err(e) => {
            let required = std::env::var("NATS_REQUIRED")
                .unwrap_or_else(|_| "true".into())
                .eq_ignore_ascii_case("true");
            if required {
                tracing::error!(
                    error = %e,
                    "NATS is REQUIRED but unavailable — crashing to prevent silent event loss"
                );
                std::process::exit(1);
            }
            tracing::error!(
                error = %e,
                "NATS unavailable — using NoopEventBus (inter-service events will NOT be delivered)"
            );
            std::sync::Arc::new(NoopEventBus)
        }
    }
}

/// No-op event bus for tests or when NATS is not configured.
#[derive(Clone)]
pub struct NoopEventBus;

#[async_trait]
impl EventBus for NoopEventBus {
    async fn publish(&self, _event: &DomainEvent) -> Result<(), EventBusError> {
        Ok(())
    }
    async fn subscribe(&self, _subject: &str) -> Result<EventSubscription, EventBusError> {
        Err(EventBusError::Connection(
            "NoopEventBus does not support subscriptions".into(),
        ))
    }
    async fn publish_raw(&self, _subject: &str, _payload: &[u8]) -> Result<(), EventBusError> {
        Ok(())
    }
}
