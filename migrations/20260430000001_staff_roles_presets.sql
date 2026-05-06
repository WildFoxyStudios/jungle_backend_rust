-- Phase 2 RBAC: staff_role_presets + extended permissions catalog
-- Naming convention: manage_X / view_X (snake_case, NO dots)

CREATE TABLE IF NOT EXISTS staff_role_presets (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(64) NOT NULL UNIQUE,
    label VARCHAR(128) NOT NULL,
    permissions JSONB NOT NULL DEFAULT '[]',
    is_system BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Seed 9 system presets
INSERT INTO staff_role_presets (name, label, permissions, is_system) VALUES
('super_admin', 'Super Admin',
 '["view_dashboard","manage_users","view_users","verify_users","impersonate_users","manage_posts","manage_comments","moderate_reports","view_moderation_queue","assign_strikes","manage_groups","manage_pages","manage_events","manage_products","manage_jobs","manage_ads","manage_ad_campaigns","manage_payments","manage_settings","manage_email_templates","manage_translations","manage_themes","manage_ai","manage_storage","manage_oauth","view_audit_log","view_activity_log","manage_api_keys","view_health","trigger_backup","send_newsletter","manage_cronjobs","manage_dlq","manage_kyc","manage_pro","manage_marketplace_disputes","manage_webhooks","manage_role_presets","view_staff_directory","manage_data_exports"]'::jsonb,
 TRUE),
('moderator', 'Moderator',
 '["view_dashboard","view_users","manage_posts","manage_comments","moderate_reports","view_moderation_queue","assign_strikes"]'::jsonb,
 TRUE),
('support', 'Support',
 '["view_dashboard","view_users","view_audit_log","view_activity_log"]'::jsonb,
 TRUE),
('finance', 'Finance',
 '["view_dashboard","manage_payments","view_audit_log"]'::jsonb,
 TRUE),
('content_editor', 'Content Editor',
 '["view_dashboard","manage_posts","manage_comments","manage_email_templates"]'::jsonb,
 TRUE),
('community_manager', 'Community Manager',
 '["view_dashboard","manage_groups","manage_pages","manage_events"]'::jsonb,
 TRUE),
('marketing', 'Marketing',
 '["view_dashboard","manage_ads","manage_ad_campaigns","send_newsletter"]'::jsonb,
 TRUE),
('developer', 'Developer',
 '["view_dashboard","view_health","manage_api_keys","manage_oauth","manage_webhooks"]'::jsonb,
 TRUE),
('auditor', 'Auditor',
 '["view_dashboard","view_audit_log","view_activity_log","view_health"]'::jsonb,
 TRUE)
ON CONFLICT (name) DO NOTHING;

-- Add 10 new permissions to the catalog
INSERT INTO admin_permissions_catalog (key, category, description) VALUES
('view_moderation_queue', 'moderation', 'View unified moderation queue'),
('assign_strikes', 'moderation', 'Assign strikes to users'),
('manage_kyc', 'users', 'Manage KYC verification requests'),
('manage_pro', 'users', 'Manage Pro memberships'),
('manage_marketplace_disputes', 'commerce', 'Resolve marketplace disputes'),
('manage_ad_campaigns', 'commerce', 'Manage ad campaigns and targeting'),
('manage_webhooks', 'system', 'Manage outgoing webhooks'),
('manage_role_presets', 'settings', 'Create and edit staff role presets'),
('view_staff_directory', 'settings', 'View staff directory'),
('manage_data_exports', 'system', 'Manage GDPR data export requests')
ON CONFLICT (key) DO NOTHING;
