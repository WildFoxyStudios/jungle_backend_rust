-- Migration: Page autoresponder for private messages
-- Plan §3.5 PG1 — admins can set an auto-reply that the messaging-service
-- sends the first time a visitor writes to the page.

ALTER TABLE pages
    ADD COLUMN IF NOT EXISTS autoresponder_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS autoresponder_message TEXT NOT NULL DEFAULT '';

COMMENT ON COLUMN pages.autoresponder_enabled IS
    'When TRUE, the messaging-service replies automatically to the first message a non-admin sends to the page.';

COMMENT ON COLUMN pages.autoresponder_message IS
    'Template body sent as the autoresponder. Plain text, max 2000 chars. Ignored when `autoresponder_enabled` is FALSE.';
