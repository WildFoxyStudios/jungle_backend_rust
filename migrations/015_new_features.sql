-- Migration 015: New features (polls, family, skills, live, oauth, verification_requests)

-- ── Polls ────────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS polls (
    id          BIGSERIAL PRIMARY KEY,
    post_id     BIGINT NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    question    TEXT NOT NULL DEFAULT '',
    options     JSONB NOT NULL DEFAULT '[]',
    ends_at     TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(post_id)
);
CREATE INDEX IF NOT EXISTS idx_polls_post ON polls(post_id);

CREATE TABLE IF NOT EXISTS poll_votes (
    id           BIGSERIAL PRIMARY KEY,
    poll_id      BIGINT NOT NULL REFERENCES polls(id) ON DELETE CASCADE,
    user_id      BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    option_index INT NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(poll_id, user_id)
);
CREATE INDEX IF NOT EXISTS idx_poll_votes_poll ON poll_votes(poll_id);

-- ── Family Relations ─────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS family_relations (
    id            BIGSERIAL PRIMARY KEY,
    user_id       BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    relative_id   BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    relation_type VARCHAR(50) NOT NULL DEFAULT '',
    status        VARCHAR(20) NOT NULL DEFAULT 'pending',
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, relative_id)
);
CREATE INDEX IF NOT EXISTS idx_family_user ON family_relations(user_id);
CREATE INDEX IF NOT EXISTS idx_family_relative ON family_relations(relative_id);

-- ── User Skills ──────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS user_skills (
    id         BIGSERIAL PRIMARY KEY,
    user_id    BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name       VARCHAR(100) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_user_skills_user ON user_skills(user_id);
CREATE INDEX IF NOT EXISTS idx_user_skills_name ON user_skills(name);

-- ── Live Streams ─────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS live_streams (
    id           BIGSERIAL PRIMARY KEY,
    user_id      BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title        VARCHAR(255) NOT NULL DEFAULT '',
    stream_key   VARCHAR(100) NOT NULL DEFAULT '',
    status       VARCHAR(20) NOT NULL DEFAULT 'live',
    viewer_count INT NOT NULL DEFAULT 0,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ended_at     TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS idx_live_streams_status ON live_streams(status);
CREATE INDEX IF NOT EXISTS idx_live_streams_user ON live_streams(user_id);

CREATE TABLE IF NOT EXISTS live_comments (
    id         BIGSERIAL PRIMARY KEY,
    stream_id  BIGINT NOT NULL REFERENCES live_streams(id) ON DELETE CASCADE,
    user_id    BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    content    TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_live_comments_stream ON live_comments(stream_id);

-- ── Verification Requests ────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS verification_requests (
    id          BIGSERIAL PRIMARY KEY,
    user_id     BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    target_type VARCHAR(20) NOT NULL DEFAULT 'user',
    target_id   BIGINT NOT NULL DEFAULT 0,
    status      VARCHAR(20) NOT NULL DEFAULT 'pending',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, target_type, target_id)
);
CREATE INDEX IF NOT EXISTS idx_verification_requests_status ON verification_requests(status);

-- ── OAuth Developer Portal ───────────────────────────────────────────
CREATE TABLE IF NOT EXISTS oauth_apps (
    id            BIGSERIAL PRIMARY KEY,
    user_id       BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    app_name      VARCHAR(100) NOT NULL,
    client_id     UUID NOT NULL DEFAULT gen_random_uuid(),
    client_secret VARCHAR(255) NOT NULL DEFAULT '',
    redirect_uri  TEXT NOT NULL DEFAULT '',
    description   TEXT,
    permissions   JSONB NOT NULL DEFAULT '["read"]',
    is_active     BOOLEAN NOT NULL DEFAULT TRUE,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(client_id)
);
CREATE INDEX IF NOT EXISTS idx_oauth_apps_user ON oauth_apps(user_id);
CREATE INDEX IF NOT EXISTS idx_oauth_apps_client_id ON oauth_apps(client_id);

CREATE TABLE IF NOT EXISTS oauth_codes (
    id           BIGSERIAL PRIMARY KEY,
    app_id       BIGINT NOT NULL REFERENCES oauth_apps(id) ON DELETE CASCADE,
    user_id      BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    code         VARCHAR(255) NOT NULL,
    redirect_uri TEXT NOT NULL DEFAULT '',
    expires_at   TIMESTAMPTZ NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(code)
);

CREATE TABLE IF NOT EXISTS oauth_tokens (
    id           BIGSERIAL PRIMARY KEY,
    app_id       BIGINT NOT NULL REFERENCES oauth_apps(id) ON DELETE CASCADE,
    user_id      BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    access_token VARCHAR(255) NOT NULL,
    expires_at   TIMESTAMPTZ NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(access_token)
);
CREATE INDEX IF NOT EXISTS idx_oauth_tokens_token ON oauth_tokens(access_token);

-- ── Add is_boosted to pages if missing ───────────────────────────────
ALTER TABLE pages ADD COLUMN IF NOT EXISTS is_boosted BOOLEAN NOT NULL DEFAULT FALSE;
