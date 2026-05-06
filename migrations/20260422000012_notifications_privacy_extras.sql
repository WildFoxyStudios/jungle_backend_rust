-- Migration: Email notification prefs + privacy settings documentation
-- Plan §3.19 (NT1/NT2) and §3.20 (PV1-PV4).
--
-- The existing JSONB `users.notification_settings` / `users.privacy_settings`
-- keep all per-user preferences; this migration adds a dedicated JSONB for
-- email-channel opts so admins can disable a channel entirely without
-- touching the in-app flags.

ALTER TABLE users
    ADD COLUMN IF NOT EXISTS email_notification_settings JSONB NOT NULL DEFAULT '{}'::jsonb;

COMMENT ON COLUMN users.email_notification_settings IS
    'Per-channel email delivery preferences. Known keys: e_liked, e_wondered, e_commented, e_shared, e_followed, e_accepted, e_mentioned, e_joined_group, e_liked_page, e_visited, e_memory, e_sent_me_message. Missing key = inherits from in-app notification_settings.';

-- Document the full set of privacy keys the UI and backend persist in
-- `users.privacy_settings`. No schema change; the JSONB already exists.
COMMENT ON COLUMN users.privacy_settings IS
    'Privacy JSONB. Known keys: follow_privacy, friend_privacy, message_privacy, post_privacy, birth_privacy, confirm_followers, show_activities, show_lastseen, online_status, profile_visibility, share_my_location, share_my_data, visit_privacy. Missing key defaults to the site-wide default.';
