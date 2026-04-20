-- ═══════════════════════════════════════════════════════════════════
-- AI Credits & Multi-Provider Configuration
-- ═══════════════════════════════════════════════════════════════════

-- Per-user credits (words for text, images for image generation)
CREATE TABLE IF NOT EXISTS user_ai_credits (
    user_id           BIGINT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    words_remaining   INTEGER NOT NULL DEFAULT 0,
    images_remaining  INTEGER NOT NULL DEFAULT 0,
    words_limit       INTEGER NOT NULL DEFAULT 0,
    images_limit      INTEGER NOT NULL DEFAULT 0,
    plan              VARCHAR(32) NOT NULL DEFAULT 'free',
    reset_at          TIMESTAMPTZ NOT NULL DEFAULT (NOW() + INTERVAL '30 days'),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_user_ai_credits_reset_at
    ON user_ai_credits (reset_at)
    WHERE words_remaining < words_limit OR images_remaining < images_limit;

-- Per-use log for auditing, analytics and cost tracking
CREATE TABLE IF NOT EXISTS ai_usage_log (
    id                 BIGSERIAL PRIMARY KEY,
    user_id            BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider           VARCHAR(32) NOT NULL,          -- openai | anthropic | gemini
    kind               VARCHAR(32) NOT NULL,          -- post | blog | images | chat | describe
    tokens_used        INTEGER NOT NULL DEFAULT 0,
    images_generated   INTEGER NOT NULL DEFAULT 0,
    cost_cents         INTEGER NOT NULL DEFAULT 0,
    success            BOOLEAN NOT NULL DEFAULT TRUE,
    error_message      TEXT,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_ai_usage_log_user_created
    ON ai_usage_log (user_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_ai_usage_log_provider_created
    ON ai_usage_log (provider, created_at DESC);

-- Dynamic provider configuration editable by admins (no redeploy required)
CREATE TABLE IF NOT EXISTS ai_provider_config (
    id                 BIGSERIAL PRIMARY KEY,
    name               VARCHAR(64) UNIQUE NOT NULL,   -- openai-primary, anthropic-fallback, etc.
    provider_type      VARCHAR(32) NOT NULL,          -- openai | anthropic | gemini
    capability         VARCHAR(32) NOT NULL,          -- text | image | both
    api_key_encrypted  TEXT NOT NULL,                 -- AES-GCM encrypted
    model_text         VARCHAR(128),
    model_image        VARCHAR(128),
    enabled            BOOLEAN NOT NULL DEFAULT TRUE,
    priority           INTEGER NOT NULL DEFAULT 100,  -- lower = higher priority (fallback chain)
    extra_config       JSONB DEFAULT '{}'::jsonb,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_ai_provider_config_priority
    ON ai_provider_config (capability, priority)
    WHERE enabled = TRUE;

-- Per-plan default credit allocation (admin editable)
CREATE TABLE IF NOT EXISTS ai_plan_credits (
    plan               VARCHAR(32) PRIMARY KEY,
    words_per_cycle    INTEGER NOT NULL DEFAULT 0,
    images_per_cycle   INTEGER NOT NULL DEFAULT 0,
    cycle_days         INTEGER NOT NULL DEFAULT 30,
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO ai_plan_credits (plan, words_per_cycle, images_per_cycle, cycle_days)
VALUES
    ('free',    2000,   5,  30),
    ('pro',    20000,  50,  30),
    ('premium', 100000, 200, 30)
ON CONFLICT (plan) DO NOTHING;
