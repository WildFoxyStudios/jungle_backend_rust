-- Migration: Chat extras (wallpaper, temporal mute, disappearing-messages job index)
-- Plan §3.1 — C4 (wallpaper), C5 (disappearing messages), C6 (mute)

-- Per-user wallpaper per conversation (one image override per member)
ALTER TABLE conversation_members
    ADD COLUMN IF NOT EXISTS wallpaper_url TEXT;

-- Temporal mute: NULL = not muted, timestamp = muted until that moment.
-- The existing `muted` boolean remains for "mute indefinitely"; when
-- `muted_until` is set, it overrides `muted` (a background query can set
-- `muted=false` once `muted_until < NOW()`).
ALTER TABLE conversation_members
    ADD COLUMN IF NOT EXISTS muted_until TIMESTAMPTZ;

COMMENT ON COLUMN conversation_members.wallpaper_url IS
    'Optional per-user background override for the chat. NULL means inherit from conversation or theme default.';

COMMENT ON COLUMN conversation_members.muted_until IS
    'If set, conversation is muted until this timestamp. Takes precedence over `muted` flag when later than NOW().';

-- Disappearing messages: duration in seconds after which each new message in
-- the conversation auto-expires. NULL = disabled.
-- Common presets: 86400 (24h), 604800 (7d), 2592000 (30d).
-- The existing `destruct_at TIMESTAMPTZ` column stays as a legacy no-op; the
-- cleanup job uses this properly-typed INTEGER instead.
ALTER TABLE conversations
    ADD COLUMN IF NOT EXISTS destruct_after_seconds INTEGER;

COMMENT ON COLUMN conversations.destruct_after_seconds IS
    'Disappearing messages lifetime in seconds. Each message older than this gets purged by the disappearing_messages_cleanup job. NULL disables the feature.';

CREATE INDEX IF NOT EXISTS idx_conversations_destruct_secs
    ON conversations(destruct_after_seconds)
    WHERE destruct_after_seconds IS NOT NULL;
