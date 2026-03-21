CREATE TABLE IF NOT EXISTS hub_status_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    hub_id UUID NOT NULL REFERENCES hubs(id) ON DELETE CASCADE,
    old_status VARCHAR(20),
    new_status VARCHAR(20) NOT NULL,
    reason TEXT,
    changed_by UUID REFERENCES admins(id),
    changed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_hub_status_history_hub_id ON hub_status_history(hub_id);
CREATE INDEX IF NOT EXISTS idx_hub_status_history_changed_at ON hub_status_history(changed_at);
