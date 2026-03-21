CREATE TABLE IF NOT EXISTS admin_hub_access (
    admin_id UUID NOT NULL REFERENCES admins(id) ON DELETE CASCADE,
    hub_id UUID NOT NULL REFERENCES hubs(id) ON DELETE CASCADE,
    granted_at TIMESTAMPTZ DEFAULT NOW(),

    PRIMARY KEY (admin_id, hub_id)
);

CREATE INDEX IF NOT EXISTS idx_admin_access_hub ON admin_hub_access(hub_id);
