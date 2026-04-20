-- ═══════════════════════════════════════════════════════════════════
-- Endpoints gap migration: Batch 4
-- Adds columns and tables required by newly added XHR-equivalent endpoints
-- ═══════════════════════════════════════════════════════════════════

-- ── Posts: comment toggle, sold flag, product link, URL preview, video views, wonder reactions ──
ALTER TABLE posts ADD COLUMN IF NOT EXISTS comments_status SMALLINT NOT NULL DEFAULT 0;
ALTER TABLE posts ADD COLUMN IF NOT EXISTS is_sold BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE posts ADD COLUMN IF NOT EXISTS url_preview JSONB;
ALTER TABLE posts ADD COLUMN IF NOT EXISTS video_views INTEGER NOT NULL DEFAULT 0;
ALTER TABLE posts ADD COLUMN IF NOT EXISTS wonder_count INTEGER NOT NULL DEFAULT 0;

-- product_id references products(id) only if the products table exists
DO $$ BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'products')
       AND NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'posts' AND column_name = 'product_id') THEN
        ALTER TABLE posts ADD COLUMN product_id BIGINT REFERENCES products(id) ON DELETE SET NULL;
    END IF;
END $$;

-- ── Calls: richer lifecycle metadata ──
ALTER TABLE calls ADD COLUMN IF NOT EXISTS answered_at TIMESTAMPTZ;
-- ended_at already exists; skip
ALTER TABLE calls ADD COLUMN IF NOT EXISTS duration_seconds INTEGER;
ALTER TABLE calls ADD COLUMN IF NOT EXISTS end_reason VARCHAR(20);

-- ── URL preview cache (per-URL Open Graph / oEmbed results) ──
CREATE TABLE IF NOT EXISTS url_preview_cache (
    url_hash        VARCHAR(64) PRIMARY KEY,  -- sha256 hex of the URL
    url             TEXT NOT NULL,
    title           TEXT,
    description     TEXT,
    image_url       TEXT,
    site_name       VARCHAR(128),
    embed_html      TEXT,                     -- for YouTube/Vimeo oEmbed
    fetched_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_url_preview_fetched
    ON url_preview_cache (fetched_at DESC);

-- ── Video view dedup (rate-limit counts per IP/user per post) ──
CREATE TABLE IF NOT EXISTS video_views_dedup (
    id              BIGSERIAL PRIMARY KEY,
    post_id         BIGINT NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    user_id         BIGINT REFERENCES users(id) ON DELETE SET NULL,
    ip_hash         VARCHAR(64),               -- sha256(ip_address) for privacy
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (post_id, user_id, ip_hash)
);

CREATE INDEX IF NOT EXISTS idx_video_views_dedup_post_created
    ON video_views_dedup (post_id, created_at DESC);

-- ── Storage providers (Batch 4: storage config CRUD for admin) ──
CREATE TABLE IF NOT EXISTS storage_providers (
    id                BIGSERIAL PRIMARY KEY,
    name              VARCHAR(64) UNIQUE NOT NULL,
    provider_type     VARCHAR(32) NOT NULL,      -- s3 | r2 | minio | wasabi | spaces | b2
    bucket            VARCHAR(128) NOT NULL,
    endpoint          VARCHAR(255),
    region            VARCHAR(64) DEFAULT 'auto',
    access_key        VARCHAR(255) NOT NULL,
    secret_key_encrypted TEXT NOT NULL,
    public_url        VARCHAR(255),
    is_active         BOOLEAN NOT NULL DEFAULT TRUE,
    priority          INTEGER NOT NULL DEFAULT 100,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_storage_providers_priority
    ON storage_providers (priority)
    WHERE is_active = TRUE;

-- ── Admin permissions catalog (reference list of granular actions) ──
-- We use a configuration table so admins can customize the list later.
CREATE TABLE IF NOT EXISTS admin_permissions_catalog (
    key         VARCHAR(64) PRIMARY KEY,
    description TEXT NOT NULL,
    category    VARCHAR(32) NOT NULL
);

INSERT INTO admin_permissions_catalog (key, description, category) VALUES
    ('view_dashboard',    'View admin dashboard',                   'general'),
    ('manage_users',      'Create, edit, delete, ban users',        'users'),
    ('view_users',        'View user list and details',             'users'),
    ('verify_users',      'Approve/reject verification requests',   'users'),
    ('impersonate_users', 'Log in as another user',                 'users'),
    ('manage_posts',      'Delete / hide any post',                 'content'),
    ('manage_comments',   'Delete / hide any comment',              'content'),
    ('moderate_reports',  'Resolve user-submitted reports',         'moderation'),
    ('manage_groups',     'Delete or edit groups',                  'communities'),
    ('manage_pages',      'Delete or edit pages',                   'communities'),
    ('manage_events',     'Delete or edit events',                  'communities'),
    ('manage_products',   'Edit / remove marketplace products',     'commerce'),
    ('manage_jobs',       'Edit / remove job posts',                'commerce'),
    ('manage_ads',        'Approve / reject advertising campaigns', 'commerce'),
    ('manage_payments',   'Refunds, withdrawals, disputes',         'payments'),
    ('manage_settings',   'Edit site configuration',                'settings'),
    ('manage_email_templates', 'Edit email templates',              'settings'),
    ('manage_translations', 'Edit i18n strings',                    'settings'),
    ('manage_themes',     'Edit site appearance / themes',          'settings'),
    ('manage_ai',         'Configure AI providers',                 'settings'),
    ('manage_storage',    'Configure upload storage providers',     'settings'),
    ('manage_oauth',      'Manage developer OAuth apps',            'settings'),
    ('view_audit_log',    'View admin audit log',                   'security'),
    ('view_activity_log', 'View user activity log',                 'security'),
    ('manage_api_keys',   'Rotate API keys',                        'security'),
    ('view_health',       'View system health / metrics',           'system'),
    ('trigger_backup',    'Manually trigger backups',               'system'),
    ('send_newsletter',   'Send broadcast emails',                  'system'),
    ('manage_cronjobs',   'Enable/disable scheduled jobs',          'system'),
    ('manage_dlq',        'Retry or discard failed events',         'system')
ON CONFLICT (key) DO NOTHING;
