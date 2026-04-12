-- ============================================================
-- FOLLOWS
-- ============================================================
CREATE TABLE follows (
    id              BIGSERIAL PRIMARY KEY,
    follower_id     BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    following_id    BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status          VARCHAR(20) NOT NULL DEFAULT 'active',
    notify          BOOLEAN NOT NULL DEFAULT FALSE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(follower_id, following_id)
);

CREATE INDEX idx_follows_follower ON follows(follower_id) WHERE status = 'active';
CREATE INDEX idx_follows_following ON follows(following_id) WHERE status = 'active';

-- ============================================================
-- BLOCKS
-- ============================================================
CREATE TABLE blocks (
    id              BIGSERIAL PRIMARY KEY,
    blocker_id      BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    blocked_id      BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(blocker_id, blocked_id)
);

-- ============================================================
-- POKES
-- ============================================================
CREATE TABLE pokes (
    id              BIGSERIAL PRIMARY KEY,
    poker_id        BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    poked_id        BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(poker_id, poked_id)
);

-- ============================================================
-- MUTES
-- ============================================================
CREATE TABLE mutes (
    id              BIGSERIAL PRIMARY KEY,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    muted_id        BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    mute_type       VARCHAR(20) NOT NULL DEFAULT 'all',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, muted_id, mute_type)
);

-- ============================================================
-- FAMILY RELATIONS
-- ============================================================
CREATE TABLE family_relations (
    id              BIGSERIAL PRIMARY KEY,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    member_id       BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    relation_type   VARCHAR(30) NOT NULL,
    status          VARCHAR(20) NOT NULL DEFAULT 'pending',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, member_id)
);

-- ============================================================
-- USER PROFILE EXTENSIONS (LinkedIn mode)
-- ============================================================
CREATE TABLE user_experience (
    id              BIGSERIAL PRIMARY KEY,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title           VARCHAR(200) NOT NULL,
    company         VARCHAR(200) NOT NULL DEFAULT '',
    location        VARCHAR(200) NOT NULL DEFAULT '',
    description     TEXT NOT NULL DEFAULT '',
    start_date      DATE,
    end_date        DATE,
    is_current      BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE TABLE user_certifications (
    id              BIGSERIAL PRIMARY KEY,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name            VARCHAR(200) NOT NULL,
    issuer          VARCHAR(200) NOT NULL DEFAULT '',
    issue_date      DATE,
    url             TEXT NOT NULL DEFAULT ''
);

CREATE TABLE user_skills (
    id              BIGSERIAL PRIMARY KEY,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    skill           VARCHAR(100) NOT NULL,
    UNIQUE(user_id, skill)
);

-- ============================================================
-- PROFILE FIELDS (custom fields from admin)
-- ============================================================
CREATE TABLE profile_fields (
    id              BIGSERIAL PRIMARY KEY,
    name            VARCHAR(100) NOT NULL,
    description     TEXT NOT NULL DEFAULT '',
    field_type      VARCHAR(30) NOT NULL,
    placement       VARCHAR(30) NOT NULL DEFAULT 'profile',
    options         JSONB NOT NULL DEFAULT '[]'::jsonb,
    required        BOOLEAN NOT NULL DEFAULT FALSE,
    active          BOOLEAN NOT NULL DEFAULT TRUE,
    sort_order      INT NOT NULL DEFAULT 0
);

CREATE TABLE user_field_values (
    id              BIGSERIAL PRIMARY KEY,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    field_id        BIGINT NOT NULL REFERENCES profile_fields(id) ON DELETE CASCADE,
    value           TEXT NOT NULL DEFAULT '',
    UNIQUE(user_id, field_id)
);
