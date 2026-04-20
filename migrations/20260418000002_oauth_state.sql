-- Add state param to oauth_codes for CSRF protection

ALTER TABLE oauth_codes
    ADD COLUMN IF NOT EXISTS state VARCHAR(128);

-- For cleanup of expired codes (optional)
CREATE INDEX IF NOT EXISTS idx_oauth_codes_expires_at
    ON oauth_codes (expires_at);
