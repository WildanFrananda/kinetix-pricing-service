-- The quota ledger: which order consumed which voucher, and which flash-sale stock.
--
-- vouchers.used_count and flash_sales.stock_sold have existed since the first migration and
-- nothing ever incremented them, so every cap this service advertises has been unenforced. A
-- counter alone cannot be fixed by incrementing it either: a saga has to be able to give quota
-- back when a later step fails, and `used_count - 1` cannot tell whether this order ever took
-- one, or whether a retry already gave it back. So the ledger records the fact, and the counter
-- becomes a cache of it.
--
-- The unique constraints are the idempotency. A repeated RedeemVoucher for the same order hits
-- the constraint rather than counting twice, which is what the contract's `already_redeemed`
-- flag is there to report.

CREATE TABLE voucher_redemptions (
    id UUID PRIMARY KEY,
    voucher_code VARCHAR(100) NOT NULL,
    order_number VARCHAR(100) NOT NULL,
    customer_principal_id VARCHAR(64) NOT NULL,
    -- NULL while the quota is held. Set when a compensation hands it back, so a second release
    -- is a no-op rather than a second refund.
    released_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT uq_voucher_redemptions_code_order UNIQUE (voucher_code, order_number)
);

CREATE INDEX ix_voucher_redemptions_order_number ON voucher_redemptions (order_number);

CREATE TABLE flash_sale_allocations (
    id UUID PRIMARY KEY,
    flash_sale_id UUID NOT NULL,
    product_id VARCHAR(100) NOT NULL,
    quantity INT NOT NULL,
    order_number VARCHAR(100) NOT NULL,
    released_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT uq_flash_sale_allocations_sale_order UNIQUE (flash_sale_id, order_number)
);

CREATE INDEX ix_flash_sale_allocations_order_number ON flash_sale_allocations (order_number);
