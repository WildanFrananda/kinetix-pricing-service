CREATE TABLE promotion_creations (
    principal_id TEXT NOT NULL,
    idempotency_key VARCHAR(255) NOT NULL,
    promotion_kind VARCHAR(32) NOT NULL,
    promotion_id UUID NOT NULL,
    request_body TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (principal_id, idempotency_key),
    CONSTRAINT promotion_creations_known_kind CHECK (
        promotion_kind IN ('DISCOUNT', 'VOUCHER', 'FLASH_SALE')
    )
);
