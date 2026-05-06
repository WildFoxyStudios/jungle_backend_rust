INSERT INTO cronjob_config (job_name, schedule, description)
VALUES
    ('webhooks_dispatcher', 'continuous', 'Deliver outgoing webhooks with exponential backoff'),
    ('erasure_processor', 'daily 04:00', 'Process GDPR erasure requests after 30-day waiting period')
ON CONFLICT (job_name) DO UPDATE SET schedule = EXCLUDED.schedule, description = EXCLUDED.description;
