-- Sunshine / WoWonder parity: PHP `admin = 2` (Wo_IsModerator) may open admin-cp.
-- Staff JWT flag `is_moderator` is derived from this column (auth-service).

ALTER TABLE users
    ADD COLUMN IF NOT EXISTS is_moderator BOOLEAN NOT NULL DEFAULT FALSE;

COMMENT ON COLUMN users.is_moderator IS
    'Site moderator (PHP admin=2). May access the admin Next app / admin APIs alongside full admins.';

-- Legacy granular table from admin_advanced migration: treat any permission as moderator staff.
UPDATE users u
SET is_moderator = TRUE
FROM user_permissions up
WHERE u.id = up.user_id
  AND (
      up.can_moderate_posts
      OR up.can_moderate_users
      OR up.can_moderate_reports
      OR up.can_manage_content
      OR up.can_manage_payments
  );
