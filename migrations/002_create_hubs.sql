CREATE TABLE IF NOT EXISTS hubs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(100) NOT NULL,
    slug VARCHAR(50) UNIQUE NOT NULL,
    boundary GEOMETRY(Polygon, 4326) NOT NULL,
    api_url VARCHAR(255) NOT NULL,
    admin_email VARCHAR(255),
    status VARCHAR(20) DEFAULT 'offline' CHECK (status IN ('online', 'offline', 'maintenance')),
    last_heartbeat TIMESTAMPTZ,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_hubs_boundary ON hubs USING GIST(boundary);
CREATE INDEX IF NOT EXISTS idx_hubs_status ON hubs(status);
CREATE INDEX IF NOT EXISTS idx_hubs_slug ON hubs(slug);
