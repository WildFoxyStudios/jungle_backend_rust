-- Migration 009: Commerce (Products, Orders, Jobs, Funding, Offers)

-- Products
CREATE TABLE products (
    id              BIGSERIAL PRIMARY KEY,
    uuid            UUID NOT NULL DEFAULT gen_random_uuid() UNIQUE,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    page_id         BIGINT REFERENCES pages(id),
    name            VARCHAR(200) NOT NULL,
    description     TEXT DEFAULT '',
    category_id     BIGINT REFERENCES categories(id),
    price           DECIMAL(15,2) NOT NULL DEFAULT 0,
    currency        VARCHAR(10) DEFAULT 'USD',
    location        VARCHAR(200) DEFAULT '',
    condition       VARCHAR(20) DEFAULT 'new',
    type            VARCHAR(20) DEFAULT 'sell',
    status          VARCHAR(20) DEFAULT 'active',
    media           JSONB DEFAULT '[]'::jsonb,
    units           INT NOT NULL DEFAULT 1,
    rating          DECIMAL(3,2) DEFAULT 0,
    review_count    INT NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_products_user ON products(user_id);
CREATE INDEX idx_products_category ON products(category_id) WHERE status = 'active';
CREATE INDEX idx_products_status ON products(status, created_at DESC);

-- Product reviews
CREATE TABLE product_reviews (
    id              BIGSERIAL PRIMARY KEY,
    product_id      BIGINT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    rating          SMALLINT NOT NULL CHECK (rating BETWEEN 1 AND 5),
    text            TEXT DEFAULT '',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(product_id, user_id)
);

-- Orders
CREATE TABLE orders (
    id              BIGSERIAL PRIMARY KEY,
    uuid            UUID NOT NULL DEFAULT gen_random_uuid() UNIQUE,
    buyer_id        BIGINT NOT NULL REFERENCES users(id),
    seller_id       BIGINT NOT NULL REFERENCES users(id),
    product_id      BIGINT NOT NULL REFERENCES products(id),
    quantity        INT NOT NULL DEFAULT 1,
    total_price     DECIMAL(15,2) NOT NULL,
    status          VARCHAR(20) NOT NULL DEFAULT 'pending',
    address         JSONB,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_orders_buyer ON orders(buyer_id, created_at DESC);
CREATE INDEX idx_orders_seller ON orders(seller_id, created_at DESC);

-- Jobs
CREATE TABLE jobs (
    id              BIGSERIAL PRIMARY KEY,
    uuid            UUID NOT NULL DEFAULT gen_random_uuid() UNIQUE,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    page_id         BIGINT REFERENCES pages(id),
    title           VARCHAR(200) NOT NULL,
    description     TEXT,
    location        VARCHAR(200) DEFAULT '',
    lat             DOUBLE PRECISION,
    lng             DOUBLE PRECISION,
    salary_min      DECIMAL(15,2) DEFAULT 0,
    salary_max      DECIMAL(15,2) DEFAULT 0,
    salary_period   VARCHAR(20) DEFAULT 'monthly',
    job_type        VARCHAR(30) DEFAULT 'full_time',
    category_id     BIGINT REFERENCES categories(id),
    image           TEXT DEFAULT '',
    currency        VARCHAR(10) DEFAULT 'USD',
    questions       JSONB DEFAULT '[]'::jsonb,
    status          VARCHAR(20) DEFAULT 'active',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_jobs_status ON jobs(status, created_at DESC);

-- Job applications
CREATE TABLE job_applications (
    id              BIGSERIAL PRIMARY KEY,
    job_id          BIGINT NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    answers         JSONB DEFAULT '{}'::jsonb,
    cover_letter    TEXT DEFAULT '',
    resume_url      TEXT DEFAULT '',
    status          VARCHAR(20) DEFAULT 'pending',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(job_id, user_id)
);

-- Funding / Crowdfunding
CREATE TABLE fundings (
    id              BIGSERIAL PRIMARY KEY,
    uuid            UUID NOT NULL DEFAULT gen_random_uuid() UNIQUE,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title           VARCHAR(200) NOT NULL,
    description     TEXT,
    goal_amount     DECIMAL(15,2) NOT NULL,
    raised_amount   DECIMAL(15,2) NOT NULL DEFAULT 0,
    image           TEXT DEFAULT '',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Funding donations
CREATE TABLE funding_donations (
    id              BIGSERIAL PRIMARY KEY,
    funding_id      BIGINT NOT NULL REFERENCES fundings(id) ON DELETE CASCADE,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    amount          DECIMAL(15,2) NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Offers
CREATE TABLE offers (
    id              BIGSERIAL PRIMARY KEY,
    uuid            UUID NOT NULL DEFAULT gen_random_uuid() UNIQUE,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    page_id         BIGINT REFERENCES pages(id),
    title           VARCHAR(200) NOT NULL,
    description     TEXT,
    image           TEXT DEFAULT '',
    discount_type   VARCHAR(20) DEFAULT 'percentage',
    discount_value  DECIMAL(15,2) NOT NULL,
    currency        VARCHAR(10) DEFAULT 'USD',
    expires_at      TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
