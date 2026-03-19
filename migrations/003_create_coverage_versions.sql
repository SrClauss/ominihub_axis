CREATE TABLE IF NOT EXISTS coverage_versions (
    id SERIAL PRIMARY KEY,
    version_hash VARCHAR(64) UNIQUE NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);
