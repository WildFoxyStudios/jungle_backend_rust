-- Consolidate WoWonder PHP `posts` legacy fields onto the canonical schema.
-- These columns were previously inferred from the JSON `metadata` blob;
-- promoting them to first-class columns lets handlers query them directly
-- without round-tripping through JSONB. All ADD COLUMNs are IF NOT EXISTS
-- so this migration is idempotent and safe to re-run.
ALTER TABLE posts
    ADD COLUMN IF NOT EXISTS embed_url TEXT,
    ADD COLUMN IF NOT EXISTS embed_provider TEXT,
    ADD COLUMN IF NOT EXISTS activity JSONB,
    ADD COLUMN IF NOT EXISTS source TEXT NOT NULL DEFAULT 'user',
    ADD COLUMN IF NOT EXISTS live_stream_id BIGINT NULL REFERENCES live_streams(id) ON DELETE SET NULL,
    ADD COLUMN IF NOT EXISTS title TEXT NULL;

-- url_preview_id keeps a reference to the canonical preview row so we can
-- evict the cache without rewriting every post. Not added if the column
-- already exists from an earlier rollout.
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'posts' AND column_name = 'url_preview_id'
    ) THEN
        ALTER TABLE posts
            ADD COLUMN url_preview_id BIGINT NULL;
        -- url_preview_cache uses url_hash (VARCHAR) as PK so we can't
        -- create an FK directly; we just keep the slot for future
        -- migration that may switch the cache to a numeric PK.
    END IF;
END $$;

CREATE INDEX IF NOT EXISTS idx_posts_source ON posts(source) WHERE deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_posts_live_stream ON posts(live_stream_id) WHERE live_stream_id IS NOT NULL;
