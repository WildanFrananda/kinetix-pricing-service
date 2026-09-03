-- Reverses up.sql. Diesel requires a down for every migration; without it `diesel migration
-- revert` has nothing to run and the migration cannot be rolled back in an incident.
DROP TABLE IF EXISTS flash_sales;
DROP TABLE IF EXISTS vouchers;
DROP TABLE IF EXISTS discounts;
