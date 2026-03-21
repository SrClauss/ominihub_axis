CREATE TABLE IF NOT EXISTS blocked_entities (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_type VARCHAR(20) NOT NULL CHECK (entity_type IN ('user', 'driver')),
    entity_id UUID NOT NULL,
    blocked_by UUID,
    reason TEXT NOT NULL,
    blocked_at TIMESTAMPTZ DEFAULT NOW(),
    expires_at TIMESTAMPTZ,
    hub_scope UUID[],

    UNIQUE(entity_type, entity_id)
);

CREATE INDEX IF NOT EXISTS idx_blocked_type_id ON blocked_entities(entity_type, entity_id);
CREATE INDEX IF NOT EXISTS idx_blocked_expires ON blocked_entities(expires_at) WHERE expires_at IS NOT NULL;
