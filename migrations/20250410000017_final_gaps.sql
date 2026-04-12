-- Migration 017: Final Gap Closure

-- ============================================================
-- 1. User social profile links (public display links)
-- PHP: facebook, linkedin, twitter, instagram, youtube, github columns
-- Stored as JSONB: { "facebook": "...", "linkedin": "...", "twitter": "...", ... }
-- ============================================================
ALTER TABLE users ADD COLUMN IF NOT EXISTS social_links JSONB NOT NULL DEFAULT '{}'::jsonb;

-- ============================================================
-- 2. Users: onboarding skip flags in dedicated columns
-- ============================================================
ALTER TABLE users ADD COLUMN IF NOT EXISTS start_up_info BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE users ADD COLUMN IF NOT EXISTS startup_image BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE users ADD COLUMN IF NOT EXISTS startup_follow BOOLEAN NOT NULL DEFAULT FALSE;

-- ============================================================
-- 3. User reports: index for faster lookup
-- ============================================================
CREATE INDEX IF NOT EXISTS idx_reports_reporter ON reports(reporter_id);
CREATE INDEX IF NOT EXISTS idx_reports_target ON reports(target_type, target_id);
CREATE INDEX IF NOT EXISTS idx_reports_status ON reports(status) WHERE status = 'pending';

-- ============================================================
-- 4. Conversations: page_id support for page messaging
-- (page_chat.php functionality - pages can have message inboxes)
-- ============================================================
ALTER TABLE conversations ADD COLUMN IF NOT EXISTS page_id BIGINT REFERENCES pages(page_id) ON DELETE CASCADE;
CREATE INDEX IF NOT EXISTS idx_conversations_page ON conversations(page_id) WHERE page_id IS NOT NULL;

-- ============================================================
-- 5. Messages: pin flag (for get_pin_message.php)
-- ============================================================
ALTER TABLE messages ADD COLUMN IF NOT EXISTS is_pinned BOOLEAN NOT NULL DEFAULT FALSE;
CREATE INDEX IF NOT EXISTS idx_messages_pinned ON messages(conversation_id) WHERE is_pinned = TRUE;

-- ============================================================
-- 6. Activities: ensure index exists
-- ============================================================
CREATE INDEX IF NOT EXISTS idx_activities_user ON activities(user_id, created_at DESC);

-- ============================================================
-- 7. Hashtags: ensure use_count stays non-negative
-- ============================================================
ALTER TABLE hashtags ADD CONSTRAINT IF NOT EXISTS hashtags_use_count_nonneg CHECK (use_count >= 0);

-- ============================================================
-- 8. Page chat: add page_id to conversations
--    ensure conversations can link to pages for page inbox
-- ============================================================
INSERT INTO site_config (category, key, value, value_type) VALUES
    ('features', 'page_chat',    'true',  'boolean'),
    ('features', 'group_chat',   'true',  'boolean'),
    ('limits',   'max_characters', '63206', 'integer')
ON CONFLICT (category, key) DO NOTHING;
