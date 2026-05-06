-- Consolidate WoWonder PHP `users` legacy fields onto first-class columns.
-- Idempotent — every alter is `IF NOT EXISTS`.

-- ── social_links + referrer_id ─────────────────────────────────────────
-- (`social_links` already shipped earlier as a JSONB column; we keep the
-- guard so this script is safe to re-run on top of it.)
ALTER TABLE users
    ADD COLUMN IF NOT EXISTS social_links JSONB NOT NULL DEFAULT '{}'::jsonb,
    ADD COLUMN IF NOT EXISTS referrer_id BIGINT NULL REFERENCES users(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_users_referrer ON users(referrer_id) WHERE referrer_id IS NOT NULL;

-- ── pending_email_changes ──────────────────────────────────────────────
-- Mirrors WoWonder's email change flow: store the new address and a
-- one-time confirmation token until the user clicks the verification link.
CREATE TABLE IF NOT EXISTS pending_email_changes (
    id            BIGSERIAL PRIMARY KEY,
    user_id       BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    new_email     VARCHAR(255) NOT NULL,
    token         VARCHAR(128) NOT NULL UNIQUE,
    requested_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at    TIMESTAMPTZ NOT NULL,
    confirmed_at  TIMESTAMPTZ NULL,
    UNIQUE (user_id)
);
CREATE INDEX IF NOT EXISTS idx_pending_email_changes_token
    ON pending_email_changes(token);

-- ── pending_phone_changes ──────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS pending_phone_changes (
    id            BIGSERIAL PRIMARY KEY,
    user_id       BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    new_phone     VARCHAR(50) NOT NULL,
    code          VARCHAR(16) NOT NULL,
    requested_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at    TIMESTAMPTZ NOT NULL,
    confirmed_at  TIMESTAMPTZ NULL,
    attempts      INT NOT NULL DEFAULT 0,
    UNIQUE (user_id)
);

-- ── payout_methods ─────────────────────────────────────────────────────
-- Replaces the per-provider columns the PHP version stored on `users`
-- (paypal_email, bank_*, btc_address, …). Every method is one row in
-- `payout_methods` keyed by provider name; `details` holds the
-- provider-specific fields as JSONB.
CREATE TABLE IF NOT EXISTS payout_methods (
    id            BIGSERIAL PRIMARY KEY,
    user_id       BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider      VARCHAR(50) NOT NULL,
    details       JSONB NOT NULL DEFAULT '{}'::jsonb,
    is_default    BOOLEAN NOT NULL DEFAULT FALSE,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, provider)
);
CREATE INDEX IF NOT EXISTS idx_payout_methods_user ON payout_methods(user_id);
