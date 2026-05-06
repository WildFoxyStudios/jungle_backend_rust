-- ============================================================================
-- Phase 13: Advanced Marketplace
-- ============================================================================

-- Saved products
CREATE TABLE IF NOT EXISTS saved_products (
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    product_id BIGINT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    saved_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, product_id)
);

-- Price alerts
CREATE TABLE IF NOT EXISTS product_price_alerts (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    product_id BIGINT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    threshold_cents BIGINT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    last_triggered_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Shipping zones
CREATE TABLE IF NOT EXISTS shipping_zones (
    id BIGSERIAL PRIMARY KEY,
    seller_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name VARCHAR(128) NOT NULL,
    countries JSONB NOT NULL DEFAULT '[]',
    is_active BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE TABLE IF NOT EXISTS shipping_rates (
    id BIGSERIAL PRIMARY KEY,
    zone_id BIGINT NOT NULL REFERENCES shipping_zones(id) ON DELETE CASCADE,
    weight_min_g INT NOT NULL DEFAULT 0,
    weight_max_g INT,
    price_cents BIGINT NOT NULL,
    estimated_days INT
);

-- Order disputes
CREATE TABLE IF NOT EXISTS order_disputes (
    id BIGSERIAL PRIMARY KEY,
    order_id BIGINT NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
    buyer_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    reason TEXT NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'under_review', 'resolved_buyer', 'resolved_seller', 'closed')),
    admin_notes TEXT,
    resolved_by BIGINT REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    resolved_at TIMESTAMPTZ
);

-- Enhanced reviews
ALTER TABLE product_reviews ADD COLUMN IF NOT EXISTS photos JSONB DEFAULT '[]';
ALTER TABLE product_reviews ADD COLUMN IF NOT EXISTS helpful_votes INT NOT NULL DEFAULT 0;
ALTER TABLE product_reviews ADD COLUMN IF NOT EXISTS is_verified_buyer BOOLEAN NOT NULL DEFAULT FALSE;

-- ============================================================================
-- Phase 14: Jobs ATS-Lite
-- ============================================================================

-- Saved jobs
CREATE TABLE IF NOT EXISTS saved_jobs (
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    job_id BIGINT NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
    saved_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, job_id)
);

-- Job alerts
CREATE TABLE IF NOT EXISTS job_alerts (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    query TEXT,
    filters JSONB DEFAULT '{}',
    frequency VARCHAR(20) NOT NULL DEFAULT 'weekly' CHECK (frequency IN ('daily', 'weekly')),
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    last_sent_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Resume upload
CREATE TABLE IF NOT EXISTS user_resumes (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    file_url TEXT NOT NULL,
    file_name VARCHAR(256),
    extracted_text TEXT,
    skills JSONB DEFAULT '[]',
    experience_years INT,
    uploaded_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    is_active BOOLEAN NOT NULL DEFAULT TRUE
);

-- ============================================================================
-- Phase 15: Watch + Long Video
-- ============================================================================

-- Watch progress
CREATE TABLE IF NOT EXISTS watch_progress (
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    movie_id BIGINT NOT NULL REFERENCES movies(id) ON DELETE CASCADE,
    position_ms BIGINT NOT NULL DEFAULT 0,
    duration_ms BIGINT,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, movie_id)
);

-- Watch later
CREATE TABLE IF NOT EXISTS watch_later (
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    movie_id BIGINT NOT NULL REFERENCES movies(id) ON DELETE CASCADE,
    saved_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, movie_id)
);

-- Video chapters
CREATE TABLE IF NOT EXISTS movie_chapters (
    id BIGSERIAL PRIMARY KEY,
    movie_id BIGINT NOT NULL REFERENCES movies(id) ON DELETE CASCADE,
    sort_order INT NOT NULL DEFAULT 0,
    title VARCHAR(256) NOT NULL,
    start_ms BIGINT NOT NULL,
    end_ms BIGINT
);
