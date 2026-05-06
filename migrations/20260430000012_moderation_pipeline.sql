-- Phase 1: OpenAI Moderation Pipeline — extend existing moderation_queue
-- Fase 1 antes se saltó por falta de API key. Ahora con OPENAI_API_KEY disponible.

-- Extend moderation_queue with AI fields
ALTER TABLE moderation_queue ADD COLUMN IF NOT EXISTS content_text TEXT;
ALTER TABLE moderation_queue ADD COLUMN IF NOT EXISTS content_image_url TEXT;
ALTER TABLE moderation_queue ADD COLUMN IF NOT EXISTS openai_flagged BOOLEAN;
ALTER TABLE moderation_queue ADD COLUMN IF NOT EXISTS openai_categories JSONB;
ALTER TABLE moderation_queue ADD COLUMN IF NOT EXISTS openai_scores JSONB;
ALTER TABLE moderation_queue ADD COLUMN IF NOT EXISTS auto_action VARCHAR(20) CHECK (auto_action IN ('auto_approve', 'human_review', 'auto_block', 'csam_block'));
ALTER TABLE moderation_queue ADD COLUMN IF NOT EXISTS resolved_at TIMESTAMPTZ;
ALTER TABLE moderation_queue ADD COLUMN IF NOT EXISTS resolved_by BIGINT REFERENCES users(id);

-- Daily moderation metrics
CREATE TABLE IF NOT EXISTS moderation_metrics_daily (
    date DATE NOT NULL,
    total_processed INT NOT NULL DEFAULT 0,
    auto_approved INT NOT NULL DEFAULT 0,
    auto_blocked INT NOT NULL DEFAULT 0,
    human_reviewed INT NOT NULL DEFAULT 0,
    false_positives INT NOT NULL DEFAULT 0,
    PRIMARY KEY (date)
);

-- Content hashes for re-identification of previously blocked content
CREATE TABLE IF NOT EXISTS content_hashes (
    id BIGSERIAL PRIMARY KEY,
    hash VARCHAR(64) NOT NULL,
    content_type VARCHAR(20) NOT NULL,
    action VARCHAR(20) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_content_hashes_hash ON content_hashes(hash);

-- Banned terms (regex patterns stored as text)
CREATE TABLE IF NOT EXISTS banned_terms (
    id BIGSERIAL PRIMARY KEY,
    pattern TEXT NOT NULL,
    category VARCHAR(64),
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_by BIGINT REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
