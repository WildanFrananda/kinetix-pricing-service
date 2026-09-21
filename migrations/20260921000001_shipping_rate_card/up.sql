-- The shipping rate card, in the service that owns money.
--
-- These four rates lived in matching's source until 2026-09-21 — `15_000 + dist_km * 3_000` and
-- three more like it, in `lib/fleet_pulse/shipping.ex`, with `shipping_server.ex` turning the
-- result into a `Money`. A service whose job is dispatching couriers was the authority for a
-- money field in a checkout, and the only way to change a price was a release of that service.
-- (docs/BOUNDARY-DEBT.md M1)
--
-- What matching keeps is what it actually knows: the distance, whether its fleet can make the
-- journey, and how long it takes. What arrives here is a tier, a distance and a weight.
--
-- One row per tier, and the fee is
--
--     base_fee
--   + per_km_fee            * distance_km
--   + per_kg_fee            * weight_kg
--   + per_kg_per_100km_fee  * weight_kg * hundred_km_units
--
-- where hundred_km_units = max(1.0, round(distance_km / 100, 1)). Every term is zero for the
-- tiers that do not use it, so the four formulas that were four functions are one expression and
-- a row each. The figures below are exactly what matching charged, to the rupiah.
CREATE TABLE shipping_rates (
    service_tier VARCHAR(64) PRIMARY KEY,
    base_fee NUMERIC(14, 2) NOT NULL,
    per_km_fee NUMERIC(14, 2) NOT NULL DEFAULT 0,
    per_kg_fee NUMERIC(14, 2) NOT NULL DEFAULT 0,
    per_kg_per_100km_fee NUMERIC(14, 2) NOT NULL DEFAULT 0,
    currency CHAR(3) NOT NULL DEFAULT 'IDR',
    -- A tier can be withdrawn from sale without deleting its history.
    active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT shipping_rates_no_negative_terms CHECK (
        base_fee >= 0
        AND per_km_fee >= 0
        AND per_kg_fee >= 0
        AND per_kg_per_100km_fee >= 0
    )
);

INSERT INTO shipping_rates
    (service_tier, base_fee, per_km_fee, per_kg_fee, per_kg_per_100km_fee)
VALUES
    ('KINETIX_INSTANT', 15000.00, 3000.00, 0.00, 0.00),
    ('KINETIX_SAMEDAY', 12000.00, 2000.00, 0.00, 0.00),
    ('KINETIX_REGULAR', 9000.00, 0.00, 0.00, 1500.00),
    ('KINETIX_CARGO', 25000.00, 0.00, 1000.00, 0.00);
