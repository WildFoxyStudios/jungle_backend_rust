CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- ============================================================
-- USERS
-- ============================================================
CREATE TABLE users (
    id              BIGSERIAL PRIMARY KEY,
    uuid            UUID NOT NULL DEFAULT gen_random_uuid() UNIQUE,
    username        VARCHAR(32) NOT NULL UNIQUE,
    email           VARCHAR(255) NOT NULL UNIQUE,
    phone_number    VARCHAR(20) UNIQUE,
    password_hash   TEXT NOT NULL,
    first_name      VARCHAR(50) NOT NULL DEFAULT '',
    last_name       VARCHAR(50) NOT NULL DEFAULT '',
    avatar          TEXT NOT NULL DEFAULT 'default-avatar.jpg',
    cover           TEXT NOT NULL DEFAULT 'default-cover.jpg',
    about           TEXT NOT NULL DEFAULT '',
    gender          VARCHAR(20) NOT NULL DEFAULT 'none',
    birthday        DATE,
    country_id      INT,
    city            VARCHAR(100) NOT NULL DEFAULT '',
    address         VARCHAR(300) NOT NULL DEFAULT '',
    website         VARCHAR(255) NOT NULL DEFAULT '',
    school          VARCHAR(200) NOT NULL DEFAULT '',
    working         VARCHAR(200) NOT NULL DEFAULT '',
    working_link    VARCHAR(255) NOT NULL DEFAULT '',
    language        VARCHAR(20) NOT NULL DEFAULT 'english',

    is_active       BOOLEAN NOT NULL DEFAULT FALSE,
    is_admin        BOOLEAN NOT NULL DEFAULT FALSE,
    is_pro          SMALLINT NOT NULL DEFAULT 0,
    is_verified     BOOLEAN NOT NULL DEFAULT FALSE,
    email_verified  BOOLEAN NOT NULL DEFAULT FALSE,
    phone_verified  BOOLEAN NOT NULL DEFAULT FALSE,
    email_code      VARCHAR(10) NOT NULL DEFAULT '',

    privacy_settings    JSONB NOT NULL DEFAULT '{
        "follow_privacy": "everyone",
        "message_privacy": "everyone",
        "post_privacy": "everyone",
        "profile_visibility": "everyone",
        "confirm_followers": false,
        "show_activities": true,
        "show_lastseen": true,
        "online_status": true
    }'::jsonb,

    notification_settings JSONB NOT NULL DEFAULT '{
        "e_liked": true, "e_wondered": true, "e_shared": true,
        "e_followed": true, "e_commented": true, "e_visited": true,
        "e_mentioned": true, "e_joined_group": true, "e_accepted": true,
        "e_profile_wall_post": true, "e_memory": true
    }'::jsonb,

    balance         DECIMAL(15,2) NOT NULL DEFAULT 0.00,
    wallet          DECIMAL(15,2) NOT NULL DEFAULT 0.00,
    points          BIGINT NOT NULL DEFAULT 0,

    social_logins   JSONB NOT NULL DEFAULT '{}'::jsonb,

    lat             DOUBLE PRECISION,
    lng             DOUBLE PRECISION,

    last_seen       TIMESTAMPTZ DEFAULT NOW(),
    is_online       BOOLEAN NOT NULL DEFAULT FALSE,

    two_factor_enabled  BOOLEAN NOT NULL DEFAULT FALSE,
    two_factor_method   VARCHAR(20),
    two_factor_secret   TEXT,

    deleted_at      TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_users_username ON users(username);
CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_last_seen ON users(last_seen);
CREATE INDEX idx_users_active ON users(id) WHERE deleted_at IS NULL AND is_active = TRUE;

-- ============================================================
-- SESSIONS
-- ============================================================
CREATE TABLE sessions (
    id              BIGSERIAL PRIMARY KEY,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash      TEXT NOT NULL UNIQUE,
    platform        VARCHAR(20) NOT NULL DEFAULT 'web',
    platform_details JSONB,
    ip_address      TEXT,
    expires_at      TIMESTAMPTZ NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_sessions_user ON sessions(user_id);
CREATE INDEX idx_sessions_token ON sessions(token_hash);

-- ============================================================
-- BACKUP CODES (2FA)
-- ============================================================
CREATE TABLE backup_codes (
    id              BIGSERIAL PRIMARY KEY,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    code_hash       TEXT NOT NULL,
    used            BOOLEAN NOT NULL DEFAULT FALSE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================
-- LOGIN ATTEMPTS (rate limiting)
-- ============================================================
CREATE TABLE login_attempts (
    id              BIGSERIAL PRIMARY KEY,
    ip_address      TEXT NOT NULL,
    user_id         BIGINT REFERENCES users(id) ON DELETE CASCADE,
    success         BOOLEAN NOT NULL DEFAULT FALSE,
    attempted_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_login_attempts_ip ON login_attempts(ip_address, attempted_at);

-- ============================================================
-- BANNED IPS
-- ============================================================
CREATE TABLE banned_ips (
    id              BIGSERIAL PRIMARY KEY,
    ip_address      TEXT NOT NULL UNIQUE,
    reason          TEXT NOT NULL DEFAULT '',
    banned_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at      TIMESTAMPTZ
);
