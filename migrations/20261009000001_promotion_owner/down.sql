DROP INDEX ix_flash_sales_merchant_principal_id;
DROP INDEX ix_vouchers_merchant_principal_id;
DROP INDEX ix_discounts_merchant_principal_id;

ALTER TABLE flash_sales DROP COLUMN merchant_principal_id;
ALTER TABLE vouchers DROP COLUMN merchant_principal_id;
ALTER TABLE discounts DROP COLUMN merchant_principal_id;
