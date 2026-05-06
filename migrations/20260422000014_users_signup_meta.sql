-- Migration: Extra signup metadata on users for the admin users page
-- Plan §3.22 AP-A3 — IP / signup source columns surfaced in the listing.

ALTER TABLE users
    ADD COLUMN IF NOT EXISTS signup_ip     TEXT,
    ADD COLUMN IF NOT EXISTS signup_source VARCHAR(32);

COMMENT ON COLUMN users.signup_ip IS
    'IP address captured at registration. Used by admin tools and anti-fraud heuristics. Not used for runtime request routing.';

COMMENT ON COLUMN users.signup_source IS
    'Entry point that produced this account. Known values: "web", "mobile", "facebook", "google", "twitter", "linkedin", "apple", "invite". Free-form for custom providers.';

CREATE INDEX IF NOT EXISTS idx_users_signup_ip
    ON users(signup_ip)
    WHERE signup_ip IS NOT NULL;
