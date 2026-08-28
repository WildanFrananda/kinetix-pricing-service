// @generated automatically by Diesel CLI.

diesel::table! {
    discounts (id) {
        id -> Uuid,
        title -> VarChar,
        discount_type -> VarChar,
        value -> Numeric,
        target_product_id -> Nullable<VarChar>,
        target_category_id -> Nullable<VarChar>,
        active -> Bool,
        start_time -> Timestamptz,
        end_time -> Timestamptz,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    vouchers (id) {
        id -> Uuid,
        code -> VarChar,
        title -> VarChar,
        discount_type -> VarChar,
        value -> Numeric,
        min_spend -> Numeric,
        max_discount -> Nullable<Numeric>,
        quota -> Int4,
        used_count -> Int4,
        active -> Bool,
        expires_at -> Timestamptz,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    flash_sales (id) {
        id -> Uuid,
        title -> VarChar,
        product_id -> VarChar,
        flash_price -> Numeric,
        stock_limit -> Int4,
        stock_sold -> Int4,
        active -> Bool,
        start_time -> Timestamptz,
        end_time -> Timestamptz,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    discounts,
    vouchers,
    flash_sales,
);
