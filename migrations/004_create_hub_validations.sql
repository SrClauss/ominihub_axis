CREATE TABLE IF NOT EXISTS hub_validations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id),
    location GEOGRAPHY(Point, 4326),
    detected_hub_id UUID REFERENCES hubs(id),
    confirmed BOOLEAN,
    created_at TIMESTAMPTZ DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_validations_user ON hub_validations(user_id);
CREATE INDEX IF NOT EXISTS idx_validations_hub ON hub_validations(detected_hub_id);
CREATE INDEX IF NOT EXISTS idx_validations_created ON hub_validations(created_at);
