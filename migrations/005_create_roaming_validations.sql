CREATE TABLE IF NOT EXISTS roaming_validations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    driver_id UUID REFERENCES users(id),
    origin_hub_id UUID REFERENCES hubs(id),
    target_hub_id UUID REFERENCES hubs(id),
    allowed BOOLEAN,
    created_at TIMESTAMPTZ DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_roaming_val_driver ON roaming_validations(driver_id);
