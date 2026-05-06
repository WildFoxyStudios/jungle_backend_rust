-- Phase 8 Recommendations: PYMK, Pages, Groups

CREATE TABLE IF NOT EXISTS recommendation_snapshots (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK (kind IN ('pymk', 'pages', 'groups', 'events')),
    target_id BIGINT NOT NULL,
    score REAL NOT NULL DEFAULT 0,
    reason TEXT,
    generated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    dismissed BOOLEAN NOT NULL DEFAULT FALSE,
    UNIQUE (user_id, kind, target_id)
);

CREATE INDEX IF NOT EXISTS idx_recs_user_kind ON recommendation_snapshots(user_id, kind, score DESC);
CREATE INDEX IF NOT EXISTS idx_recs_generated ON recommendation_snapshots(generated_at);
