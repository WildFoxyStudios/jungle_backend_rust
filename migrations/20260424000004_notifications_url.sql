-- Persist a precomputed deep-link on each notification row so clients can
-- navigate without rebuilding the URL from `target_type`/`target_id`. The
-- column is nullable: callers that haven't been migrated yet still work.
ALTER TABLE notifications
    ADD COLUMN IF NOT EXISTS url TEXT NULL;
