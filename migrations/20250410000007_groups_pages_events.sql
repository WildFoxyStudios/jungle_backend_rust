-- Migration 007: Categories, Pages, Groups, Events

-- Categories (generic — replaces Pages_Categories, Groups_Categories, Blogs_Categories, etc.)
CREATE TABLE categories (
    id              BIGSERIAL PRIMARY KEY,
    type            VARCHAR(30) NOT NULL,              -- page, group, blog, product, job
    parent_id       BIGINT REFERENCES categories(id),
    name_key        VARCHAR(160) NOT NULL,
    slug            VARCHAR(100),
    active          BOOLEAN NOT NULL DEFAULT TRUE,
    sort_order      INT NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_categories_type ON categories(type) WHERE active = TRUE;

-- Pages
CREATE TABLE pages (
    id              BIGSERIAL PRIMARY KEY,
    uuid            UUID NOT NULL DEFAULT gen_random_uuid() UNIQUE,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    page_name       VARCHAR(32) NOT NULL UNIQUE,
    page_title      VARCHAR(100) NOT NULL,
    avatar          TEXT DEFAULT 'default-page.jpg',
    cover           TEXT DEFAULT 'default-cover.jpg',
    about           TEXT DEFAULT '',
    category_id     BIGINT REFERENCES categories(id),
    website         VARCHAR(255) DEFAULT '',
    phone           VARCHAR(30) DEFAULT '',
    address         VARCHAR(300) DEFAULT '',
    company         VARCHAR(100) DEFAULT '',
    is_verified     BOOLEAN NOT NULL DEFAULT FALSE,
    is_boosted      BOOLEAN NOT NULL DEFAULT FALSE,
    active          BOOLEAN NOT NULL DEFAULT TRUE,
    rating          DECIMAL(3,2) DEFAULT 0.00,
    rating_count    INT DEFAULT 0,
    like_count      INT NOT NULL DEFAULT 0,
    social_links    JSONB DEFAULT '{}'::jsonb,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_pages_user ON pages(user_id);
CREATE INDEX idx_pages_category ON pages(category_id) WHERE active = TRUE;

-- Page likes
CREATE TABLE page_likes (
    id              BIGSERIAL PRIMARY KEY,
    page_id         BIGINT NOT NULL REFERENCES pages(id) ON DELETE CASCADE,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(page_id, user_id)
);

-- Page admins
CREATE TABLE page_admins (
    id              BIGSERIAL PRIMARY KEY,
    page_id         BIGINT NOT NULL REFERENCES pages(id) ON DELETE CASCADE,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    permissions     JSONB DEFAULT '{}'::jsonb,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(page_id, user_id)
);

-- Page ratings
CREATE TABLE page_ratings (
    id              BIGSERIAL PRIMARY KEY,
    page_id         BIGINT NOT NULL REFERENCES pages(id) ON DELETE CASCADE,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    rating          SMALLINT NOT NULL CHECK (rating BETWEEN 1 AND 5),
    review          TEXT DEFAULT '',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(page_id, user_id)
);

-- Groups
CREATE TABLE groups (
    id              BIGSERIAL PRIMARY KEY,
    uuid            UUID NOT NULL DEFAULT gen_random_uuid() UNIQUE,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    group_name      VARCHAR(32) NOT NULL UNIQUE,
    group_title     VARCHAR(100) NOT NULL,
    avatar          TEXT DEFAULT 'default-group.jpg',
    cover           TEXT DEFAULT 'default-cover.jpg',
    about           TEXT DEFAULT '',
    category_id     BIGINT REFERENCES categories(id),
    privacy         VARCHAR(20) NOT NULL DEFAULT 'public',
    join_privacy    VARCHAR(20) NOT NULL DEFAULT 'open',
    active          BOOLEAN NOT NULL DEFAULT TRUE,
    member_count    INT NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_groups_user ON groups(user_id);
CREATE INDEX idx_groups_category ON groups(category_id) WHERE active = TRUE;

-- Group members (unifies Wo_Group_Members + Wo_GroupAdmins)
CREATE TABLE group_members (
    id              BIGSERIAL PRIMARY KEY,
    group_id        BIGINT NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role            VARCHAR(20) NOT NULL DEFAULT 'member',   -- owner, admin, moderator, member
    status          VARCHAR(20) NOT NULL DEFAULT 'active',   -- active, pending, banned
    permissions     JSONB DEFAULT '{}'::jsonb,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(group_id, user_id)
);

CREATE INDEX idx_group_members_user ON group_members(user_id) WHERE status = 'active';

-- Events
CREATE TABLE events (
    id              BIGSERIAL PRIMARY KEY,
    uuid            UUID NOT NULL DEFAULT gen_random_uuid() UNIQUE,
    creator_id      BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name            VARCHAR(150) NOT NULL,
    description     TEXT NOT NULL DEFAULT '',
    location        VARCHAR(300) DEFAULT '',
    cover           TEXT DEFAULT 'default-cover.jpg',
    start_at        TIMESTAMPTZ NOT NULL,
    end_at          TIMESTAMPTZ NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_events_creator ON events(creator_id);
CREATE INDEX idx_events_upcoming ON events(start_at) WHERE start_at > NOW();

-- Event responses (unifies Wo_Egoing + Wo_Einterested + Wo_Einvited)
CREATE TABLE event_responses (
    id              BIGSERIAL PRIMARY KEY,
    event_id        BIGINT NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    response        VARCHAR(20) NOT NULL,    -- going, interested, not_going, invited
    inviter_id      BIGINT REFERENCES users(id),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(event_id, user_id)
);
