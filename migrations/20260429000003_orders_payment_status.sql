-- Marketplace orders: track how the buyer paid (wallet vs unpaid awaiting card gateway).
ALTER TABLE orders
    ADD COLUMN IF NOT EXISTS payment_status VARCHAR(20) NOT NULL DEFAULT 'unpaid';

COMMENT ON COLUMN orders.payment_status IS 'unpaid | wallet_paid | external_paid';
