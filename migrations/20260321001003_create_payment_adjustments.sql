CREATE TABLE IF NOT EXISTS payment_adjustments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    payment_id UUID NOT NULL REFERENCES franchise_payments(id) ON DELETE CASCADE,
    adjustment_type VARCHAR(20) NOT NULL CHECK (adjustment_type IN ('discount', 'penalty', 'credit')),
    amount DOUBLE PRECISION NOT NULL,
    reason TEXT NOT NULL,
    created_by UUID REFERENCES admins(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_payment_adjustments_payment_id ON payment_adjustments(payment_id);
