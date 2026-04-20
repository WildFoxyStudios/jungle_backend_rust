-- ═══════════════════════════════════════════════════════════════════
-- Newsletter queue + cronjob run history
-- ═══════════════════════════════════════════════════════════════════
--
-- `newsletter_queue` buffers bulk emails created by admin campaigns and
-- consumed by the `newsletter_dispatcher` job in `jobs-runner`.
--
-- `cronjob_runs` records the outcome of each cron invocation so the
-- admin UI can surface freshness + health per job.

CREATE TABLE IF NOT EXISTS newsletter_queue (
    id                BIGSERIAL PRIMARY KEY,
    campaign_id       BIGINT,
    recipient_email   VARCHAR(254) NOT NULL,
    recipient_user_id BIGINT REFERENCES users(id) ON DELETE SET NULL,
    subject           VARCHAR(255) NOT NULL,
    body              TEXT NOT NULL,
    status            VARCHAR(20) NOT NULL DEFAULT 'pending',
    -- 'pending' | 'sent' | 'failed'
    attempts          INTEGER NOT NULL DEFAULT 0,
    error_message     TEXT,
    sent_at           TIMESTAMPTZ,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_newsletter_queue_pending
    ON newsletter_queue (status, attempts) WHERE status = 'pending';
CREATE INDEX IF NOT EXISTS idx_newsletter_queue_user
    ON newsletter_queue (recipient_user_id);

CREATE TABLE IF NOT EXISTS cronjob_runs (
    id         BIGSERIAL PRIMARY KEY,
    name       VARCHAR(100) NOT NULL,
    status     VARCHAR(20)  NOT NULL,
    -- 'healthy' | 'warning' | 'error'
    message    TEXT,
    duration_ms INTEGER,
    ran_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_cronjob_runs_name_time ON cronjob_runs(name, ran_at DESC);
