UPDATE vouchers
SET code = upper(code),
    updated_at = now()
WHERE code <> upper(code);

UPDATE voucher_redemptions
SET voucher_code = upper(voucher_code)
WHERE voucher_code <> upper(voucher_code);

CREATE UNIQUE INDEX vouchers_code_upper_key ON vouchers (upper(code));
