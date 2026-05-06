-- Extend `invitation_links` with usage-tracking columns so a single code
-- can be reused up to `max_uses` times before it is auto-deactivated.
--
-- The admin handler (`crates/admin-service/src/handlers/invitations.rs`)
-- and the new auth register flow both expect these columns; the original
-- 20250410000011_remaining.sql migration only created the bare minimum.

ALTER TABLE invitation_links
    ADD COLUMN IF NOT EXISTS max_uses BIGINT NOT NULL DEFAULT 100,
    ADD COLUMN IF NOT EXISTS uses BIGINT NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS is_active BOOLEAN NOT NULL DEFAULT TRUE;

-- Fast-path for register flow: look up by code only when still valid.
CREATE INDEX IF NOT EXISTS idx_invitation_links_code_active
    ON invitation_links(code)
    WHERE is_active = TRUE;
