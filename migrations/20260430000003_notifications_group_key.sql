-- Phase 5: Grouped notifications + deep links

ALTER TABLE notifications ADD COLUMN IF NOT EXISTS group_key VARCHAR(255);
ALTER TABLE notifications ADD COLUMN IF NOT EXISTS group_count INT NOT NULL DEFAULT 1;
ALTER TABLE notifications ADD COLUMN IF NOT EXISTS deep_link VARCHAR(512);

CREATE INDEX IF NOT EXISTS idx_notifications_group_key ON notifications(recipient_id, group_key, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_notifications_user_created ON notifications(recipient_id, created_at DESC);
