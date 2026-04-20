-- Track which onboarding steps a user has completed or skipped.
--
-- Shape: { "avatar": "completed" | "skipped", "info": ..., "follow": ... }

ALTER TABLE users
    ADD COLUMN IF NOT EXISTS onboarding_progress JSONB NOT NULL DEFAULT '{}'::jsonb;
