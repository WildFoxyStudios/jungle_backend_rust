INSERT INTO cronjob_config (job_name, schedule, description)
VALUES ('pagerank', 'daily 03:00', 'Compute PYMK, page, and group recommendations')
ON CONFLICT (job_name) DO UPDATE SET schedule = EXCLUDED.schedule, description = EXCLUDED.description;
