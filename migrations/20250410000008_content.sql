-- Migration 008: Content (Blogs, Forums, Movies, Games)

-- Blogs
CREATE TABLE blogs (
    id              BIGSERIAL PRIMARY KEY,
    uuid            UUID NOT NULL DEFAULT gen_random_uuid() UNIQUE,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title           VARCHAR(200) NOT NULL,
    content         TEXT,
    description     TEXT,
    thumbnail       TEXT DEFAULT 'default-blog.jpg',
    category_id     BIGINT REFERENCES categories(id),
    tags            TEXT[] DEFAULT '{}',
    view_count      INT NOT NULL DEFAULT 0,
    share_count     INT NOT NULL DEFAULT 0,
    is_approved     BOOLEAN NOT NULL DEFAULT TRUE,
    search_vector   TSVECTOR,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_blogs_user ON blogs(user_id, created_at DESC);
CREATE INDEX idx_blogs_search ON blogs USING GIN(search_vector);
CREATE INDEX idx_blogs_category ON blogs(category_id) WHERE is_approved = TRUE;

-- Blog comments (unified with replies via parent_id)
CREATE TABLE blog_comments (
    id              BIGSERIAL PRIMARY KEY,
    blog_id         BIGINT NOT NULL REFERENCES blogs(id) ON DELETE CASCADE,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    parent_id       BIGINT REFERENCES blog_comments(id) ON DELETE CASCADE,
    content         TEXT NOT NULL,
    like_count      INT NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_blog_comments_blog ON blog_comments(blog_id, created_at);

-- Auto-update blog search_vector
CREATE OR REPLACE FUNCTION blog_search_vector_update() RETURNS trigger AS $$
BEGIN
    NEW.search_vector := to_tsvector('english', COALESCE(NEW.title, '') || ' ' || COALESCE(NEW.description, '') || ' ' || COALESCE(NEW.content, ''));
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_blog_search_vector
    BEFORE INSERT OR UPDATE OF title, description, content ON blogs
    FOR EACH ROW EXECUTE FUNCTION blog_search_vector_update();

-- Forum sections
CREATE TABLE forum_sections (
    id              BIGSERIAL PRIMARY KEY,
    name            VARCHAR(200) NOT NULL,
    description     VARCHAR(500) DEFAULT ''
);

-- Forums
CREATE TABLE forums (
    id              BIGSERIAL PRIMARY KEY,
    section_id      BIGINT NOT NULL REFERENCES forum_sections(id) ON DELETE CASCADE,
    name            VARCHAR(200) NOT NULL,
    description     VARCHAR(500) DEFAULT '',
    thread_count    INT NOT NULL DEFAULT 0,
    last_post_at    TIMESTAMPTZ
);

-- Forum threads
CREATE TABLE forum_threads (
    id              BIGSERIAL PRIMARY KEY,
    forum_id        BIGINT NOT NULL REFERENCES forums(id) ON DELETE CASCADE,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title           VARCHAR(300) NOT NULL,
    content         TEXT NOT NULL,
    view_count      INT NOT NULL DEFAULT 0,
    reply_count     INT NOT NULL DEFAULT 0,
    last_reply_at   TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_forum_threads_forum ON forum_threads(forum_id, created_at DESC);

-- Forum replies
CREATE TABLE forum_replies (
    id              BIGSERIAL PRIMARY KEY,
    thread_id       BIGINT NOT NULL REFERENCES forum_threads(id) ON DELETE CASCADE,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    content         TEXT NOT NULL,
    quoted_reply_id BIGINT REFERENCES forum_replies(id),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Movies
CREATE TABLE movies (
    id              BIGSERIAL PRIMARY KEY,
    user_id         BIGINT NOT NULL REFERENCES users(id),
    name            VARCHAR(200) NOT NULL,
    cover           TEXT DEFAULT '',
    video_url       TEXT NOT NULL,
    iframe_url      TEXT DEFAULT '',
    description     TEXT DEFAULT '',
    genre           VARCHAR(50) DEFAULT '',
    country         VARCHAR(50) DEFAULT '',
    stars           VARCHAR(300) DEFAULT '',
    producer        VARCHAR(200) DEFAULT '',
    release_year    INT,
    duration        VARCHAR(50) DEFAULT '',
    quality         VARCHAR(20) DEFAULT '',
    view_count      INT NOT NULL DEFAULT 0,
    is_approved     BOOLEAN NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Games
CREATE TABLE games (
    id              BIGSERIAL PRIMARY KEY,
    name            VARCHAR(100) NOT NULL,
    avatar          TEXT NOT NULL,
    link            TEXT NOT NULL,
    active          BOOLEAN NOT NULL DEFAULT TRUE,
    player_count    INT NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE game_players (
    id              BIGSERIAL PRIMARY KEY,
    game_id         BIGINT NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    last_played_at  TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(game_id, user_id)
);
