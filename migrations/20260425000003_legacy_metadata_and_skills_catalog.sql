-- M11-DATA-2 / M11-DATA-3
-- Adds `legacy_metadata` JSONB columns to the consolidated tables so the
-- php-migrator can preserve fields that were dropped or renamed during the
-- WoWonder → Jungle schema migration without losing data.
--
-- Adds `skills_catalog` so the user-skills feature can present a curated
-- list instead of free-form strings (the existing `user_skills.skill`
-- column is preserved for backwards compatibility).

ALTER TABLE users
    ADD COLUMN IF NOT EXISTS legacy_metadata JSONB NOT NULL DEFAULT '{}'::jsonb;

ALTER TABLE posts
    ADD COLUMN IF NOT EXISTS legacy_metadata JSONB NOT NULL DEFAULT '{}'::jsonb;

CREATE INDEX IF NOT EXISTS idx_users_legacy_id
    ON users ((legacy_metadata->>'wowonder_user_id'));

CREATE INDEX IF NOT EXISTS idx_posts_legacy_id
    ON posts ((legacy_metadata->>'wowonder_post_id'));

CREATE TABLE IF NOT EXISTS skills_catalog (
    id          BIGSERIAL PRIMARY KEY,
    name        TEXT        NOT NULL UNIQUE,
    category    TEXT        NOT NULL DEFAULT 'General',
    is_active   BOOLEAN     NOT NULL DEFAULT TRUE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_skills_catalog_category
    ON skills_catalog (category)
    WHERE is_active;
