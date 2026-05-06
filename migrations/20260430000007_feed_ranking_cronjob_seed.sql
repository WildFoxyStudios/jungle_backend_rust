INSERT INTO cronjob_config (job_name, schedule, description)
VALUES ('feed_ranking', 'every 30m', 'Compute EdgeRank scores for feed posts')
ON CONFLICT (job_name) DO UPDATE SET schedule = EXCLUDED.schedule, description = EXCLUDED.description;
