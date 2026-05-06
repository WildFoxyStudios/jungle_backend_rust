-- Announcements scheduling: add `title`, `starts_at`, and `ends_at` so admins
-- can publish a banner for a fixed window. The previous schema only had
-- `text + active`, with no way to declare when an announcement should appear.

ALTER TABLE announcements
    ADD COLUMN IF NOT EXISTS title      TEXT        NOT NULL DEFAULT '',
    ADD COLUMN IF NOT EXISTS starts_at  TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS ends_at    TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS idx_announcements_window
    ON announcements (active, starts_at, ends_at);
