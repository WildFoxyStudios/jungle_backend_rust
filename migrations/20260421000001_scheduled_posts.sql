-- Add missing columns for scheduled posts feature
ALTER TABLE posts ADD COLUMN IF NOT EXISTS scheduled_at TIMESTAMPTZ;
ALTER TABLE posts ADD COLUMN IF NOT EXISTS published_at TIMESTAMPTZ;

-- Backfill published_at for existing posts
UPDATE posts SET published_at = created_at WHERE published_at IS NULL;
