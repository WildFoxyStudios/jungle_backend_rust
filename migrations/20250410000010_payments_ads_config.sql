-- Migration 010: Payments, Ads, Config, Translations, Reports, Custom Pages

-- Payment transactions
CREATE TABLE payment_transactions (
    id              BIGSERIAL PRIMARY KEY,
    uuid            UUID NOT NULL DEFAULT gen_random_uuid() UNIQUE,
    user_id         BIGINT NOT NULL REFERENCES users(id),
    amount          DECIMAL(15,2) NOT NULL,
    currency        VARCHAR(10) DEFAULT 'USD',
    provider        VARCHAR(30) NOT NULL,
    provider_ref    TEXT,
    type            VARCHAR(30) NOT NULL,
    status          VARCHAR(20) NOT NULL DEFAULT 'pending',
    metadata        JSONB DEFAULT '{}'::jsonb,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_payment_tx_user ON payment_transactions(user_id, created_at DESC);
CREATE INDEX idx_payment_tx_status ON payment_transactions(status);

-- Withdrawal requests
CREATE TABLE withdrawal_requests (
    id              BIGSERIAL PRIMARY KEY,
    user_id         BIGINT NOT NULL REFERENCES users(id),
    amount          DECIMAL(15,2) NOT NULL,
    method          VARCHAR(30) NOT NULL,
    details         JSONB NOT NULL,
    status          VARCHAR(20) NOT NULL DEFAULT 'pending',
    admin_note      TEXT DEFAULT '',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    processed_at    TIMESTAMPTZ
);

-- User Ads
CREATE TABLE user_ads (
    id              BIGSERIAL PRIMARY KEY,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    ad_type         VARCHAR(30) NOT NULL,
    target_id       BIGINT,
    name            VARCHAR(200) DEFAULT '',
    headline        VARCHAR(200) DEFAULT '',
    description     TEXT DEFAULT '',
    image           TEXT DEFAULT '',
    url             TEXT DEFAULT '',
    audience        VARCHAR(20) DEFAULT 'all',
    placement       VARCHAR(20) DEFAULT 'sidebar',
    budget          DECIMAL(15,2) NOT NULL DEFAULT 0,
    bid_type        VARCHAR(20) DEFAULT 'cpc',
    status          VARCHAR(20) DEFAULT 'pending',
    impressions     INT NOT NULL DEFAULT 0,
    clicks          INT NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_user_ads_user ON user_ads(user_id);
CREATE INDEX idx_user_ads_active ON user_ads(status, placement) WHERE status = 'active';

-- Site Configuration
CREATE TABLE site_config (
    id              BIGSERIAL PRIMARY KEY,
    category        VARCHAR(50) NOT NULL,
    key             VARCHAR(100) NOT NULL,
    value           TEXT NOT NULL DEFAULT '',
    value_type      VARCHAR(20) DEFAULT 'string',
    UNIQUE(category, key)
);

-- Translations
CREATE TABLE translations (
    id              BIGSERIAL PRIMARY KEY,
    lang            VARCHAR(20) NOT NULL,
    key             VARCHAR(200) NOT NULL,
    value           TEXT NOT NULL DEFAULT '',
    UNIQUE(lang, key)
);

CREATE INDEX idx_translations_lang ON translations(lang);

-- Languages
CREATE TABLE languages (
    id              BIGSERIAL PRIMARY KEY,
    name            VARCHAR(50) NOT NULL UNIQUE,
    iso_code        VARCHAR(10) NOT NULL,
    direction       VARCHAR(3) NOT NULL DEFAULT 'ltr',
    flag_image      TEXT DEFAULT '',
    active          BOOLEAN NOT NULL DEFAULT TRUE
);

-- Custom pages
CREATE TABLE custom_pages (
    id              BIGSERIAL PRIMARY KEY,
    slug            VARCHAR(100) NOT NULL UNIQUE,
    title           VARCHAR(200) NOT NULL,
    content         TEXT,
    page_type       VARCHAR(20) DEFAULT 'custom',
    active          BOOLEAN NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Email templates
CREATE TABLE email_templates (
    id              BIGSERIAL PRIMARY KEY,
    name            VARCHAR(100) NOT NULL UNIQUE,
    subject         VARCHAR(200) NOT NULL DEFAULT '',
    body            TEXT NOT NULL DEFAULT '',
    variables       TEXT[] DEFAULT '{}',
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Reports
CREATE TABLE reports (
    id              BIGSERIAL PRIMARY KEY,
    reporter_id     BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    target_type     VARCHAR(30) NOT NULL,
    target_id       BIGINT NOT NULL,
    reason          VARCHAR(100) NOT NULL,
    description     TEXT DEFAULT '',
    status          VARCHAR(20) DEFAULT 'pending',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_reports_status ON reports(status, created_at DESC);

-- Announcements
CREATE TABLE announcements (
    id              BIGSERIAL PRIMARY KEY,
    text            TEXT NOT NULL,
    target          VARCHAR(20) DEFAULT 'all',
    active          BOOLEAN NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
