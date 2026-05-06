-- Migration: Granular per-user admin permissions
-- Plan §3.22 AP-A1 — replaces the "under construction" stub on
-- admin/users/[id]/permissions with a real editor.

ALTER TABLE users
    ADD COLUMN IF NOT EXISTS permissions JSONB NOT NULL DEFAULT '{}'::jsonb;

COMMENT ON COLUMN users.permissions IS
    'Granular admin permissions, merged on top of role defaults. Shape: { "<resource>.<action>": true|false }. Known keys include users.read, users.ban, posts.moderate, pages.create, groups.moderate, events.moderate, admin.settings, admin.finance, content.moderate. Missing key inherits from the role.';

CREATE INDEX IF NOT EXISTS idx_users_permissions_gin
    ON users USING GIN (permissions);
