-- Reel audio (catalog / user upload / derived from another reel) + post metadata for remix / templates.

CREATE TABLE IF NOT EXISTS reel_audio_tracks (
    id                   BIGSERIAL PRIMARY KEY,
    title                VARCHAR(300) NOT NULL DEFAULT '',
    artist_label         VARCHAR(300) NOT NULL DEFAULT '',
    source               VARCHAR(32)  NOT NULL DEFAULT 'user_upload',
    uploaded_media_id    BIGINT REFERENCES uploaded_media(id) ON DELETE SET NULL,
    source_post_id       BIGINT REFERENCES posts(id) ON DELETE SET NULL,
    created_by_user_id   BIGINT REFERENCES users(id) ON DELETE SET NULL,
    duration_ms          INTEGER,
    use_count            BIGINT NOT NULL DEFAULT 0,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_reel_audio_tracks_use ON reel_audio_tracks(use_count DESC);
CREATE INDEX IF NOT EXISTS idx_reel_audio_tracks_created ON reel_audio_tracks(created_at DESC);

ALTER TABLE posts ADD COLUMN IF NOT EXISTS audio_track_id BIGINT REFERENCES reel_audio_tracks(id) ON DELETE SET NULL;
ALTER TABLE posts ADD COLUMN IF NOT EXISTS remix_of_post_id BIGINT REFERENCES posts(id) ON DELETE SET NULL;
ALTER TABLE posts ADD COLUMN IF NOT EXISTS template_key VARCHAR(64);
ALTER TABLE posts ADD COLUMN IF NOT EXISTS allow_remix BOOLEAN NOT NULL DEFAULT TRUE;

CREATE INDEX IF NOT EXISTS idx_posts_audio_track ON posts(audio_track_id) WHERE audio_track_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_posts_remix_of ON posts(remix_of_post_id) WHERE remix_of_post_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS reel_insight_samples (
    id         BIGSERIAL PRIMARY KEY,
    user_id    BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    post_id    BIGINT NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    bucket_sec SMALLINT NOT NULL CHECK (bucket_sec >= 0 AND bucket_sec < 120),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_reel_insight_samples_post ON reel_insight_samples(post_id, bucket_sec);
