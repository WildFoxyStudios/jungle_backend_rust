-- Migration 006: Notifications

CREATE TABLE notifications (
    id              BIGSERIAL PRIMARY KEY,
    recipient_id    BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    sender_id       BIGINT REFERENCES users(id) ON DELETE SET NULL,
    type            VARCHAR(50) NOT NULL,       -- following, liked_post, comment, reaction, message, etc.
    target_type     VARCHAR(30),                -- post, comment, page, group, story, event, etc.
    target_id       BIGINT,
    text            TEXT DEFAULT '',
    url             TEXT DEFAULT '',
    is_read         BOOLEAN NOT NULL DEFAULT FALSE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_notifications_recipient ON notifications(recipient_id, created_at DESC);
CREATE INDEX idx_notifications_unread ON notifications(recipient_id, is_read) WHERE is_read = FALSE;
CREATE INDEX idx_notifications_type ON notifications(recipient_id, type);

-- Notification preferences per user (stored in users.notification_settings JSONB)
-- No separate table needed — the preferences are embedded in the users table.
-- Example JSONB: {"email_on_follow": true, "push_on_message": true, "email_on_comment": false, ...}
