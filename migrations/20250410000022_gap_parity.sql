-- Migration 015: Gap Parity — remaining items from MIGRATION_PLAN_PART1-4 audit

-- ============================================================
-- 1. monetization_settings JSONB on users
--    (PHP: Wo_UserMonetization merged into users per PART4 plan)
-- ============================================================
ALTER TABLE users ADD COLUMN IF NOT EXISTS monetization_settings JSONB NOT NULL DEFAULT '{
    "enabled": false,
    "minimum_subscription_price": 0,
    "wallet_currency": "USD"
}'::jsonb;

-- ============================================================
-- 2. Mobile push tokens table (FCM + APNs)
--    Already exists as push_tokens — add indexes if missing
-- ============================================================
CREATE INDEX IF NOT EXISTS idx_push_tokens_user ON push_tokens(user_id);
CREATE INDEX IF NOT EXISTS idx_push_tokens_platform ON push_tokens(platform);

-- ============================================================
-- 3. pro_plans enrichment — ensure all columns exist
--    (PHP: Wo_Manage_Pro → pro_plans, referenced by jobs-runner)
-- ============================================================
ALTER TABLE pro_plans ADD COLUMN IF NOT EXISTS description TEXT DEFAULT '';
ALTER TABLE pro_plans ADD COLUMN IF NOT EXISTS features JSONB DEFAULT '[]'::jsonb;
ALTER TABLE pro_plans ADD COLUMN IF NOT EXISTS is_active BOOLEAN NOT NULL DEFAULT TRUE;

-- ============================================================
-- 4. recent_searches improvements
--    (PHP: Wo_RecentSearches)
-- ============================================================
ALTER TABLE recent_searches ADD COLUMN IF NOT EXISTS query_text VARCHAR(300) DEFAULT '';
CREATE INDEX IF NOT EXISTS idx_recent_searches_user ON recent_searches(user_id, searched_at DESC);

-- ============================================================
-- 5. users table: device token columns for push notifications
--    (PHP: android_m_device_id, ios_m_device_id referenced in get-general-data.php)
-- ============================================================
ALTER TABLE users ADD COLUMN IF NOT EXISTS android_device_id TEXT DEFAULT '';
ALTER TABLE users ADD COLUMN IF NOT EXISTS ios_device_id TEXT DEFAULT '';
ALTER TABLE users ADD COLUMN IF NOT EXISTS android_notification_id TEXT DEFAULT '';
ALTER TABLE users ADD COLUMN IF NOT EXISTS ios_notification_id TEXT DEFAULT '';

-- ============================================================
-- 6. user_ads: add missing view tracking column
-- ============================================================
ALTER TABLE user_ads ADD COLUMN IF NOT EXISTS views INT NOT NULL DEFAULT 0;

-- ============================================================
-- 7. reports: add resolved_by and notes
-- ============================================================
ALTER TABLE reports ADD COLUMN IF NOT EXISTS resolved_by BIGINT REFERENCES users(id);
ALTER TABLE reports ADD COLUMN IF NOT EXISTS admin_note TEXT DEFAULT '';
ALTER TABLE reports ADD COLUMN IF NOT EXISTS resolved_at TIMESTAMPTZ;

-- ============================================================
-- 8. Full-text search trigger on posts (idempotent)
-- ============================================================
CREATE OR REPLACE FUNCTION posts_search_vector_update() RETURNS trigger AS $$
BEGIN
    NEW.search_vector := to_tsvector('simple', COALESCE(NEW.content, ''));
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_posts_search ON posts;
CREATE TRIGGER trg_posts_search
    BEFORE INSERT OR UPDATE OF content ON posts
    FOR EACH ROW EXECUTE FUNCTION posts_search_vector_update();

-- ============================================================
-- 9. Ensure search_vector column exists on posts
-- ============================================================
ALTER TABLE posts ADD COLUMN IF NOT EXISTS search_vector TSVECTOR;
CREATE INDEX IF NOT EXISTS idx_posts_search ON posts USING GIN(search_vector);

-- ============================================================
-- 10. Backfill search_vector for existing posts/blogs
-- ============================================================
UPDATE posts SET search_vector = to_tsvector('simple', COALESCE(content, ''))
WHERE search_vector IS NULL;

UPDATE blogs SET search_vector =
    to_tsvector('english', COALESCE(title,'') || ' ' || COALESCE(description,'') || ' ' || COALESCE(content,''))
WHERE search_vector IS NULL;
