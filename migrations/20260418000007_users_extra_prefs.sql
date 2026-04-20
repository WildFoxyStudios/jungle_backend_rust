-- ═══════════════════════════════════════════════════════════════════
-- Extra per-user preferences columns (WoWonder parity)
-- ═══════════════════════════════════════════════════════════════════
--
-- These columns cover:
--  - chat presence status (online/busy/away/offline/invisible)
--  - per-user layout preferences (collapsed groups/services/users sidebars)
--  - per-user theme overrides (colors, font, density)
--  - pro subscription grace-period (reminder) timestamp
--  - stable device id the web client keeps in localStorage for push/session
--
-- All columns are nullable or default to a safe value, so no backfill is needed.

ALTER TABLE users ADD COLUMN IF NOT EXISTS chat_status VARCHAR(20) NOT NULL DEFAULT 'online'
    CHECK (chat_status IN ('online', 'busy', 'away', 'offline', 'invisible'));

ALTER TABLE users ADD COLUMN IF NOT EXISTS sidebar_prefs JSONB NOT NULL DEFAULT '{}'::jsonb;

ALTER TABLE users ADD COLUMN IF NOT EXISTS theme_settings JSONB NOT NULL DEFAULT '{}'::jsonb;

ALTER TABLE users ADD COLUMN IF NOT EXISTS pro_remainder TIMESTAMPTZ;

ALTER TABLE users ADD COLUMN IF NOT EXISTS web_device_id VARCHAR(100);

CREATE INDEX IF NOT EXISTS idx_users_chat_status
    ON users (chat_status) WHERE chat_status <> 'offline';

-- ── Pages: geolocation for nearby_business / nearby_shops queries ───────
ALTER TABLE pages ADD COLUMN IF NOT EXISTS lat DOUBLE PRECISION;
ALTER TABLE pages ADD COLUMN IF NOT EXISTS lng DOUBLE PRECISION;

CREATE INDEX IF NOT EXISTS idx_pages_geo
    ON pages (lat, lng) WHERE lat IS NOT NULL AND lng IS NOT NULL;

-- ── uploaded_media: updated_at for rotate/crop cache-busting ────────────
ALTER TABLE uploaded_media ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW();

-- ── Emojis (catalog served publicly + extended by admin) ────────────────
CREATE TABLE IF NOT EXISTS emojis (
    id          BIGSERIAL PRIMARY KEY,
    shortcode   VARCHAR(50) NOT NULL UNIQUE,
    unicode     VARCHAR(30),
    image_url   TEXT,
    category    VARCHAR(30) NOT NULL DEFAULT 'misc',
    is_custom   BOOLEAN NOT NULL DEFAULT FALSE,
    sort_order  INTEGER NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_emojis_category ON emojis (category, sort_order);

-- ── Cronjob config (enable/disable + schedule overrides) ────────────────
CREATE TABLE IF NOT EXISTS cronjob_config (
    job_name     VARCHAR(100) PRIMARY KEY,
    schedule     VARCHAR(100) NOT NULL,
    -- Human-readable schedule, e.g. "@hourly", "every 5m"; for display only.
    enabled      BOOLEAN NOT NULL DEFAULT TRUE,
    last_run_at  TIMESTAMPTZ,
    last_status  VARCHAR(20),
    description  TEXT,
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Seed the catalog with every job currently shipped in jobs-runner so the
-- admin UI can render the full list immediately.
INSERT INTO cronjob_config (job_name, schedule, description) VALUES
    ('birthdays',                    'daily 00:05',     'Wish happy birthday to users celebrating today'),
    ('events_reminder',              'daily 09:00',     'Email reminders for events happening in ≤24h'),
    ('hashtag_trending',             'every 15m',       'Rebuild trending hashtags table'),
    ('live_cleanup',                 'every 5m',        'End orphaned live streams with no active host'),
    ('pro_subscription_check',       'every 1h',        'Expire pro memberships past their end date'),
    ('stories_expiry',               'every 10m',       'Delete stories older than 24h'),
    ('publish_scheduled_posts',      'every 1m',        'Publish posts whose scheduled_at has passed'),
    ('dlq_consumer',                 'continuous',      'Drain dead-letter queue for failed domain events'),
    ('auto_delete_old_messages',     'daily 03:00',     'Delete messages per users.auto_delete_settings'),
    ('weekly_memories_digest',       'monday 08:00',    'Email users posts from this day in past years'),
    ('expire_pending_ads',           'every 1h',        'Mark ads as finished when budget or date expires'),
    ('analytics_snapshot_daily',     'daily 00:30',     'Aggregate yesterdays stats into daily_analytics'),
    ('crypto_payment_reconciliation', 'every 15m',      'Poll pending crypto payments for confirmations'),
    ('newsletter_dispatcher',        'every 5m',        'Dispatch queued newsletter emails')
ON CONFLICT (job_name) DO NOTHING;
