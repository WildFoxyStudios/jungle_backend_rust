-- Migration 005: Media uploads + Stories

-- Uploaded media tracking
CREATE TABLE uploaded_media (
    id              BIGSERIAL PRIMARY KEY,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    file_url        TEXT NOT NULL,
    file_type       VARCHAR(20) NOT NULL,       -- image, video, audio, file
    file_name       VARCHAR(255) DEFAULT '',
    file_size       BIGINT NOT NULL DEFAULT 0,
    mime_type       VARCHAR(100) DEFAULT '',
    width           INT,
    height          INT,
    duration        INT,                        -- seconds, for video/audio
    thumbnail_url   TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_uploaded_media_user ON uploaded_media(user_id, created_at DESC);
CREATE INDEX idx_uploaded_media_type ON uploaded_media(user_id, file_type);

-- Stories
CREATE TABLE stories (
    id              BIGSERIAL PRIMARY KEY,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at      TIMESTAMPTZ NOT NULL DEFAULT (NOW() + INTERVAL '24 hours')
);

CREATE INDEX idx_stories_user ON stories(user_id, created_at DESC);
CREATE INDEX idx_stories_active ON stories(expires_at);

CREATE TABLE story_media (
    id              BIGSERIAL PRIMARY KEY,
    story_id        BIGINT NOT NULL REFERENCES stories(id) ON DELETE CASCADE,
    media_type      VARCHAR(20) NOT NULL,       -- image, video
    media_url       TEXT NOT NULL,
    thumbnail_url   TEXT,
    description     TEXT DEFAULT '',
    duration        INT DEFAULT 5,              -- display duration in seconds
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE story_views (
    id              BIGSERIAL PRIMARY KEY,
    story_media_id  BIGINT NOT NULL REFERENCES story_media(id) ON DELETE CASCADE,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    viewed_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(story_media_id, user_id)
);

-- Story reactions (reuse reactions table: target_type='story_media')
