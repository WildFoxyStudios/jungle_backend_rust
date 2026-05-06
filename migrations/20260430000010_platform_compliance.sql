-- ============================================================================
-- Phases 19-26: Platform, Compliance, Anti-abuse, Activity, Collaborative, Fundraising
-- Combined migration — all tables are independent, no cross-dependencies.
-- ============================================================================

-- ============================================================================
-- Phase 19: Developer Platform — Webhooks + Embeds
-- ============================================================================
CREATE TABLE IF NOT EXISTS webhooks (
    id BIGSERIAL PRIMARY KEY,
    app_id BIGINT NOT NULL REFERENCES oauth_apps(id) ON DELETE CASCADE,
    url TEXT NOT NULL,
    events JSONB NOT NULL DEFAULT '[]',
    secret VARCHAR(256),
    is_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS webhook_deliveries (
    id BIGSERIAL PRIMARY KEY,
    webhook_id BIGINT NOT NULL REFERENCES webhooks(id) ON DELETE CASCADE,
    event_type VARCHAR(128) NOT NULL,
    payload JSONB NOT NULL,
    response_status INT,
    response_body TEXT,
    attempts INT NOT NULL DEFAULT 0,
    next_retry_at TIMESTAMPTZ,
    status VARCHAR(20) NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'success', 'failed', 'giving_up')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================================
-- Phase 20: Compliance GDPR
-- ============================================================================
CREATE TABLE IF NOT EXISTS data_export_requests (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status VARCHAR(20) NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'processing', 'ready', 'expired', 'error')),
    file_url TEXT,
    file_size_bytes BIGINT,
    requested_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ
);

ALTER TABLE users ADD COLUMN IF NOT EXISTS erasure_scheduled_at TIMESTAMPTZ;

-- Cookie consent
CREATE TABLE IF NOT EXISTS cookie_consents (
    user_id BIGINT REFERENCES users(id) ON DELETE CASCADE,
    session_id VARCHAR(128),
    preferences JSONB NOT NULL DEFAULT '{}',
    consented_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, session_id)
);

-- Email tracking
CREATE TABLE IF NOT EXISTS email_events (
    id BIGSERIAL PRIMARY KEY,
    email_id BIGINT,
    event_type VARCHAR(20) NOT NULL CHECK (event_type IN ('sent', 'delivered', 'opened', 'clicked', 'bounced', 'complained')),
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ip VARCHAR(45),
    user_agent TEXT
);

-- ============================================================================
-- Phase 21: Anti-abuse + Memorialized + Support
-- ============================================================================

-- Login fingerprints
CREATE TABLE IF NOT EXISTS login_fingerprints (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    fingerprint VARCHAR(256) NOT NULL,
    first_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    trust_level INT NOT NULL DEFAULT 0
);

ALTER TABLE users ADD COLUMN IF NOT EXISTS trust_level INT NOT NULL DEFAULT 0;

-- Memorialization
ALTER TABLE users ADD COLUMN IF NOT EXISTS memorialized_at TIMESTAMPTZ;
ALTER TABLE users ADD COLUMN IF NOT EXISTS legacy_contact_id BIGINT REFERENCES users(id);

-- Help Center
CREATE TABLE IF NOT EXISTS help_articles (
    id BIGSERIAL PRIMARY KEY,
    slug VARCHAR(256) NOT NULL UNIQUE,
    locale VARCHAR(5) NOT NULL DEFAULT 'en',
    title VARCHAR(512) NOT NULL,
    content TEXT NOT NULL,
    category VARCHAR(128),
    sort_order INT NOT NULL DEFAULT 0,
    is_published BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Support tickets
CREATE TABLE IF NOT EXISTS support_tickets (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    subject VARCHAR(512) NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'in_progress', 'waiting_user', 'resolved', 'closed')),
    priority VARCHAR(10) NOT NULL DEFAULT 'medium' CHECK (priority IN ('low', 'medium', 'high', 'urgent')),
    assigned_to BIGINT REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    resolved_at TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS support_ticket_messages (
    id BIGSERIAL PRIMARY KEY,
    ticket_id BIGINT NOT NULL REFERENCES support_tickets(id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    content TEXT NOT NULL,
    is_staff_reply BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Status page
CREATE TABLE IF NOT EXISTS status_incidents (
    id BIGSERIAL PRIMARY KEY,
    title VARCHAR(512) NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'investigating' CHECK (status IN ('investigating', 'identified', 'monitoring', 'resolved')),
    affected_services JSONB NOT NULL DEFAULT '[]',
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    resolved_at TIMESTAMPTZ
);

-- ============================================================================
-- Phase 23: Activity Log
-- ============================================================================

-- Note: `activities` table already exists. Add classification columns.
ALTER TABLE activities ADD COLUMN IF NOT EXISTS activity_type VARCHAR(50);
ALTER TABLE activities ADD COLUMN IF NOT EXISTS ip_address VARCHAR(45);
ALTER TABLE activities ADD COLUMN IF NOT EXISTS user_agent TEXT;

-- ============================================================================
-- Phase 24: Collaborative Features
-- ============================================================================

CREATE TABLE IF NOT EXISTS post_coauthors (
    post_id BIGINT NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status VARCHAR(20) NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'accepted', 'rejected')),
    added_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (post_id, user_id)
);

ALTER TABLE posts ADD COLUMN IF NOT EXISTS is_collaborative BOOLEAN NOT NULL DEFAULT FALSE;

CREATE TABLE IF NOT EXISTS album_collaborators (
    album_id BIGINT NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    can_add BOOLEAN NOT NULL DEFAULT TRUE,
    can_delete BOOLEAN NOT NULL DEFAULT FALSE,
    added_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (album_id, user_id)
);

ALTER TABLE posts ADD COLUMN IF NOT EXISTS is_shared BOOLEAN NOT NULL DEFAULT FALSE;

CREATE TABLE IF NOT EXISTS watch_parties (
    id BIGSERIAL PRIMARY KEY,
    host_user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    video_type VARCHAR(20) NOT NULL CHECK (video_type IN ('movie', 'reel', 'live', 'upload')),
    video_id BIGINT NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'live' CHECK (status IN ('live', 'ended')),
    participant_count INT NOT NULL DEFAULT 0,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ended_at TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS watch_party_participants (
    party_id BIGINT NOT NULL REFERENCES watch_parties(id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    joined_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (party_id, user_id)
);

-- ============================================================================
-- Phase 25: Search & Discovery
-- ============================================================================

ALTER TABLE posts ADD COLUMN IF NOT EXISTS content_lang VARCHAR(5);

CREATE TABLE IF NOT EXISTS post_translations (
    post_id BIGINT NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    lang VARCHAR(5) NOT NULL,
    translated_text TEXT NOT NULL,
    translated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (post_id, lang)
);

-- Search aliases for Graph Search
CREATE TABLE IF NOT EXISTS search_aliases (
    id BIGSERIAL PRIMARY KEY,
    phrase TEXT NOT NULL,
    field_name TEXT NOT NULL,
    aliased_value TEXT NOT NULL
);

-- ============================================================================
-- Phase 26: Personal Fundraising
-- ============================================================================

ALTER TABLE fundings ADD COLUMN IF NOT EXISTS funding_type VARCHAR(20) NOT NULL DEFAULT 'project'
    CHECK (funding_type IN ('project', 'personal_cause', 'nonprofit', 'emergency'));
ALTER TABLE fundings ADD COLUMN IF NOT EXISTS beneficiary_name TEXT;
ALTER TABLE fundings ADD COLUMN IF NOT EXISTS beneficiary_relationship TEXT;
ALTER TABLE fundings ADD COLUMN IF NOT EXISTS withdrawal_frequency VARCHAR(20) DEFAULT 'on_completion'
    CHECK (withdrawal_frequency IN ('on_completion', 'weekly', 'as_donated'));
ALTER TABLE fundings ADD COLUMN IF NOT EXISTS is_transparent BOOLEAN NOT NULL DEFAULT TRUE;
ALTER TABLE fundings ADD COLUMN IF NOT EXISTS verified_at TIMESTAMPTZ;
