-- Migration: Admin panel parity — additional tables and columns for full PHP feature coverage

-- ── Currencies (stored in config in PHP, but managed via admin UI) ──────────
CREATE TABLE IF NOT EXISTS currencies (
    id          BIGSERIAL PRIMARY KEY,
    code        VARCHAR(10) NOT NULL UNIQUE,
    name        VARCHAR(100) NOT NULL DEFAULT '',
    symbol      VARCHAR(10) NOT NULL DEFAULT '',
    format      VARCHAR(30) NOT NULL DEFAULT '{symbol}{amount}',
    is_active   BOOLEAN NOT NULL DEFAULT TRUE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
INSERT INTO currencies (code, name, symbol) VALUES
    ('USD', 'US Dollar', '$'),
    ('EUR', 'Euro', '€'),
    ('GBP', 'British Pound', '£'),
    ('JPY', 'Japanese Yen', '¥'),
    ('BRL', 'Brazilian Real', 'R$'),
    ('INR', 'Indian Rupee', '₹'),
    ('TRY', 'Turkish Lira', '₺'),
    ('RUB', 'Russian Ruble', '₽')
ON CONFLICT (code) DO NOTHING;

-- ── Mass Notifications (admin sends to all users) ──────────────────────────
CREATE TABLE IF NOT EXISTS mass_notifications (
    id          BIGSERIAL PRIMARY KEY,
    admin_id    BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title       VARCHAR(255) NOT NULL DEFAULT '',
    message     TEXT NOT NULL DEFAULT '',
    url         TEXT NOT NULL DEFAULT '',
    target      VARCHAR(50) NOT NULL DEFAULT 'all', -- all, pro, new, male, female
    sent_count  INT NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ── Auto settings tables (auto-friend, auto-join, auto-like) ───────────────
-- These are stored in site_config but need tracking tables for which users/pages/groups
CREATE TABLE IF NOT EXISTS auto_follow_accounts (
    id          BIGSERIAL PRIMARY KEY,
    user_id     BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE UNIQUE
);

CREATE TABLE IF NOT EXISTS auto_join_groups (
    id          BIGSERIAL PRIMARY KEY,
    group_id    BIGINT NOT NULL REFERENCES groups(id) ON DELETE CASCADE UNIQUE
);

CREATE TABLE IF NOT EXISTS auto_like_pages (
    id          BIGSERIAL PRIMARY KEY,
    page_id     BIGINT NOT NULL REFERENCES pages(id) ON DELETE CASCADE UNIQUE
);

-- ── API Access Keys (for third-party integrations) ─────────────────────────
CREATE TABLE IF NOT EXISTS api_access_keys (
    id          BIGSERIAL PRIMARY KEY,
    name        VARCHAR(100) NOT NULL,
    api_key     VARCHAR(255) NOT NULL UNIQUE,
    secret_key  VARCHAR(255) NOT NULL,
    permissions JSONB NOT NULL DEFAULT '["read"]',
    is_active   BOOLEAN NOT NULL DEFAULT TRUE,
    created_by  BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    last_used   TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ── Referrals tracking ─────────────────────────────────────────────────────
-- users table already has referrer_id; add a dedicated referrals table for commission tracking
CREATE TABLE IF NOT EXISTS referral_earnings (
    id              BIGSERIAL PRIMARY KEY,
    referrer_id     BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    referred_id     BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    amount          NUMERIC(12,2) NOT NULL DEFAULT 0,
    source          VARCHAR(50) NOT NULL DEFAULT 'pro', -- pro, wallet, etc.
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_referral_earnings_referrer ON referral_earnings(referrer_id);

-- ── Fake user flag (admin-created test accounts) ──────────────────────────
ALTER TABLE users ADD COLUMN IF NOT EXISTS is_fake BOOLEAN NOT NULL DEFAULT FALSE;

-- ── Add missing gender_id reference to genders table ───────────────────────
-- Genders already created in migration 13, but ensure name column exists
-- (migration 13 already has it)

-- ── Sitemap generation log ────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS sitemap_logs (
    id          BIGSERIAL PRIMARY KEY,
    file_path   TEXT NOT NULL,
    entries     INT NOT NULL DEFAULT 0,
    generated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ── Ads countries targeting ───────────────────────────────────────────────
ALTER TABLE user_ads ADD COLUMN IF NOT EXISTS target_countries JSONB NOT NULL DEFAULT '[]';
ALTER TABLE user_ads ADD COLUMN IF NOT EXISTS target_gender VARCHAR(20) NOT NULL DEFAULT 'all';
ALTER TABLE user_ads ADD COLUMN IF NOT EXISTS daily_budget NUMERIC(12,2) NOT NULL DEFAULT 0;
ALTER TABLE user_ads ADD COLUMN IF NOT EXISTS total_budget NUMERIC(12,2) NOT NULL DEFAULT 0;

-- ── Content monetization settings columns ──────────────────────────────────
ALTER TABLE users ADD COLUMN IF NOT EXISTS monetization_enabled BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE users ADD COLUMN IF NOT EXISTS subscription_price NUMERIC(12,2) NOT NULL DEFAULT 0;

-- ── Forum enhancements ─────────────────────────────────────────────────────
ALTER TABLE forum_threads ADD COLUMN IF NOT EXISTS is_pinned BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE forum_threads ADD COLUMN IF NOT EXISTS is_locked BOOLEAN NOT NULL DEFAULT FALSE;

-- ── Movie/Game admin columns ───────────────────────────────────────────────
ALTER TABLE movies ADD COLUMN IF NOT EXISTS is_approved BOOLEAN NOT NULL DEFAULT TRUE;
ALTER TABLE movies ADD COLUMN IF NOT EXISTS admin_featured BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE games ADD COLUMN IF NOT EXISTS is_active BOOLEAN NOT NULL DEFAULT TRUE;

-- ── Story admin management ─────────────────────────────────────────────────
ALTER TABLE stories ADD COLUMN IF NOT EXISTS is_reported BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE stories ADD COLUMN IF NOT EXISTS admin_hidden BOOLEAN NOT NULL DEFAULT FALSE;
