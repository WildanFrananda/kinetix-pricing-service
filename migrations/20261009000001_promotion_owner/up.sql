-- Who a promotion belongs to.
--
-- The owner is the merchant principal identity names for the seller — the same key catalog stores on
-- each product and order sends with each cart — and a seller's promotion applies only to that
-- merchant's goods. NULL means the platform: an admin's promotion, which applies to every cart.
--
-- Nothing recorded who created the rows that exist when this runs, so they become platform
-- promotions; review them after deploying.

ALTER TABLE discounts ADD COLUMN merchant_principal_id VARCHAR(100);
ALTER TABLE vouchers ADD COLUMN merchant_principal_id VARCHAR(100);
ALTER TABLE flash_sales ADD COLUMN merchant_principal_id VARCHAR(100);

CREATE INDEX ix_discounts_merchant_principal_id ON discounts (merchant_principal_id);
CREATE INDEX ix_vouchers_merchant_principal_id ON vouchers (merchant_principal_id);
CREATE INDEX ix_flash_sales_merchant_principal_id ON flash_sales (merchant_principal_id);
