CREATE TABLE IF NOT EXISTS af_system_config (
    key TEXT PRIMARY KEY,
    value JSONB NOT NULL,
    updated_by UUID REFERENCES auth.users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO af_system_config (key, value)
VALUES
    ('signup_whitelist_enabled', 'false'::jsonb),
    ('guest_invites_require_admin_approval', 'false'::jsonb)
ON CONFLICT (key) DO NOTHING;

CREATE TABLE IF NOT EXISTS af_signup_whitelist (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    kind TEXT NOT NULL CHECK (kind IN ('email', 'domain')),
    value TEXT NOT NULL,
    created_by UUID REFERENCES auth.users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (kind, value),
    CHECK (value = lower(trim(value)))
);

CREATE INDEX IF NOT EXISTS idx_af_signup_whitelist_kind
    ON af_signup_whitelist(kind);

ALTER TABLE af_workspace_invitation
    ADD COLUMN IF NOT EXISTS admin_approval_status SMALLINT NOT NULL DEFAULT 0
    CHECK (admin_approval_status IN (0, 1, 2));

CREATE INDEX IF NOT EXISTS idx_af_workspace_invitation_admin_approval
    ON af_workspace_invitation(admin_approval_status)
    WHERE status = 0;
