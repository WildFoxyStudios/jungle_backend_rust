-- ═══════════════════════════════════════════════════════════════════
-- Admin audit log + Event bus dead-letter storage
-- ═══════════════════════════════════════════════════════════════════

-- Admin audit log: every mutating action by an admin user is recorded
CREATE TABLE IF NOT EXISTS admin_audit_log (
    id              BIGSERIAL PRIMARY KEY,
    admin_user_id   BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    action          VARCHAR(64) NOT NULL,          -- http method: POST|PUT|PATCH|DELETE
    resource_type   VARCHAR(64) NOT NULL,          -- the /v1/admin/<type>/... prefix
    resource_id     VARCHAR(128),                  -- path id or "-"
    endpoint        TEXT NOT NULL,                 -- full path
    status          INTEGER NOT NULL,              -- HTTP response status
    changes         JSONB,                         -- request body (sanitized)
    ip_address      INET,
    user_agent      TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_admin_audit_admin_user
    ON admin_audit_log (admin_user_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_admin_audit_resource
    ON admin_audit_log (resource_type, resource_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_admin_audit_created_at
    ON admin_audit_log (created_at DESC);

-- Event bus dead-letter queue (NATS retry exhausted)
CREATE TABLE IF NOT EXISTS event_dlq (
    id          BIGSERIAL PRIMARY KEY,
    subject     VARCHAR(128) NOT NULL,          -- e.g. dlq.events.post.created
    payload     JSONB NOT NULL,                 -- original event data
    error       TEXT,
    attempt     INTEGER NOT NULL DEFAULT 0,
    retry_at    TIMESTAMPTZ,                    -- if null → manual retry only
    consumed_at TIMESTAMPTZ,                    -- set when admin retries or discards
    consumed_by BIGINT REFERENCES users(id),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_event_dlq_unconsumed
    ON event_dlq (created_at DESC)
    WHERE consumed_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_event_dlq_subject
    ON event_dlq (subject, created_at DESC);
