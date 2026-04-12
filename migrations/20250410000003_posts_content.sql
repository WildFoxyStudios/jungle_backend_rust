-- ============================================================
-- POSTS
-- ============================================================
CREATE TABLE posts (
    id              BIGSERIAL PRIMARY KEY,
    uuid            UUID NOT NULL DEFAULT gen_random_uuid() UNIQUE,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    page_id         BIGINT,
    group_id        BIGINT,
    event_id        BIGINT,
    parent_id       BIGINT REFERENCES posts(id) ON DELETE SET NULL,
    recipient_id    BIGINT REFERENCES users(id) ON DELETE SET NULL,

    content         TEXT NOT NULL DEFAULT '',
    post_type       VARCHAR(30) NOT NULL DEFAULT 'text',
    media           JSONB NOT NULL DEFAULT '[]'::jsonb,
    colored_post    JSONB,

    location        VARCHAR(300) NOT NULL DEFAULT '',
    lat             DOUBLE PRECISION,
    lng             DOUBLE PRECISION,

    feeling         VARCHAR(50) NOT NULL DEFAULT '',
    feeling_type    VARCHAR(50) NOT NULL DEFAULT '',

    privacy         VARCHAR(20) NOT NULL DEFAULT 'everyone',
    is_approved     BOOLEAN NOT NULL DEFAULT TRUE,
    is_pinned       BOOLEAN NOT NULL DEFAULT FALSE,
    is_boosted      BOOLEAN NOT NULL DEFAULT FALSE,
    is_reel         BOOLEAN NOT NULL DEFAULT FALSE,

    like_count      INT NOT NULL DEFAULT 0,
    comment_count   INT NOT NULL DEFAULT 0,
    share_count     INT NOT NULL DEFAULT 0,
    view_count      INT NOT NULL DEFAULT 0,

    search_vector   TSVECTOR,

    deleted_at      TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_posts_user ON posts(user_id, created_at DESC) WHERE deleted_at IS NULL;
CREATE INDEX idx_posts_group ON posts(group_id, created_at DESC) WHERE group_id IS NOT NULL AND deleted_at IS NULL;
CREATE INDEX idx_posts_page ON posts(page_id, created_at DESC) WHERE page_id IS NOT NULL AND deleted_at IS NULL;
CREATE INDEX idx_posts_created ON posts(created_at DESC) WHERE deleted_at IS NULL;
CREATE INDEX idx_posts_search ON posts USING GIN(search_vector);
CREATE INDEX idx_posts_feed ON posts(id DESC) WHERE deleted_at IS NULL AND is_approved = TRUE;

CREATE OR REPLACE FUNCTION posts_search_update() RETURNS TRIGGER AS $$
BEGIN
    NEW.search_vector := setweight(to_tsvector('simple', COALESCE(NEW.content, '')), 'A');
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_posts_search BEFORE INSERT OR UPDATE OF content
    ON posts FOR EACH ROW EXECUTE FUNCTION posts_search_update();

-- ============================================================
-- REACTIONS (unified: likes, wonders, reactions for posts, comments, blogs, etc.)
-- ============================================================
CREATE TABLE reactions (
    id              BIGSERIAL PRIMARY KEY,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    target_type     VARCHAR(20) NOT NULL,
    target_id       BIGINT NOT NULL,
    reaction_type   VARCHAR(20) NOT NULL DEFAULT 'like',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, target_type, target_id)
);

CREATE INDEX idx_reactions_target ON reactions(target_type, target_id);

-- ============================================================
-- COMMENTS (self-referencing: parent_id NULL = comment, NOT NULL = reply)
-- ============================================================
CREATE TABLE comments (
    id              BIGSERIAL PRIMARY KEY,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    post_id         BIGINT NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    parent_id       BIGINT REFERENCES comments(id) ON DELETE CASCADE,
    content         TEXT NOT NULL DEFAULT '',
    media           JSONB NOT NULL DEFAULT '[]'::jsonb,
    like_count      INT NOT NULL DEFAULT 0,
    reply_count     INT NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_comments_post ON comments(post_id, created_at) WHERE parent_id IS NULL;
CREATE INDEX idx_comments_parent ON comments(parent_id, created_at) WHERE parent_id IS NOT NULL;

-- ============================================================
-- SAVED / HIDDEN POSTS
-- ============================================================
CREATE TABLE saved_posts (
    id              BIGSERIAL PRIMARY KEY,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    post_id         BIGINT NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, post_id)
);

CREATE TABLE hidden_posts (
    id              BIGSERIAL PRIMARY KEY,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    post_id         BIGINT NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, post_id)
);

-- ============================================================
-- HASHTAGS
-- ============================================================
CREATE TABLE hashtags (
    id              BIGSERIAL PRIMARY KEY,
    tag             VARCHAR(255) NOT NULL UNIQUE,
    use_count       INT NOT NULL DEFAULT 0,
    trending        BOOLEAN NOT NULL DEFAULT FALSE,
    last_used_at    TIMESTAMPTZ DEFAULT NOW(),
    expires_at      DATE
);

CREATE TABLE post_hashtags (
    post_id         BIGINT NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    hashtag_id      BIGINT NOT NULL REFERENCES hashtags(id) ON DELETE CASCADE,
    PRIMARY KEY(post_id, hashtag_id)
);

-- ============================================================
-- POLLS
-- ============================================================
CREATE TABLE polls (
    id              BIGSERIAL PRIMARY KEY,
    post_id         BIGINT NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    options         JSONB NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE poll_votes (
    id              BIGSERIAL PRIMARY KEY,
    poll_id         BIGINT NOT NULL REFERENCES polls(id) ON DELETE CASCADE,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    option_index    SMALLINT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(poll_id, user_id)
);
