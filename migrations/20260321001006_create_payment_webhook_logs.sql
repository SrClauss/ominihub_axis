CREATE TABLE IF NOT EXISTS payment_webhook_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    gateway VARCHAR(50) NOT NULL DEFAULT 'mercadopago',
    event_type VARCHAR(100),
    payload JSONB NOT NULL,
    processed BOOLEAN NOT NULL DEFAULT false,
    error_message TEXT,
    payment_id UUID REFERENCES franchise_payments(id),
    received_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    processed_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_payment_webhook_logs_received_at ON payment_webhook_logs(received_at);
CREATE INDEX IF NOT EXISTS idx_payment_webhook_logs_processed ON payment_webhook_logs(processed);
