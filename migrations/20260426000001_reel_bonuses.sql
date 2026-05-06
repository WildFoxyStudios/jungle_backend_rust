-- Reel Bonuses: Creator monetization for reels
CREATE TABLE IF NOT EXISTS reel_bonus_pools (
    id BIGSERIAL PRIMARY KEY,
    period_start TIMESTAMPTZ NOT NULL,
    period_end TIMESTAMPTZ NOT NULL,
    total_budget NUMERIC(12,2) NOT NULL DEFAULT 0,
    total_views BIGINT NOT NULL DEFAULT 0,
    status VARCHAR(20) NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'calculated', 'paid')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    calculated_at TIMESTAMPTZ,
    paid_at TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS reel_earnings (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    pool_id BIGINT NOT NULL REFERENCES reel_bonus_pools(id) ON DELETE CASCADE,
    reel_id BIGINT REFERENCES posts(id) ON DELETE SET NULL,
    views_count BIGINT NOT NULL DEFAULT 0,
    earnings_amount NUMERIC(12,4) NOT NULL DEFAULT 0,
    status VARCHAR(20) NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'paid')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    paid_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_reel_earnings_user ON reel_earnings(user_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_reel_earnings_pool ON reel_earnings(pool_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_reel_earnings_unique ON reel_earnings(user_id, pool_id, COALESCE(reel_id, 0));

-- Config entries for reel bonuses
INSERT INTO site_config (category, key, value) VALUES
    ('monetization', 'reel_bonus_enabled', '1'),
    ('monetization', 'reel_bonus_min_views', '1000'),
    ('monetization', 'reel_bonus_period_days', '30'),
    ('monetization', 'reel_bonus_cpm', '2.50')
ON CONFLICT (category, key) DO NOTHING;
