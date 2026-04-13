-- Migration: Missing PHP tables not yet ported to PostgreSQL
-- Source: Wo_UserAddress, Wo_UserCard, Wo_Relationship, Wo_PinnedPosts,
--         Wo_Sub_Categories, Wo_Gender, Wo_Albums_Media, Wo_MovieComments,
--         Wo_MovieCommentReplies, Wo_Pages_Invites, Wo_MonetizationSubscription,
--         Wo_Live_Sub_Users, Wo_UserAds_Data, Wo_Refund, Wo_PendingPayments,
--         Wo_Purchases, Wo_Terms, Wo_UserOpenTo, Wo_UserLanguages,
--         Wo_BlogMovieLikes/Wo_BlogMovieDisLikes, Wo_Codes, bank_receipts,
--         Wo_Emails, Wo_Apps/Wo_AppsSessions/Wo_Apps_Hash/Wo_Apps_Permission,
--         Wo_AgoraVideoCall, Wo_AudioCalls, Wo_VideoCalles, Wo_PatreonSubscribers,
--         Wo_Manage_Pro, Wo_UserTiers, wondertage_settings

-- ── User Addresses (Wo_UserAddress) ─────────────────────────────────────────
CREATE TABLE IF NOT EXISTS user_addresses (
    id          BIGSERIAL PRIMARY KEY,
    user_id     BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name        VARCHAR(100) NOT NULL DEFAULT '',
    phone       VARCHAR(50) NOT NULL DEFAULT '',
    country     VARCHAR(100) NOT NULL DEFAULT '',
    city        VARCHAR(100) NOT NULL DEFAULT '',
    zip         VARCHAR(20) NOT NULL DEFAULT '',
    address     TEXT NOT NULL DEFAULT '',
    is_default  BOOLEAN NOT NULL DEFAULT FALSE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_user_addresses_user ON user_addresses(user_id);

-- ── Shopping Cart (Wo_UserCard) ─────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS shopping_cart (
    id          BIGSERIAL PRIMARY KEY,
    user_id     BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    product_id  BIGINT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    units       INT NOT NULL DEFAULT 1,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, product_id)
);
CREATE INDEX IF NOT EXISTS idx_shopping_cart_user ON shopping_cart(user_id);

-- ── Relationships (Wo_Relationship) ─────────────────────────────────────────
-- relationship: 1=single, 2=in_relationship, 3=married, 4=engaged, 5=complicated
CREATE TABLE IF NOT EXISTS relationships (
    id              BIGSERIAL PRIMARY KEY,
    from_user_id    BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    to_user_id      BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    relationship    SMALLINT NOT NULL DEFAULT 0,
    is_accepted     BOOLEAN NOT NULL DEFAULT FALSE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_relationships_from ON relationships(from_user_id);
CREATE INDEX IF NOT EXISTS idx_relationships_to ON relationships(to_user_id);

-- ── Pinned Posts (Wo_PinnedPosts) ───────────────────────────────────────────
CREATE TABLE IF NOT EXISTS pinned_posts (
    id          BIGSERIAL PRIMARY KEY,
    user_id     BIGINT REFERENCES users(id) ON DELETE CASCADE,
    page_id     BIGINT REFERENCES pages(id) ON DELETE CASCADE,
    group_id    BIGINT REFERENCES groups(id) ON DELETE CASCADE,
    event_id    BIGINT REFERENCES events(id) ON DELETE CASCADE,
    post_id     BIGINT NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    is_active   BOOLEAN NOT NULL DEFAULT TRUE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_pinned_posts_post ON pinned_posts(post_id);

-- ── Sub-Categories (Wo_Sub_Categories) ──────────────────────────────────────
CREATE TABLE IF NOT EXISTS sub_categories (
    id          BIGSERIAL PRIMARY KEY,
    category_id BIGINT NOT NULL REFERENCES categories(id) ON DELETE CASCADE,
    lang_key    VARCHAR(200) NOT NULL DEFAULT '',
    type        VARCHAR(200) NOT NULL DEFAULT '',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_sub_categories_category ON sub_categories(category_id);

-- ── Genders (Wo_Gender) ────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS genders (
    id          BIGSERIAL PRIMARY KEY,
    gender_id   VARCHAR(50) NOT NULL DEFAULT '0',
    name        VARCHAR(100) NOT NULL DEFAULT '',
    image       VARCHAR(300) NOT NULL DEFAULT ''
);

-- ── Albums Media (Wo_Albums_Media) ──────────────────────────────────────────
CREATE TABLE IF NOT EXISTS albums_media (
    id          BIGSERIAL PRIMARY KEY,
    post_id     BIGINT NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    user_id     BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    image       TEXT NOT NULL DEFAULT '',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_albums_media_post ON albums_media(post_id);
CREATE INDEX IF NOT EXISTS idx_albums_media_user ON albums_media(user_id);

-- ── Movie Comments (Wo_MovieComments + Wo_MovieCommentReplies) ──────────────
CREATE TABLE IF NOT EXISTS movie_comments (
    id          BIGSERIAL PRIMARY KEY,
    movie_id    BIGINT NOT NULL REFERENCES movies(id) ON DELETE CASCADE,
    user_id     BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    parent_id   BIGINT REFERENCES movie_comments(id) ON DELETE CASCADE,
    text        TEXT NOT NULL DEFAULT '',
    likes       INT NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_movie_comments_movie ON movie_comments(movie_id);
CREATE INDEX IF NOT EXISTS idx_movie_comments_parent ON movie_comments(parent_id);

-- ── Page Invites (Wo_Pages_Invites) ────────────────────────────────────────
CREATE TABLE IF NOT EXISTS page_invites (
    id          BIGSERIAL PRIMARY KEY,
    page_id     BIGINT NOT NULL REFERENCES pages(id) ON DELETE CASCADE,
    inviter_id  BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    invited_id  BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(page_id, invited_id)
);
CREATE INDEX IF NOT EXISTS idx_page_invites_invited ON page_invites(invited_id);

-- ── Creator/Monetization Subscriptions (Wo_MonetizationSubscription + Wo_PatreonSubscribers) ─
CREATE TABLE IF NOT EXISTS monetization_subscriptions (
    id                  BIGSERIAL PRIMARY KEY,
    subscriber_id       BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    creator_id          BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    tier_id             BIGINT REFERENCES creator_tiers(id) ON DELETE SET NULL,
    status              SMALLINT NOT NULL DEFAULT 1,
    last_payment_date   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at          TIMESTAMPTZ,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_monetization_subs_subscriber ON monetization_subscriptions(subscriber_id);
CREATE INDEX IF NOT EXISTS idx_monetization_subs_creator ON monetization_subscriptions(creator_id);

-- ── Live Streams (needed before live_viewers FK) ──────────────────────────
CREATE TABLE IF NOT EXISTS live_streams (
    id           BIGSERIAL PRIMARY KEY,
    user_id      BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title        VARCHAR(255) NOT NULL DEFAULT '',
    stream_key   VARCHAR(100) NOT NULL DEFAULT '',
    status       VARCHAR(20) NOT NULL DEFAULT 'live',
    viewer_count INT NOT NULL DEFAULT 0,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ended_at     TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS idx_live_streams_status ON live_streams(status);
CREATE INDEX IF NOT EXISTS idx_live_streams_user ON live_streams(user_id);

-- ── Live Stream Viewers (Wo_Live_Sub_Users) ────────────────────────────────
CREATE TABLE IF NOT EXISTS live_viewers (
    id          BIGSERIAL PRIMARY KEY,
    user_id     BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    stream_id   BIGINT NOT NULL REFERENCES live_streams(id) ON DELETE CASCADE,
    is_watching BOOLEAN NOT NULL DEFAULT TRUE,
    joined_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_live_viewers_stream ON live_viewers(stream_id);

-- ── Ad Analytics (Wo_UserAds_Data) ─────────────────────────────────────────
CREATE TABLE IF NOT EXISTS ad_analytics (
    id          BIGSERIAL PRIMARY KEY,
    user_id     BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    ad_id       BIGINT NOT NULL REFERENCES user_ads(id) ON DELETE CASCADE,
    clicks      INT NOT NULL DEFAULT 0,
    views       INT NOT NULL DEFAULT 0,
    spend       NUMERIC(12,2) NOT NULL DEFAULT 0,
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_ad_analytics_ad ON ad_analytics(ad_id);
CREATE INDEX IF NOT EXISTS idx_ad_analytics_date ON ad_analytics(recorded_at);

-- ── Refund Requests (Wo_Refund) ────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS refund_requests (
    id              BIGSERIAL PRIMARY KEY,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    order_hash_id   VARCHAR(100) NOT NULL DEFAULT '',
    pro_type        VARCHAR(50) NOT NULL DEFAULT '',
    description     TEXT,
    status          SMALLINT NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_refund_requests_user ON refund_requests(user_id);

-- ── Pending Payments (Wo_PendingPayments) ──────────────────────────────────
CREATE TABLE IF NOT EXISTS pending_payments (
    id              BIGSERIAL PRIMARY KEY,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    payment_data    JSONB NOT NULL DEFAULT '{}',
    method_name     VARCHAR(100) NOT NULL DEFAULT '',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_pending_payments_user ON pending_payments(user_id);

-- ── Purchases (Wo_Purchases) ───────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS purchases (
    id              BIGSERIAL PRIMARY KEY,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    owner_id        BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    order_hash_id   VARCHAR(100) NOT NULL DEFAULT '',
    data            JSONB,
    final_price     NUMERIC(12,2) NOT NULL DEFAULT 0,
    commission      NUMERIC(12,2) NOT NULL DEFAULT 0,
    price           NUMERIC(12,2) NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_purchases_user ON purchases(user_id);
CREATE INDEX IF NOT EXISTS idx_purchases_owner ON purchases(owner_id);

-- ── Terms / Legal Pages (Wo_Terms) ─────────────────────────────────────────
CREATE TABLE IF NOT EXISTS terms_pages (
    id      BIGSERIAL PRIMARY KEY,
    type    VARCHAR(32) NOT NULL UNIQUE,
    text    TEXT NOT NULL DEFAULT ''
);
INSERT INTO terms_pages (type, text) VALUES
    ('terms', ''),
    ('privacy', ''),
    ('about', ''),
    ('community', ''),
    ('cookies', '')
ON CONFLICT (type) DO NOTHING;

-- ── User Open-To (Wo_UserOpenTo) ───────────────────────────────────────────
CREATE TABLE IF NOT EXISTS user_open_to (
    id      BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    value   VARCHAR(200) NOT NULL DEFAULT ''
);
CREATE INDEX IF NOT EXISTS idx_user_open_to_user ON user_open_to(user_id);

-- ── User Languages (Wo_UserLanguages) ──────────────────────────────────────
CREATE TABLE IF NOT EXISTS user_languages (
    id      BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    language VARCHAR(100) NOT NULL DEFAULT ''
);
CREATE INDEX IF NOT EXISTS idx_user_languages_user ON user_languages(user_id);

-- ── Blog/Movie Likes (Wo_BlogMovieLikes + Wo_BlogMovieDisLikes) ────────────
-- Consolidated into a single table with is_like boolean
CREATE TABLE IF NOT EXISTS blog_movie_reactions (
    id          BIGSERIAL PRIMARY KEY,
    user_id     BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    target_type VARCHAR(20) NOT NULL, -- 'blog_comment', 'movie_comment'
    target_id   BIGINT NOT NULL,
    is_like     BOOLEAN NOT NULL DEFAULT TRUE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, target_type, target_id)
);
CREATE INDEX IF NOT EXISTS idx_blog_movie_reactions_target ON blog_movie_reactions(target_type, target_id);

-- ── Verification Codes (Wo_Codes) ──────────────────────────────────────────
CREATE TABLE IF NOT EXISTS verification_codes (
    id          BIGSERIAL PRIMARY KEY,
    user_id     BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    code        VARCHAR(50) NOT NULL,
    type        VARCHAR(30) NOT NULL DEFAULT 'email', -- email, sms, 2fa
    expires_at  TIMESTAMPTZ NOT NULL,
    used        BOOLEAN NOT NULL DEFAULT FALSE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_verification_codes_user ON verification_codes(user_id);
CREATE INDEX IF NOT EXISTS idx_verification_codes_code ON verification_codes(code);

-- ── Bank Receipts (bank_receipts) ──────────────────────────────────────────
CREATE TABLE IF NOT EXISTS bank_receipts (
    id              BIGSERIAL PRIMARY KEY,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    fund_id         BIGINT NOT NULL DEFAULT 0,
    description     TEXT NOT NULL DEFAULT '',
    price           NUMERIC(12,2) NOT NULL DEFAULT 0,
    mode            VARCHAR(50) NOT NULL DEFAULT '',
    approved        BOOLEAN NOT NULL DEFAULT FALSE,
    receipt_file    VARCHAR(250) NOT NULL DEFAULT '',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    approved_at     TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS idx_bank_receipts_user ON bank_receipts(user_id);

-- ── Sent Emails Log (Wo_Emails) ────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS sent_emails (
    id          BIGSERIAL PRIMARY KEY,
    user_id     BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    email_type  VARCHAR(50) NOT NULL DEFAULT '',
    subject     VARCHAR(255) NOT NULL DEFAULT '',
    sent_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_sent_emails_user ON sent_emails(user_id);

-- ── OAuth/Third-Party Apps (Wo_Apps + Wo_AppsSessions + Wo_Apps_Hash + Wo_Apps_Permission) ─
-- Already have oauth_apps, oauth_codes, oauth_tokens — add app_permissions
CREATE TABLE IF NOT EXISTS app_permissions (
    id          BIGSERIAL PRIMARY KEY,
    app_id      BIGINT NOT NULL REFERENCES oauth_apps(id) ON DELETE CASCADE,
    user_id     BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    scopes      JSONB NOT NULL DEFAULT '[]',
    granted_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(app_id, user_id)
);
CREATE INDEX IF NOT EXISTS idx_app_permissions_user ON app_permissions(user_id);

-- ── Video/Audio Calls (Wo_AgoraVideoCall + Wo_AudioCalls + Wo_VideoCalles) ─
-- Already have calls table — add call_participants for multi-party
CREATE TABLE IF NOT EXISTS call_participants (
    id          BIGSERIAL PRIMARY KEY,
    call_id     BIGINT NOT NULL REFERENCES calls(id) ON DELETE CASCADE,
    user_id     BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status      VARCHAR(20) NOT NULL DEFAULT 'ringing', -- ringing, answered, declined, missed
    joined_at   TIMESTAMPTZ,
    left_at     TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS idx_call_participants_call ON call_participants(call_id);
CREATE INDEX IF NOT EXISTS idx_call_participants_user ON call_participants(user_id);

-- ── Agora Video Call Tokens ────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS agora_tokens (
    id          BIGSERIAL PRIMARY KEY,
    user_id     BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    call_id     BIGINT NOT NULL REFERENCES calls(id) ON DELETE CASCADE,
    channel     VARCHAR(100) NOT NULL,
    token       TEXT NOT NULL,
    role        SMALLINT NOT NULL DEFAULT 1,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at  TIMESTAMPTZ NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_agora_tokens_call ON agora_tokens(call_id);

-- ── Theme/UI Settings (wondertage_settings) ────────────────────────────────
CREATE TABLE IF NOT EXISTS theme_settings (
    id      BIGSERIAL PRIMARY KEY,
    name    VARCHAR(100) NOT NULL UNIQUE,
    value   TEXT NOT NULL DEFAULT ''
);

-- ── Pro Plans (create if not exists, needed before ALTER) ──────────────────
CREATE TABLE IF NOT EXISTS pro_plans (
    id BIGSERIAL PRIMARY KEY,
    plan_type VARCHAR(50) NOT NULL UNIQUE,
    title VARCHAR(200) NOT NULL,
    price NUMERIC(10,2) NOT NULL DEFAULT 0,
    period_days INT NOT NULL DEFAULT 30,
    features JSONB DEFAULT '[]'::jsonb,
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- ── Pro Plans / Manage Pro (Wo_Manage_Pro) ─────────────────────────────────
-- Extend existing pro_plans if needed
ALTER TABLE pro_plans ADD COLUMN IF NOT EXISTS featured_member INT NOT NULL DEFAULT 0;
ALTER TABLE pro_plans ADD COLUMN IF NOT EXISTS profile_visitors INT NOT NULL DEFAULT 0;
ALTER TABLE pro_plans ADD COLUMN IF NOT EXISTS verified_badge BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE pro_plans ADD COLUMN IF NOT EXISTS discount_type VARCHAR(20) NOT NULL DEFAULT '';
ALTER TABLE pro_plans ADD COLUMN IF NOT EXISTS discount_percent NUMERIC(5,2) NOT NULL DEFAULT 0;
ALTER TABLE pro_plans ADD COLUMN IF NOT EXISTS post_promotion INT NOT NULL DEFAULT 0;
ALTER TABLE pro_plans ADD COLUMN IF NOT EXISTS page_promotion INT NOT NULL DEFAULT 0;

-- ── Add album_name to posts (for photo albums) ─────────────────────────────
ALTER TABLE posts ADD COLUMN IF NOT EXISTS album_name VARCHAR(255);

-- ── Add missing indexes on existing tables ─────────────────────────────────
CREATE INDEX IF NOT EXISTS idx_game_players_game ON game_players(game_id);
CREATE INDEX IF NOT EXISTS idx_game_players_user ON game_players(user_id);
CREATE INDEX IF NOT EXISTS idx_user_ads_user ON user_ads(user_id);
CREATE INDEX IF NOT EXISTS idx_products_user ON products(user_id);
CREATE INDEX IF NOT EXISTS idx_jobs_user ON jobs(user_id);
CREATE INDEX IF NOT EXISTS idx_fundings_user ON fundings(user_id);
CREATE INDEX IF NOT EXISTS idx_offers_user ON offers(user_id);
CREATE INDEX IF NOT EXISTS idx_withdrawal_requests_user ON withdrawal_requests(user_id);
