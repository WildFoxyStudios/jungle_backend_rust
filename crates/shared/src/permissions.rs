//! Staff permission catalog — 40 granular permissions for admin RBAC.
//!
//! Naming convention: `manage_X` (write/delete) and `view_X` (read-only),
//! matching the `admin_permissions_catalog` DB table seeded in migration
//! `20260430000001_staff_roles_presets.sql`.

use serde::{Deserialize, Serialize};

/// Every permission in the system, mirroring `admin_permissions_catalog.key`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    // Dashboard
    ViewDashboard,

    // Users
    ManageUsers,
    ViewUsers,
    VerifyUsers,
    ImpersonateUsers,
    ManageKyc,
    ManagePro,

    // Content
    ManagePosts,
    ManageComments,

    // Moderation
    ModerateReports,
    ViewModerationQueue,
    AssignStrikes,

    // Communities
    ManageGroups,
    ManagePages,
    ManageEvents,

    // Commerce
    ManageProducts,
    ManageJobs,
    ManageAds,
    ManageAdCampaigns,
    ManageMarketplaceDisputes,

    // Payments
    ManagePayments,

    // Settings
    ManageSettings,
    ManageEmailTemplates,
    ManageTranslations,
    ManageThemes,
    ManageAi,
    ManageStorage,
    ManageOauth,
    ManageRolePresets,
    ViewStaffDirectory,

    // System
    ViewAuditLog,
    ViewActivityLog,
    ManageApiKeys,
    ViewHealth,
    TriggerBackup,
    SendNewsletter,
    ManageCronjobs,
    ManageDlq,
    ManageWebhooks,
    ManageDataExports,
}

impl Permission {
    /// Returns the snake_case DB key, e.g. `Permission::ManageUsers` → `"manage_users"`.
    pub fn as_key(self) -> &'static str {
        match self {
            Self::ViewDashboard => "view_dashboard",
            Self::ManageUsers => "manage_users",
            Self::ViewUsers => "view_users",
            Self::VerifyUsers => "verify_users",
            Self::ImpersonateUsers => "impersonate_users",
            Self::ManageKyc => "manage_kyc",
            Self::ManagePro => "manage_pro",
            Self::ManagePosts => "manage_posts",
            Self::ManageComments => "manage_comments",
            Self::ModerateReports => "moderate_reports",
            Self::ViewModerationQueue => "view_moderation_queue",
            Self::AssignStrikes => "assign_strikes",
            Self::ManageGroups => "manage_groups",
            Self::ManagePages => "manage_pages",
            Self::ManageEvents => "manage_events",
            Self::ManageProducts => "manage_products",
            Self::ManageJobs => "manage_jobs",
            Self::ManageAds => "manage_ads",
            Self::ManageAdCampaigns => "manage_ad_campaigns",
            Self::ManageMarketplaceDisputes => "manage_marketplace_disputes",
            Self::ManagePayments => "manage_payments",
            Self::ManageSettings => "manage_settings",
            Self::ManageEmailTemplates => "manage_email_templates",
            Self::ManageTranslations => "manage_translations",
            Self::ManageThemes => "manage_themes",
            Self::ManageAi => "manage_ai",
            Self::ManageStorage => "manage_storage",
            Self::ManageOauth => "manage_oauth",
            Self::ManageRolePresets => "manage_role_presets",
            Self::ViewStaffDirectory => "view_staff_directory",
            Self::ViewAuditLog => "view_audit_log",
            Self::ViewActivityLog => "view_activity_log",
            Self::ManageApiKeys => "manage_api_keys",
            Self::ViewHealth => "view_health",
            Self::TriggerBackup => "trigger_backup",
            Self::SendNewsletter => "send_newsletter",
            Self::ManageCronjobs => "manage_cronjobs",
            Self::ManageDlq => "manage_dlq",
            Self::ManageWebhooks => "manage_webhooks",
            Self::ManageDataExports => "manage_data_exports",
        }
    }

    /// All 40 permissions for UI catalog rendering.
    pub fn all() -> Vec<Permission> {
        use Permission::*;
        vec![
            ViewDashboard,
            ManageUsers, ViewUsers, VerifyUsers, ImpersonateUsers, ManageKyc, ManagePro,
            ManagePosts, ManageComments,
            ModerateReports, ViewModerationQueue, AssignStrikes,
            ManageGroups, ManagePages, ManageEvents,
            ManageProducts, ManageJobs, ManageAds, ManageAdCampaigns, ManageMarketplaceDisputes,
            ManagePayments,
            ManageSettings, ManageEmailTemplates, ManageTranslations, ManageThemes,
            ManageAi, ManageStorage, ManageOauth, ManageRolePresets, ViewStaffDirectory,
            ViewAuditLog, ViewActivityLog, ManageApiKeys, ViewHealth,
            TriggerBackup, SendNewsletter, ManageCronjobs, ManageDlq,
            ManageWebhooks, ManageDataExports,
        ]
    }

    /// Human-readable category for grouping in the UI.
    pub fn category(self) -> &'static str {
        match self {
            Self::ViewDashboard => "general",
            Self::ManageUsers | Self::ViewUsers | Self::VerifyUsers
            | Self::ImpersonateUsers | Self::ManageKyc | Self::ManagePro => "users",
            Self::ManagePosts | Self::ManageComments => "content",
            Self::ModerateReports | Self::ViewModerationQueue | Self::AssignStrikes => "moderation",
            Self::ManageGroups | Self::ManagePages | Self::ManageEvents => "communities",
            Self::ManageProducts | Self::ManageJobs | Self::ManageAds
            | Self::ManageAdCampaigns | Self::ManageMarketplaceDisputes => "commerce",
            Self::ManagePayments => "payments",
            Self::ManageSettings | Self::ManageEmailTemplates | Self::ManageTranslations
            | Self::ManageThemes | Self::ManageAi | Self::ManageStorage
            | Self::ManageOauth | Self::ManageRolePresets | Self::ViewStaffDirectory => "settings",
            Self::ViewAuditLog | Self::ViewActivityLog | Self::ManageApiKeys
            | Self::ViewHealth | Self::TriggerBackup | Self::SendNewsletter
            | Self::ManageCronjobs | Self::ManageDlq | Self::ManageWebhooks
            | Self::ManageDataExports => "system",
        }
    }
}
