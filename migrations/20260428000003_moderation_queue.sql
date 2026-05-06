-- Pending content (e.g. posts/reels awaiting admin approval when `post_approval` is enabled)
CREATE TABLE IF NOT EXISTS moderation_queue (
    id BIGSERIAL PRIMARY KEY,
    target_type TEXT NOT NULL,
    target_id BIGINT NOT NULL,
    submitted_by_user_id BIGINT REFERENCES users (id) ON DELETE SET NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (target_type, target_id)
);

CREATE INDEX IF NOT EXISTS idx_moderation_queue_status ON moderation_queue (status, created_at DESC);
