CREATE TABLE IF NOT EXISTS franchise_notifications (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    hub_id UUID NOT NULL REFERENCES hubs(id) ON DELETE CASCADE,
    notification_type VARCHAR(50) NOT NULL CHECK (notification_type IN (
        'payment_due', 'payment_overdue', 'grace_period_started',
        'restricted_mode', 'suspended', 'payment_received', 'payment_failed'
    )),
    message TEXT NOT NULL,
    metadata JSONB,
    sent_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    read_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_franchise_notifications_hub_id ON franchise_notifications(hub_id);
CREATE INDEX IF NOT EXISTS idx_franchise_notifications_read_at ON franchise_notifications(read_at);
