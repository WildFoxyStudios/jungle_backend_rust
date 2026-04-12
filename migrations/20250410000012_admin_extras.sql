-- ============================================================
-- Admin extras: backup logs, newsletter subscribers
-- ============================================================

CREATE TABLE IF NOT EXISTS backup_logs (
    id              BIGSERIAL PRIMARY KEY,
    filename        VARCHAR(255) NOT NULL,
    size_bytes      BIGINT NOT NULL DEFAULT 0,
    status          VARCHAR(20) NOT NULL DEFAULT 'pending',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS newsletter_subscribers (
    id              BIGSERIAL PRIMARY KEY,
    email           VARCHAR(255) NOT NULL UNIQUE,
    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_newsletter_subscribers_active ON newsletter_subscribers(is_active);
