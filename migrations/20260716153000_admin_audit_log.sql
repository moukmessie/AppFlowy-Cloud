CREATE TABLE IF NOT EXISTS af_admin_audit_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    actor_uuid UUID REFERENCES auth.users(id) ON DELETE SET NULL,
    action TEXT NOT NULL,
    target_type TEXT NOT NULL,
    target_id TEXT,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_af_admin_audit_log_created_at
    ON af_admin_audit_log(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_af_admin_audit_log_actor
    ON af_admin_audit_log(actor_uuid, created_at DESC);
