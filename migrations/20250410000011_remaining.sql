-- Migration 011: Remaining tables (calls, stickers, gifts, pro, creator, oauth, misc)

-- Verification requests
CREATE TABLE verification_requests (
    id              BIGSERIAL PRIMARY KEY,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    full_name       VARCHAR(200) NOT NULL,
    message         TEXT DEFAULT '',
    document_url    TEXT NOT NULL,
    status          VARCHAR(20) DEFAULT 'pending',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Activities
CREATE TABLE activities (
    id              BIGSERIAL PRIMARY KEY,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    activity_type   VARCHAR(50) NOT NULL,
    target_type     VARCHAR(30),
    target_id       BIGINT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_activities_user ON activities(user_id, created_at DESC);

-- Stickers
CREATE TABLE stickers (
    id              BIGSERIAL PRIMARY KEY,
    name            VARCHAR(100) NOT NULL,
    image           TEXT NOT NULL,
    active          BOOLEAN NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Gifts
CREATE TABLE gifts (
    id              BIGSERIAL PRIMARY KEY,
    name            VARCHAR(250),
    media_file      TEXT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE user_gifts (
    id              BIGSERIAL PRIMARY KEY,
    gift_id         BIGINT NOT NULL REFERENCES gifts(id) ON DELETE CASCADE,
    sender_id       BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    receiver_id     BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Calls
CREATE TABLE calls (
    id              BIGSERIAL PRIMARY KEY,
    caller_id       BIGINT NOT NULL REFERENCES users(id),
    callee_id       BIGINT NOT NULL REFERENCES users(id),
    call_type       VARCHAR(10) NOT NULL,
    provider        VARCHAR(20) DEFAULT 'agora',
    room_name       VARCHAR(100) NOT NULL,
    status          VARCHAR(20) NOT NULL DEFAULT 'ringing',
    access_tokens   JSONB DEFAULT '{}'::jsonb,
    started_at      TIMESTAMPTZ,
    ended_at        TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_calls_caller ON calls(caller_id, created_at DESC);
CREATE INDEX idx_calls_callee ON calls(callee_id, created_at DESC);

-- Pro subscriptions
CREATE TABLE pro_subscriptions (
    id              BIGSERIAL PRIMARY KEY,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    plan_type       SMALLINT NOT NULL,
    period          VARCHAR(20) NOT NULL,
    amount_paid     DECIMAL(15,2) NOT NULL,
    started_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at      TIMESTAMPTZ,
    is_active       BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE INDEX idx_pro_subs_user ON pro_subscriptions(user_id) WHERE is_active = TRUE;

-- Creator tiers & subscriptions
CREATE TABLE creator_tiers (
    id              BIGSERIAL PRIMARY KEY,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name            VARCHAR(100) NOT NULL,
    price           DECIMAL(15,2) NOT NULL,
    description     TEXT DEFAULT '',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE creator_subscriptions (
    id              BIGSERIAL PRIMARY KEY,
    creator_id      BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    subscriber_id   BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    tier_id         BIGINT REFERENCES creator_tiers(id),
    amount          DECIMAL(15,2) NOT NULL,
    status          VARCHAR(20) DEFAULT 'active',
    started_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at      TIMESTAMPTZ,
    UNIQUE(creator_id, subscriber_id)
);

-- Announcement views
CREATE TABLE announcement_views (
    announcement_id BIGINT NOT NULL REFERENCES announcements(id) ON DELETE CASCADE,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    PRIMARY KEY(announcement_id, user_id)
);

-- OAuth apps & tokens
CREATE TABLE oauth_apps (
    id              BIGSERIAL PRIMARY KEY,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name            VARCHAR(100) NOT NULL,
    description     TEXT DEFAULT '',
    website_url     VARCHAR(255) DEFAULT '',
    callback_url    VARCHAR(255) NOT NULL,
    avatar          TEXT DEFAULT 'default-app.png',
    client_id       VARCHAR(64) NOT NULL UNIQUE,
    client_secret   VARCHAR(128) NOT NULL,
    active          BOOLEAN NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE oauth_tokens (
    id              BIGSERIAL PRIMARY KEY,
    app_id          BIGINT NOT NULL REFERENCES oauth_apps(id) ON DELETE CASCADE,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    access_token    TEXT NOT NULL UNIQUE,
    refresh_token   TEXT UNIQUE,
    scopes          TEXT[] DEFAULT '{}',
    expires_at      TIMESTAMPTZ NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Colored post templates
CREATE TABLE colored_post_templates (
    id              BIGSERIAL PRIMARY KEY,
    color_1         VARCHAR(20) NOT NULL,
    color_2         VARCHAR(20) NOT NULL,
    text_color      VARCHAR(20) NOT NULL,
    image           TEXT DEFAULT ''
);

-- Reaction types (configurable)
CREATE TABLE reaction_types (
    id              BIGSERIAL PRIMARY KEY,
    name            VARCHAR(30) NOT NULL UNIQUE,
    icon            TEXT NOT NULL,
    is_default      BOOLEAN NOT NULL DEFAULT FALSE
);

-- Invitation links
CREATE TABLE invitation_links (
    id              BIGSERIAL PRIMARY KEY,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    code            VARCHAR(300) NOT NULL UNIQUE,
    used_by         BIGINT REFERENCES users(id),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at      TIMESTAMPTZ
);

-- Affiliate requests
CREATE TABLE affiliate_requests (
    id              BIGSERIAL PRIMARY KEY,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    referred_id     BIGINT REFERENCES users(id),
    amount          DECIMAL(15,2) NOT NULL DEFAULT 0,
    status          VARCHAR(20) DEFAULT 'pending',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Recent searches
CREATE TABLE recent_searches (
    id              BIGSERIAL PRIMARY KEY,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    search_type     VARCHAR(20) NOT NULL,
    target_id       BIGINT NOT NULL,
    searched_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_recent_searches_user ON recent_searches(user_id, searched_at DESC);
