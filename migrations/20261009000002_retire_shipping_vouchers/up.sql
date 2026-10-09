-- Vouchers discount goods only. A voucher of a kind this service does not price, or one whose code made
-- it a shipping voucher, is switched off rather than left to be read as a discount on the goods.
UPDATE vouchers
SET active = false,
    updated_at = now()
WHERE active
  AND (upper(trim(discount_type)) NOT IN ('PERCENTAGE', 'FIXED') OR code LIKE '%FREE_SHIP%');
