-- Migration 004: Messaging (conversations, messages, broadcasts)

-- Conversations (direct & group chats)
CREATE TABLE conversations (
    id              BIGSERIAL PRIMARY KEY,
    type            VARCHAR(20) NOT NULL DEFAULT 'direct',  -- direct, group
    name            VARCHAR(100),
    avatar          TEXT,
    creator_id      BIGINT REFERENCES users(id) ON DELETE SET NULL,
    color           VARCHAR(20) DEFAULT '#0084ff',
    destruct_at     TIMESTAMPTZ,
    last_message_at TIMESTAMPTZ DEFAULT NOW(),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_conversations_last_msg ON conversations(last_message_at DESC);

-- Conversation members
CREATE TABLE conversation_members (
    id              BIGSERIAL PRIMARY KEY,
    conversation_id BIGINT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role            VARCHAR(20) NOT NULL DEFAULT 'member',   -- owner, admin, member
    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    last_read_at    TIMESTAMPTZ DEFAULT NOW(),
    muted           BOOLEAN NOT NULL DEFAULT FALSE,
    pinned          BOOLEAN NOT NULL DEFAULT FALSE,
    archived        BOOLEAN NOT NULL DEFAULT FALSE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(conversation_id, user_id)
);

CREATE INDEX idx_conv_members_user ON conversation_members(user_id) WHERE is_active = TRUE;
CREATE INDEX idx_conv_members_pinned ON conversation_members(user_id) WHERE pinned = TRUE AND is_active = TRUE;
CREATE INDEX idx_conv_members_archived ON conversation_members(user_id) WHERE archived = TRUE AND is_active = TRUE;

-- Messages
CREATE TABLE messages (
    id              BIGSERIAL PRIMARY KEY,
    conversation_id BIGINT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    sender_id       BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    content         TEXT DEFAULT '',
    message_type    VARCHAR(20) NOT NULL DEFAULT 'text',    -- text, image, video, audio, file, sticker, gif, contact, location
    media           JSONB DEFAULT '[]'::jsonb,
    reply_to_id     BIGINT REFERENCES messages(id) ON DELETE SET NULL,
    forwarded_from  BIGINT REFERENCES messages(id) ON DELETE SET NULL,
    is_pinned       BOOLEAN NOT NULL DEFAULT FALSE,
    is_favorited    BOOLEAN NOT NULL DEFAULT FALSE,
    seen_by         JSONB DEFAULT '[]'::jsonb,              -- array of user_ids who read it
    deleted_at      TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_messages_conversation ON messages(conversation_id, created_at DESC) WHERE deleted_at IS NULL;
CREATE INDEX idx_messages_sender ON messages(sender_id, created_at DESC);
CREATE INDEX idx_messages_pinned ON messages(conversation_id) WHERE is_pinned = TRUE AND deleted_at IS NULL;

-- Broadcasts
CREATE TABLE broadcasts (
    id              BIGSERIAL PRIMARY KEY,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name            VARCHAR(100) NOT NULL,
    avatar          TEXT DEFAULT 'default-group.jpg',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE broadcast_members (
    id              BIGSERIAL PRIMARY KEY,
    broadcast_id    BIGINT NOT NULL REFERENCES broadcasts(id) ON DELETE CASCADE,
    user_id         BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(broadcast_id, user_id)
);

-- Message reactions (reuse reactions table with target_type='message')
-- No new table needed, reactions table from 003 handles this.
