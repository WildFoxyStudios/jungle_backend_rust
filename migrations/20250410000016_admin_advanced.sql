-- Migration 016: Admin Advanced — site_ads, custom_code, permissions, auto_settings extras

-- ============================================================
-- Site Advertisements (admin-managed banners/system ads)
-- PHP: manage-site-ads, ads-settings
-- ============================================================
CREATE TABLE IF NOT EXISTS site_ads (
    id              BIGSERIAL PRIMARY KEY,
    name            VARCHAR(200) NOT NULL DEFAULT '',
    ad_type         VARCHAR(30) NOT NULL DEFAULT 'banner', -- banner, sidebar, header, footer, popup
    content         TEXT NOT NULL DEFAULT '',              -- HTML/script content
    image           TEXT DEFAULT '',
    url             TEXT DEFAULT '',
    position        VARCHAR(50) DEFAULT 'sidebar',
    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    views           INT NOT NULL DEFAULT 0,
    clicks          INT NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================
-- Custom Code (header/footer JS and CSS)
-- PHP: custom-code, update_custom_code
-- ============================================================
CREATE TABLE IF NOT EXISTS custom_code (
    id              BIGSERIAL PRIMARY KEY,
    position        VARCHAR(20) NOT NULL UNIQUE, -- header, footer
    content         TEXT NOT NULL DEFAULT '',
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO custom_code (position, content) VALUES ('header', ''), ('footer', '')
ON CONFLICT (position) DO NOTHING;

-- ============================================================
-- User Permissions (admin-assigned moderator roles)
-- PHP: manage-permissions, update_moderator_permission
-- ============================================================
CREATE TABLE IF NOT EXISTS user_permissions (
    id              BIGSERIAL PRIMARY KEY,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE UNIQUE,
    can_moderate_posts   BOOLEAN NOT NULL DEFAULT FALSE,
    can_moderate_users   BOOLEAN NOT NULL DEFAULT FALSE,
    can_moderate_reports BOOLEAN NOT NULL DEFAULT FALSE,
    can_manage_content   BOOLEAN NOT NULL DEFAULT FALSE,
    can_manage_payments  BOOLEAN NOT NULL DEFAULT FALSE,
    granted_by      BIGINT REFERENCES users(id),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================
-- Auto Settings: ensure tables have all needed columns
-- PHP: auto-friend (auto_follow), auto-join, auto-like, auto-delete
-- ============================================================
ALTER TABLE auto_follow_accounts ADD COLUMN IF NOT EXISTS is_active BOOLEAN NOT NULL DEFAULT TRUE;
ALTER TABLE auto_join_groups     ADD COLUMN IF NOT EXISTS is_active BOOLEAN NOT NULL DEFAULT TRUE;
ALTER TABLE auto_like_pages      ADD COLUMN IF NOT EXISTS is_active BOOLEAN NOT NULL DEFAULT TRUE;

-- Auto-delete settings (stored in site_config)
INSERT INTO site_config (category, key, value, value_type) VALUES
    ('auto_delete', 'enabled',                  'false',  'boolean'),
    ('auto_delete', 'delete_posts_older_days',  '365',    'integer'),
    ('auto_delete', 'delete_stories_enabled',   'true',   'boolean'),
    ('auto_delete', 'delete_inactive_users_days','730',   'integer')
ON CONFLICT (category, key) DO NOTHING;

-- ============================================================
-- Push Notification Settings (site_config entries)
-- PHP: push-notifications-system
-- ============================================================
INSERT INTO site_config (category, key, value, value_type) VALUES
    ('push',  'fcm_server_key',     '', 'string'),
    ('push',  'fcm_project_id',     '', 'string'),
    ('push',  'apns_key_id',        '', 'string'),
    ('push',  'apns_team_id',       '', 'string'),
    ('push',  'apns_bundle_id',     '', 'string'),
    ('push',  'web_push_public_key','', 'string'),
    ('push',  'web_push_private_key','','string')
ON CONFLICT (category, key) DO NOTHING;

-- ============================================================
-- AI Settings (site_config entries)
-- PHP: ai-settings
-- ============================================================
INSERT INTO site_config (category, key, value, value_type) VALUES
    ('ai', 'enabled',                'false',         'boolean'),
    ('ai', 'provider',               'openai',        'string'),
    ('ai', 'openai_api_key',         '',              'string'),
    ('ai', 'openai_model',           'gpt-4',         'string'),
    ('ai', 'google_vision_api_key',  '',              'string'),
    ('ai', 'image_generation',       'false',         'boolean'),
    ('ai', 'content_moderation',     'false',         'boolean')
ON CONFLICT (category, key) DO NOTHING;

-- ============================================================
-- Store/Marketplace Settings (site_config entries)
-- PHP: store-settings
-- ============================================================
INSERT INTO site_config (category, key, value, value_type) VALUES
    ('store', 'commission_rate',          '0',      'decimal'),
    ('store', 'min_product_price',        '0',      'decimal'),
    ('store', 'max_product_price',        '99999',  'decimal'),
    ('store', 'allow_digital_products',   'true',   'boolean'),
    ('store', 'shipping_enabled',         'false',  'boolean'),
    ('store', 'tax_rate',                 '0',      'decimal')
ON CONFLICT (category, key) DO NOTHING;

-- ============================================================
-- Affiliates/Referrals Settings (site_config entries)
-- PHP: affiliates-settings
-- ============================================================
INSERT INTO site_config (category, key, value, value_type) VALUES
    ('affiliates', 'enabled',         'false',  'boolean'),
    ('affiliates', 'commission_type', 'fixed',  'string'),
    ('affiliates', 'commission_value','0',      'decimal'),
    ('affiliates', 'min_withdrawal',  '10',     'decimal'),
    ('affiliates', 'per_user',        '0',      'decimal')
ON CONFLICT (category, key) DO NOTHING;

-- ============================================================
-- Pro Settings (site_config entries)
-- PHP: pro-settings, pro-features
-- ============================================================
INSERT INTO site_config (category, key, value, value_type) VALUES
    ('pro', 'enabled',                      'false', 'boolean'),
    ('pro', 'monthly_price',                '9.99',  'decimal'),
    ('pro', 'yearly_price',                 '99.99', 'decimal'),
    ('pro', 'feature_no_ads',               'true',  'boolean'),
    ('pro', 'feature_verified_badge',       'true',  'boolean'),
    ('pro', 'feature_boosted_posts',        'true',  'boolean'),
    ('pro', 'feature_extra_storage',        'true',  'boolean'),
    ('pro', 'feature_analytics',            'false', 'boolean')
ON CONFLICT (category, key) DO NOTHING;

-- ============================================================
-- Website Mode (site_config entries)
-- PHP: website_mode
-- ============================================================
INSERT INTO site_config (category, key, value, value_type) VALUES
    ('website_mode', 'active_mode',         'social',     'string'),
    ('website_mode', 'linkedin_mode',       'false',      'boolean'),
    ('website_mode', 'instagram_mode',      'false',      'boolean'),
    ('website_mode', 'patreon_mode',        'false',      'boolean'),
    ('website_mode', 'twitter_mode',        'false',      'boolean'),
    ('website_mode', 'dating_mode',         'false',      'boolean')
ON CONFLICT (category, key) DO NOTHING;

-- ============================================================
-- Ads System Settings (site_config entries)
-- PHP: ads-settings, ads-countries
-- ============================================================
INSERT INTO site_config (category, key, value, value_type) VALUES
    ('ads', 'system_enabled',       'false',  'boolean'),
    ('ads', 'currency',             'USD',    'string'),
    ('ads', 'min_budget',           '1',      'decimal'),
    ('ads', 'cpc_rate',             '0.01',   'decimal'),
    ('ads', 'cpm_rate',             '0.001',  'decimal'),
    ('ads', 'country_targeting',    'false',  'boolean'),
    ('ads', 'admob_enabled',        'false',  'boolean'),
    ('ads', 'admob_app_id',         '',       'string'),
    ('ads', 'admob_banner_id',      '',       'string'),
    ('ads', 'admob_interstitial_id','',       'string'),
    ('ads', 'admob_rewarded_id',    '',       'string')
ON CONFLICT (category, key) DO NOTHING;

-- ============================================================
-- Agora Video/Audio Call Settings (site_config entries)
-- PHP: agora.php
-- ============================================================
INSERT INTO site_config (category, key, value, value_type) VALUES
    ('agora', 'enabled',           'false', 'boolean'),
    ('agora', 'app_id',            '',      'string'),
    ('agora', 'app_certificate',   '',      'string'),
    ('agora', 'customer_key',      '',      'string'),
    ('agora', 'customer_secret',   '',      'string')
ON CONFLICT (category, key) DO NOTHING;

-- Movies needs CREATE capability (add missing columns)
ALTER TABLE movies ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ DEFAULT NOW();
ALTER TABLE movies ADD COLUMN IF NOT EXISTS is_featured BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE movies ADD COLUMN IF NOT EXISTS category_id BIGINT REFERENCES categories(id);

-- Forum sections/forums need delete tracking
ALTER TABLE forum_sections ADD COLUMN IF NOT EXISTS created_at TIMESTAMPTZ NOT NULL DEFAULT NOW();
ALTER TABLE forum_sections ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW();
ALTER TABLE forums ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW();
ALTER TABLE forums ADD COLUMN IF NOT EXISTS is_active BOOLEAN NOT NULL DEFAULT TRUE;
