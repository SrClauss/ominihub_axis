CREATE TABLE IF NOT EXISTS hub_status (
    hub_id UUID PRIMARY KEY REFERENCES hubs(id) ON DELETE CASCADE,
    operational_status VARCHAR(20) NOT NULL CHECK (
        operational_status IN ('active', 'grace', 'restricted', 'suspended', 'terminated')
    ) DEFAULT 'active',
    restriction_reason TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);
