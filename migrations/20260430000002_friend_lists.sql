-- Phase 4 Privacy: friend_lists + audience columns

CREATE TABLE IF NOT EXISTS friend_lists (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name VARCHAR(128) NOT NULL,
    list_type VARCHAR(20) NOT NULL DEFAULT 'custom' CHECK (list_type IN ('close_friends', 'restricted', 'custom')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, name)
);

CREATE TABLE IF NOT EXISTS friend_list_members (
    list_id BIGINT NOT NULL REFERENCES friend_lists(id) ON DELETE CASCADE,
    friend_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    added_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (list_id, friend_id)
);

-- Audience per post
ALTER TABLE posts ADD COLUMN IF NOT EXISTS audience VARCHAR(20) NOT NULL DEFAULT 'public'
    CHECK (audience IN ('public', 'friends', 'close_friends', 'custom', 'only_me'));
ALTER TABLE posts ADD COLUMN IF NOT EXISTS audience_list_id BIGINT REFERENCES friend_lists(id);

-- Audience per story
ALTER TABLE stories ADD COLUMN IF NOT EXISTS audience VARCHAR(20) NOT NULL DEFAULT 'public'
    CHECK (audience IN ('public', 'friends', 'close_friends', 'custom', 'only_me'));

-- Lock profile
ALTER TABLE users ADD COLUMN IF NOT EXISTS profile_locked BOOLEAN NOT NULL DEFAULT FALSE;

-- Create default close_friends and restricted lists for existing users
INSERT INTO friend_lists (user_id, name, list_type)
SELECT id, 'Close Friends', 'close_friends' FROM users
ON CONFLICT (user_id, name) DO NOTHING;

INSERT INTO friend_lists (user_id, name, list_type)
SELECT id, 'Restricted', 'restricted' FROM users
ON CONFLICT (user_id, name) DO NOTHING;
