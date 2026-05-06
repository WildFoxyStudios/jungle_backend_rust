-- Phase 6 Feed EdgeRank: signal capture columns

-- Extend post_viewers with dwell time and scroll depth
ALTER TABLE post_viewers ADD COLUMN IF NOT EXISTS dwell_ms INT;
ALTER TABLE post_viewers ADD COLUMN IF NOT EXISTS scroll_depth REAL;
ALTER TABLE post_viewers ADD COLUMN IF NOT EXISTS source VARCHAR(20) DEFAULT 'feed';

-- Post scores per user (precomputed by feed_ranking job)
CREATE TABLE IF NOT EXISTS post_scores_user (
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    post_id BIGINT NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    score REAL NOT NULL DEFAULT 0,
    affinity_score REAL NOT NULL DEFAULT 0,
    engagement_score REAL NOT NULL DEFAULT 0,
    recency_score REAL NOT NULL DEFAULT 0,
    content_type_boost REAL NOT NULL DEFAULT 0,
    computed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, post_id)
);

CREATE INDEX IF NOT EXISTS idx_post_scores_user_id ON post_scores_user(user_id, score DESC);

-- Feed snooze
CREATE TABLE IF NOT EXISTS feed_snoozes (
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    snoozed_user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    until_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, snoozed_user_id)
);

-- Extend hidden_posts with hide_all option
ALTER TABLE hidden_posts ADD COLUMN IF NOT EXISTS hide_all BOOLEAN NOT NULL DEFAULT FALSE;
