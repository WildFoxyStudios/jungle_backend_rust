-- ============================================================================
-- Phase 10: Advanced Messaging
-- ============================================================================

-- Full-text search for messages (column is `content`, not `message_text`)
ALTER TABLE messages ADD COLUMN IF NOT EXISTS search_vector tsvector
    GENERATED ALWAYS AS (to_tsvector('english', coalesce(content, ''))) STORED;
CREATE INDEX IF NOT EXISTS idx_messages_search ON messages USING GIN (search_vector);

-- In-chat polls
CREATE TABLE IF NOT EXISTS message_polls (
    id BIGSERIAL PRIMARY KEY,
    conversation_id BIGINT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    creator_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    question TEXT NOT NULL,
    options JSONB NOT NULL DEFAULT '[]',
    is_closed BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS message_poll_votes (
    poll_id BIGINT NOT NULL REFERENCES message_polls(id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    option_index INT NOT NULL,
    voted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (poll_id, user_id)
);

-- ============================================================================
-- Phase 11: Advanced Groups + Pages
-- ============================================================================

-- Group rules
CREATE TABLE IF NOT EXISTS group_rules (
    id BIGSERIAL PRIMARY KEY,
    group_id BIGINT NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
    sort_order INT NOT NULL DEFAULT 0,
    title VARCHAR(256) NOT NULL,
    description TEXT
);

-- Page CTA button
ALTER TABLE pages ADD COLUMN IF NOT EXISTS cta_type VARCHAR(20) CHECK (cta_type IN ('call', 'message', 'book', 'shop', 'learn_more', 'sign_up', 'donate'));
ALTER TABLE pages ADD COLUMN IF NOT EXISTS cta_url TEXT;
ALTER TABLE pages ADD COLUMN IF NOT EXISTS cta_label VARCHAR(128);

-- Page multi-location
CREATE TABLE IF NOT EXISTS page_locations (
    id BIGSERIAL PRIMARY KEY,
    page_id BIGINT NOT NULL REFERENCES pages(id) ON DELETE CASCADE,
    address TEXT,
    lat DOUBLE PRECISION,
    lng DOUBLE PRECISION,
    phone VARCHAR(64),
    is_primary BOOLEAN NOT NULL DEFAULT FALSE
);

-- Page insights (precomputed stats)
CREATE TABLE IF NOT EXISTS page_insights_daily (
    page_id BIGINT NOT NULL REFERENCES pages(id) ON DELETE CASCADE,
    date DATE NOT NULL,
    impressions INT NOT NULL DEFAULT 0,
    engagements INT NOT NULL DEFAULT 0,
    new_likes INT NOT NULL DEFAULT 0,
    PRIMARY KEY (page_id, date)
);

-- Group insights
CREATE TABLE IF NOT EXISTS group_insights_daily (
    group_id BIGINT NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
    date DATE NOT NULL,
    new_members INT NOT NULL DEFAULT 0,
    active_members INT NOT NULL DEFAULT 0,
    posts_created INT NOT NULL DEFAULT 0,
    PRIMARY KEY (group_id, date)
);

-- ============================================================================
-- Phase 12: Advanced Events
-- ============================================================================

-- Event recurrence (iCalendar RRULE)
ALTER TABLE events ADD COLUMN IF NOT EXISTS rrule VARCHAR(512);

-- Co-hosts
CREATE TABLE IF NOT EXISTS event_cohosts (
    event_id BIGINT NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role VARCHAR(20) NOT NULL DEFAULT 'cohost',
    PRIMARY KEY (event_id, user_id)
);

-- Event tickets
CREATE TABLE IF NOT EXISTS event_tickets (
    id BIGSERIAL PRIMARY KEY,
    event_id BIGINT NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    tier VARCHAR(64) NOT NULL DEFAULT 'general',
    price_cents BIGINT NOT NULL DEFAULT 0,
    qr_code TEXT,
    is_used BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Event discussion
CREATE TABLE IF NOT EXISTS event_discussions (
    id BIGSERIAL PRIMARY KEY,
    event_id BIGINT NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    content TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
