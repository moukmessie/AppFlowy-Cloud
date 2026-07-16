INSERT INTO af_system_config (key, value) VALUES
    ('block_disposable_email_domains', 'true'::jsonb)
ON CONFLICT (key) DO NOTHING;

CREATE TABLE IF NOT EXISTS af_disposable_email_domain (
    domain TEXT PRIMARY KEY CHECK (domain = lower(domain) AND domain NOT LIKE '%@%'),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID REFERENCES auth.users(id) ON DELETE SET NULL
);

INSERT INTO af_disposable_email_domain (domain) VALUES
    ('10minutemail.com'),
    ('guerrillamail.com'),
    ('mailinator.com'),
    ('temp-mail.org'),
    ('yopmail.com')
ON CONFLICT DO NOTHING;
