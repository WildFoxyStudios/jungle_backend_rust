-- ═══════════════════════════════════════════════════════════════════
-- Story Highlights (Instagram-style permanent story collections)
-- ═══════════════════════════════════════════════════════════════════
--
-- Matches PHP `api/highlight` endpoint with subtypes:
--   create, delete, add_story, get_highlight, get_highlight_stories
--
-- A user bundles expired/expiring story_media items into named, cover-image
-- "Highlights" that remain visible on their profile forever.

CREATE TABLE IF NOT EXISTS story_highlights (
    id          BIGSERIAL PRIMARY KEY,
    user_id     BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title       VARCHAR(60) NOT NULL,
    cover_url   TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_story_highlights_user
    ON story_highlights (user_id, created_at DESC);

CREATE TABLE IF NOT EXISTS story_highlight_items (
    id              BIGSERIAL PRIMARY KEY,
    highlight_id    BIGINT NOT NULL REFERENCES story_highlights(id) ON DELETE CASCADE,
    story_media_id  BIGINT NOT NULL REFERENCES story_media(id) ON DELETE CASCADE,
    added_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    sort_order      INTEGER NOT NULL DEFAULT 0,
    UNIQUE (highlight_id, story_media_id)
);

CREATE INDEX IF NOT EXISTS idx_story_highlight_items_highlight
    ON story_highlight_items (highlight_id, sort_order, added_at DESC);
