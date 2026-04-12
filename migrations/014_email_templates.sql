-- Email templates for transactional emails
CREATE TABLE IF NOT EXISTS email_templates (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(200) NOT NULL UNIQUE,
    subject VARCHAR(500) NOT NULL,
    body TEXT NOT NULL,
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Seed default templates
INSERT INTO email_templates (name, subject, body) VALUES
    ('welcome', 'Welcome to {{site_name}}!', '<h1>Welcome {{first_name}}</h1><p>Thank you for joining {{site_name}}.</p>'),
    ('password_reset', 'Reset your password', '<h1>Password Reset</h1><p>Click <a href="{{reset_link}}">here</a> to reset your password.</p>'),
    ('email_verification', 'Verify your email', '<h1>Email Verification</h1><p>Your code: <strong>{{code}}</strong></p>'),
    ('new_follower', '{{sender_name}} started following you', '<p>{{sender_name}} started following you on {{site_name}}.</p>'),
    ('new_message', 'New message from {{sender_name}}', '<p>You have a new message from {{sender_name}}: {{preview}}</p>'),
    ('pro_expiring', 'Your Pro subscription is expiring soon', '<p>Your Pro subscription expires on {{expiry_date}}. Renew now to keep your benefits.</p>'),
    ('pro_expired', 'Your Pro subscription has expired', '<p>Your Pro subscription expired. Renew to restore your Pro features.</p>'),
    ('newsletter', '{{subject}}', '{{body}}')
ON CONFLICT (name) DO NOTHING;

-- Newsletter subscribers unique index
CREATE UNIQUE INDEX IF NOT EXISTS idx_newsletter_email ON newsletter_subscribers(email);

-- Post hashtags junction table (if not exists)
CREATE TABLE IF NOT EXISTS post_hashtags (
    id BIGSERIAL PRIMARY KEY,
    post_id BIGINT NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    hashtag_id BIGINT NOT NULL REFERENCES hashtags(id) ON DELETE CASCADE,
    UNIQUE(post_id, hashtag_id)
);
CREATE INDEX IF NOT EXISTS idx_post_hashtags_hashtag ON post_hashtags(hashtag_id);
CREATE INDEX IF NOT EXISTS idx_post_hashtags_post ON post_hashtags(post_id);
