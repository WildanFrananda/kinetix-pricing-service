# 💰 Kinetix Pricing Service (`kinetix-pricing-service`)

High-performance microservice written in **Rust 1.98 (Rocket 0.5 & Tonic gRPC & SQLx)** responsible for calculating product prices, applying multi-tier discounts, managing voucher redemptions, and handling time-windowed flash sales for the Kinetix E-Commerce ecosystem.

---

## 🏛️ Architecture & Ports

- **Port `:6000` (HTTP REST Admin)**:
  - Admin/Merchant promo management endpoints (`POST /api/v1/discounts`, `POST /api/v1/vouchers`, `POST /api/v1/flash-sales`, `GET /health`).
  - Protected by `AdminOrMerchantGuard` (validates `X-User-Role: ADMIN` or `X-User-Role: MERCHANT` from Kong API Gateway).
- **Port `:50054` (gRPC Protobuf)**:
  - High-performance inter-service price calculation server (`PricingService` RPC `CalculatePrice`).
  - Used directly by backend microservices like `kinetix-catalog-service` over Protobuf / HTTP/2.

---

## 🚀 Key Features

1. **Pricing Calculation Engine (gRPC :50054)**: Calculates line-item totals and cart subtotals by prioritizing active Flash Sales (#1), Category/Product Discounts (#2), and Voucher redemptions (#3).
2. **Discounts Engine**: Percentage & fixed value discounts targetable by product ID, category ID, or global catalog.
3. **Vouchers Engine**: Code-based claims with minimum spend threshold, maximum discount limit, usage quotas, and expiry guards.
4. **Flash Sales Engine**: Time-windowed flash sale prices with atomic stock allocation limits.
5. **Trait-Based Dependency Injection**: Decoupled repository ports (`DiscountRepositoryPort`, `VoucherRepositoryPort`, `FlashSaleRepositoryPort`) and strategy evaluators (`DiscountEvaluator`, `VoucherEvaluator`).
6. **Zero Hardcoded Secrets & Fail-Fast**: Mandatory startup validation requiring `DATABASE_URL`.

---

## ⚡ Quick Start

```bash
# 1. Check Cargo Compilation (0 Errors, 0 Warnings)
cargo check

# 2. Run Unit Test Suite
cargo test

# 3. Run Service locally on Port 6000 (REST) and :50054 (gRPC)
DATABASE_URL=postgres://postgres:postgres@localhost:5432/kinetix_pricing_dev cargo run
```
