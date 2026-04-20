-- ═══════════════════════════════════════════════════════════════════
-- Make users.password_hash nullable
-- ═══════════════════════════════════════════════════════════════════
--
-- OAuth / social-login users have no password at sign-up time. The WoWonder
-- PHP flow lets them set one later via the `api/update_social_login`
-- endpoint. To mirror that we need `password_hash` to allow NULL so the
-- social-login INSERT in auth-service/handlers/social.rs succeeds.
--
-- When a user with NULL password tries to log in with email + password,
-- the normal login handler will already reject the attempt (hash mismatch),
-- so security is unchanged.

ALTER TABLE users ALTER COLUMN password_hash DROP NOT NULL;
