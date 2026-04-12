-- Push notification tokens
CREATE TABLE IF NOT EXISTS push_tokens (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token TEXT NOT NULL,
    platform VARCHAR(20) NOT NULL DEFAULT 'fcm',
    device_id VARCHAR(200),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(user_id, token)
);
CREATE INDEX IF NOT EXISTS idx_push_tokens_user ON push_tokens(user_id);

-- User invitation codes
CREATE TABLE IF NOT EXISTS invite_codes (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    code VARCHAR(50) NOT NULL UNIQUE,
    max_uses INT DEFAULT 10,
    uses INT DEFAULT 0,
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Color posts
ALTER TABLE posts ADD COLUMN IF NOT EXISTS colored_bg JSONB;

-- Saved posts cleanup index
CREATE INDEX IF NOT EXISTS idx_saved_posts_user ON saved_posts(user_id, created_at DESC);

-- Hidden posts cleanup index  
CREATE INDEX IF NOT EXISTS idx_hidden_posts_user ON hidden_posts(user_id);

-- Ads click tracking
CREATE TABLE IF NOT EXISTS ad_clicks (
    id BIGSERIAL PRIMARY KEY,
    ad_id BIGINT NOT NULL REFERENCES user_ads(id) ON DELETE CASCADE,
    user_id BIGINT REFERENCES users(id) ON DELETE SET NULL,
    ip_address VARCHAR(45),
    created_at TIMESTAMPTZ DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_ad_clicks_ad ON ad_clicks(ad_id, created_at DESC);

-- Gifts/Stickers
CREATE TABLE IF NOT EXISTS gift_categories (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(200) NOT NULL,
    is_active BOOLEAN DEFAULT TRUE,
    sort_order INT DEFAULT 0
);

CREATE TABLE IF NOT EXISTS gifts (
    id BIGSERIAL PRIMARY KEY,
    category_id BIGINT REFERENCES gift_categories(id) ON DELETE SET NULL,
    name VARCHAR(200) NOT NULL,
    image TEXT NOT NULL,
    price NUMERIC(10,2) DEFAULT 0,
    is_active BOOLEAN DEFAULT TRUE
);

CREATE TABLE IF NOT EXISTS user_gifts (
    id BIGSERIAL PRIMARY KEY,
    sender_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recipient_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    gift_id BIGINT NOT NULL REFERENCES gifts(id) ON DELETE CASCADE,
    message TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Stickers
CREATE TABLE IF NOT EXISTS sticker_packs (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(200) NOT NULL,
    preview_url TEXT,
    is_premium BOOLEAN DEFAULT FALSE,
    price NUMERIC(10,2) DEFAULT 0,
    is_active BOOLEAN DEFAULT TRUE
);

CREATE TABLE IF NOT EXISTS stickers (
    id BIGSERIAL PRIMARY KEY,
    pack_id BIGINT NOT NULL REFERENCES sticker_packs(id) ON DELETE CASCADE,
    image_url TEXT NOT NULL,
    sort_order INT DEFAULT 0
);

CREATE TABLE IF NOT EXISTS user_sticker_packs (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    pack_id BIGINT NOT NULL REFERENCES sticker_packs(id) ON DELETE CASCADE,
    purchased_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(user_id, pack_id)
);

-- Activity log index
CREATE INDEX IF NOT EXISTS idx_activities_action ON activities(action, created_at DESC);

-- User experience table (for LinkedIn mode)
CREATE TABLE IF NOT EXISTS user_experience (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title VARCHAR(500) NOT NULL,
    company VARCHAR(500) NOT NULL,
    location VARCHAR(500),
    description TEXT,
    start_date DATE,
    end_date DATE,
    is_current BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Seq reset for all BIGSERIAL after data migration
-- Run after migrate_mysql_to_pg.py:
-- SELECT setval('users_id_seq', (SELECT COALESCE(MAX(id), 0) + 1 FROM users));
-- SELECT setval('posts_id_seq', (SELECT COALESCE(MAX(id), 0) + 1 FROM posts));
-- etc.
