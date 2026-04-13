-- Full-text search vectors
ALTER TABLE posts ADD COLUMN IF NOT EXISTS search_vector tsvector;
CREATE INDEX IF NOT EXISTS idx_posts_search ON posts USING GIN(search_vector);

CREATE OR REPLACE FUNCTION posts_search_update() RETURNS TRIGGER AS $$
BEGIN
    NEW.search_vector := setweight(to_tsvector('simple', COALESCE(NEW.content, '')), 'A');
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_posts_search ON posts;
CREATE TRIGGER trg_posts_search BEFORE INSERT OR UPDATE OF content
    ON posts FOR EACH ROW EXECUTE FUNCTION posts_search_update();

ALTER TABLE blogs ADD COLUMN IF NOT EXISTS search_vector tsvector;
CREATE INDEX IF NOT EXISTS idx_blogs_search ON blogs USING GIN(search_vector);

CREATE OR REPLACE FUNCTION blogs_search_update() RETURNS TRIGGER AS $$
BEGIN
    NEW.search_vector :=
        setweight(to_tsvector('simple', COALESCE(NEW.title, '')), 'A') ||
        setweight(to_tsvector('simple', COALESCE(NEW.description, '')), 'B');
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_blogs_search ON blogs;
CREATE TRIGGER trg_blogs_search BEFORE INSERT OR UPDATE OF title, description
    ON blogs FOR EACH ROW EXECUTE FUNCTION blogs_search_update();

-- Pro plans table
CREATE TABLE IF NOT EXISTS pro_plans (
    id BIGSERIAL PRIMARY KEY,
    plan_type VARCHAR(50) NOT NULL UNIQUE,
    title VARCHAR(200) NOT NULL,
    price NUMERIC(10,2) NOT NULL DEFAULT 0,
    period_days INT NOT NULL DEFAULT 30,
    features JSONB DEFAULT '[]'::jsonb,
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- User ads improvements
ALTER TABLE user_ads ADD COLUMN IF NOT EXISTS bid_type VARCHAR(20) DEFAULT 'views';
ALTER TABLE user_ads ADD COLUMN IF NOT EXISTS clicks BIGINT DEFAULT 0;

-- Profile fields (custom fields)
CREATE TABLE IF NOT EXISTS profile_fields (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(200) NOT NULL,
    field_type VARCHAR(50) DEFAULT 'text',
    options JSONB DEFAULT '[]'::jsonb,
    is_required BOOLEAN DEFAULT FALSE,
    sort_order INT DEFAULT 0,
    is_active BOOLEAN DEFAULT TRUE
);

CREATE TABLE IF NOT EXISTS user_field_values (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    field_id BIGINT NOT NULL REFERENCES profile_fields(id) ON DELETE CASCADE,
    value TEXT,
    UNIQUE(user_id, field_id)
);

-- Custom pages
CREATE TABLE IF NOT EXISTS custom_pages (
    id BIGSERIAL PRIMARY KEY,
    title VARCHAR(500) NOT NULL,
    slug VARCHAR(200) NOT NULL UNIQUE,
    content TEXT,
    page_type VARCHAR(50) DEFAULT 'custom',
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Email templates
CREATE TABLE IF NOT EXISTS email_templates (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(200) NOT NULL UNIQUE,
    subject VARCHAR(500),
    body TEXT,
    variables JSONB DEFAULT '[]'::jsonb,
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- User certifications
CREATE TABLE IF NOT EXISTS user_certifications (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name VARCHAR(500) NOT NULL,
    organization VARCHAR(500),
    issue_date DATE,
    expiry_date DATE,
    credential_url TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- User projects
CREATE TABLE IF NOT EXISTS user_projects (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name VARCHAR(500) NOT NULL,
    description TEXT,
    url TEXT,
    start_date DATE,
    end_date DATE,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Recent searches (may already exist from earlier migration with searched_at column)
ALTER TABLE recent_searches ADD COLUMN IF NOT EXISTS query VARCHAR(500) NOT NULL DEFAULT '';
ALTER TABLE recent_searches ADD COLUMN IF NOT EXISTS search_type VARCHAR(50) DEFAULT 'all';

-- Colored post templates
CREATE TABLE IF NOT EXISTS colored_post_templates (
    id BIGSERIAL PRIMARY KEY,
    image TEXT NOT NULL,
    text_color VARCHAR(20) DEFAULT '#ffffff',
    is_active BOOLEAN DEFAULT TRUE,
    sort_order INT DEFAULT 0
);

-- Live streaming
ALTER TABLE users ADD COLUMN IF NOT EXISTS is_live BOOLEAN DEFAULT FALSE;
ALTER TABLE users ADD COLUMN IF NOT EXISTS live_stream_id VARCHAR(200);

-- Reels: posts with is_reel = TRUE (no separate table needed, just index)
ALTER TABLE posts ADD COLUMN IF NOT EXISTS is_reel BOOLEAN DEFAULT FALSE;
ALTER TABLE posts ADD COLUMN IF NOT EXISTS view_count BIGINT DEFAULT 0;
CREATE INDEX IF NOT EXISTS idx_posts_reels ON posts(id DESC) WHERE is_reel = TRUE AND deleted_at IS NULL;

-- Verification requests
CREATE TABLE IF NOT EXISTS verification_requests (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    full_name VARCHAR(500),
    document_url TEXT,
    status VARCHAR(20) DEFAULT 'pending',
    admin_note TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    reviewed_at TIMESTAMPTZ
);

-- Newsletter subscribers
CREATE TABLE IF NOT EXISTS newsletter_subscribers (
    id BIGSERIAL PRIMARY KEY,
    email VARCHAR(500) NOT NULL UNIQUE,
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Activities log
CREATE TABLE IF NOT EXISTS activities (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    action VARCHAR(100) NOT NULL,
    target_type VARCHAR(50),
    target_id BIGINT,
    metadata JSONB DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_activities_user ON activities(user_id, created_at DESC);

-- Banned IPs
CREATE TABLE IF NOT EXISTS banned_ips (
    id BIGSERIAL PRIMARY KEY,
    ip_address VARCHAR(45) NOT NULL UNIQUE,
    reason TEXT,
    banned_by BIGINT REFERENCES users(id),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    expires_at TIMESTAMPTZ
);

-- Backfill search vectors for existing data
UPDATE posts SET search_vector = setweight(to_tsvector('simple', COALESCE(content, '')), 'A') WHERE search_vector IS NULL;
UPDATE blogs SET search_vector = setweight(to_tsvector('simple', COALESCE(title, '')), 'A') || setweight(to_tsvector('simple', COALESCE(description, '')), 'B') WHERE search_vector IS NULL;
