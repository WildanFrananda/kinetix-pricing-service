# 💰 Kinetix Pricing Service (`kinetix-pricing-service`)

High-performance microservice written in **Rust (Rocket 0.5 & SQLx)** responsible for calculating product prices, applying multi-tier discounts, managing voucher redemptions, and handling time-windowed flash sales for the Kinetix E-Commerce ecosystem.

---

## 🚀 Key Features

1. **Product Pricing Engine**: Calculates line-item totals and cart subtotals by prioritizing active Flash Sales, Category/Product Discounts, and Voucher redemptions.
2. **Discounts Engine**: Percentage & fixed value discounts targetable by product ID or category ID.
3. **Vouchers Engine**: Code-based claims with minimum spend threshold, maximum discount limit, and usage quotas.
4. **Flash Sales Engine**: Time-windowed flash sale prices with stock allocation limits.

---

## ⚡ Quick Start

```bash
# 1. Check Cargo Compilation
cargo check

# 2. Run Test Suite
cargo test

# 3. Run Service locally on Port 6000
cargo run
```
