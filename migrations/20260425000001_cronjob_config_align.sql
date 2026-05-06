-- ═══════════════════════════════════════════════════════════════════
-- Align cronjob_config seed with the actual job spawn names used by
-- jobs-runner and the admin UI catalog (system/cronjobs page).
-- ═══════════════════════════════════════════════════════════════════
--
-- The original seed (20260418000007) used legacy PHP-style names like
-- `stories_expiry`, `events_reminder`, `live_cleanup`. The Rust runner
-- spawns jobs with their module names instead, and `cronjob_runs.name`
-- is keyed on those. To keep the admin UI's "Last run" join valid we
-- upsert the canonical names here; old rows are left in place so any
-- manual configuration is preserved (admins can delete them at will).

INSERT INTO cronjob_config (job_name, schedule, description) VALUES
    ('story_cleanup',                  'every 5m',     'Delete stories whose 24h window has expired'),
    ('session_cleanup',                'every 1h',     'Purge expired user_sessions'),
    ('notification_cleanup',           'every 24h',    'Delete read notifications older than 30 days'),
    ('pro_subscription_check',         'every 1h',     'Expire pro memberships past their end date'),
    ('event_reminders',                'every 1h',     'Notify attendees about events in the next 24h'),
    ('ad_budget_check',                'every 30m',    'Pause ads with depleted budget'),
    ('hashtag_trending',               'every 1h',     'Rebuild trending-hashtag cache'),
    ('login_attempts_cleanup',         'every 6h',     'Purge old login_attempts rows'),
    ('memories_notification',          'every 24h',    'Notify users with anniversary posts'),
    ('birthday_notifications',         'every 24h',    'Notify followers about birthdays'),
    ('live_stream_cleanup',            'every 5m',     'Reset stale live-stream flags'),
    ('publish_scheduled_posts',        'every 1m',     'Publish posts whose scheduled_at has passed'),
    ('auto_delete_old_messages',       'every 6h',     'Delete messages older than retention window'),
    ('weekly_memories_digest',         'monday 09:00', 'Weekly memories email digest'),
    ('analytics_snapshot_daily',       'daily 00:10',  'Aggregate yesterday''s metrics'),
    ('crypto_payment_reconciliation',  'every 15m',    'Reconcile crypto transactions via provider API'),
    ('expire_pending_ads',             'every 1h',     'Mark ads as expired when budget/date done'),
    ('newsletter_dispatcher',          'every 5m',     'Send pending newsletter emails in batches'),
    ('disappearing_messages_cleanup',  'every 5m',     'Purge messages past their disappearing window'),
    ('unmute_expired_conversations',   'every 15m',    'Clear muted_until once mute window elapsed'),
    ('post_viewers_cleanup',           'every 24h',    'Trim post_viewers older than 90 days'),
    ('dlq_consumer',                   'continuous',   'Persist NATS dead-letter events')
ON CONFLICT (job_name) DO UPDATE
   SET description = EXCLUDED.description,
       schedule    = COALESCE(cronjob_config.schedule, EXCLUDED.schedule),
       updated_at  = NOW();
