-- Pricing's schema, reconciled with src/schema.rs.
--
-- These tables were created without five of the columns Diesel declares, and with three of them
-- in a different order. The service started, reported healthy, and answered 500 to every request
-- that touched a table:
--
--   GET /api/v1/discounts -> {"message":"Database error: column discounts.start_time does not exist"}
--
-- The missing columns were discounts.start_time, discounts.end_time, and updated_at on all three
-- tables. The order differences were target_product_id/target_category_id in discounts,
-- active/expires_at in vouchers, and the position of active in flash_sales. Order matters here
-- because the models derive Selectable against src/schema.rs, and Postgres cannot reorder columns
-- with ALTER — so this migration was corrected in place rather than patched by a second one. It
-- had been applied exactly once, in the local lab, on the day it was written.
--
-- src/schema.rs is the source of truth: it is what the Rust code compiles against. Column names,
-- order, types and nullability below are a transcription of it.

CREATE TABLE discounts (
    id UUID PRIMARY KEY,
    title VARCHAR(255) NOT NULL,
    discount_type VARCHAR(50) NOT NULL,
    value NUMERIC(12, 2) NOT NULL,
    target_product_id VARCHAR(100),
    target_category_id VARCHAR(100),
    active BOOLEAN NOT NULL DEFAULT TRUE,
    start_time TIMESTAMPTZ NOT NULL,
    end_time TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE vouchers (
    id UUID PRIMARY KEY,
    code VARCHAR(100) NOT NULL UNIQUE,
    title VARCHAR(255) NOT NULL,
    discount_type VARCHAR(50) NOT NULL,
    value NUMERIC(12, 2) NOT NULL,
    min_spend NUMERIC(12, 2) NOT NULL DEFAULT 0.00,
    max_discount NUMERIC(12, 2),
    quota INT NOT NULL DEFAULT 100,
    used_count INT NOT NULL DEFAULT 0,
    active BOOLEAN NOT NULL DEFAULT TRUE,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE flash_sales (
    id UUID PRIMARY KEY,
    title VARCHAR(255) NOT NULL,
    product_id VARCHAR(100) NOT NULL,
    flash_price NUMERIC(12, 2) NOT NULL,
    stock_limit INT NOT NULL,
    stock_sold INT NOT NULL DEFAULT 0,
    active BOOLEAN NOT NULL DEFAULT TRUE,
    start_time TIMESTAMPTZ NOT NULL,
    end_time TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);
