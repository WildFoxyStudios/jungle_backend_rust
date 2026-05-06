-- Phase 16: Live Advanced Features (pure backend, no external APIs)
-- Co-hosts, live polls

-- Live co-hosts
CREATE TABLE IF NOT EXISTS live_cohosts (
    live_id BIGINT NOT NULL,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role VARCHAR(20) NOT NULL DEFAULT 'cohost',
    invited_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    accepted_at TIMESTAMPTZ,
    PRIMARY KEY (live_id, user_id)
);

-- Live polls
CREATE TABLE IF NOT EXISTS live_polls (
    id BIGSERIAL PRIMARY KEY,
    live_id BIGINT NOT NULL,
    question TEXT NOT NULL,
    options JSONB NOT NULL DEFAULT '[]',
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_by BIGINT NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS live_poll_votes (
    poll_id BIGINT NOT NULL REFERENCES live_polls(id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    option_index INT NOT NULL,
    voted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (poll_id, user_id)
);
