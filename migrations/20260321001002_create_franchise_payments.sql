CREATE TABLE IF NOT EXISTS franchise_payments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    hub_id UUID NOT NULL REFERENCES hubs(id) ON DELETE CASCADE,
    due_date DATE NOT NULL,
    amount DOUBLE PRECISION NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'paid', 'overdue', 'cancelled')),
    paid_at TIMESTAMPTZ,
    payment_method VARCHAR(50),
    transaction_id VARCHAR(100),
    gateway_payment_url TEXT,
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_franchise_payments_hub_id ON franchise_payments(hub_id);
CREATE INDEX IF NOT EXISTS idx_franchise_payments_status ON franchise_payments(status);
CREATE INDEX IF NOT EXISTS idx_franchise_payments_due_date ON franchise_payments(due_date);
