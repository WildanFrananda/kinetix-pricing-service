# AGENTS.md — Kinetix Pricing Service (`kinetix-pricing-service`)

This file guides AI coding agents working on the **Kinetix Pricing Service**.

---

## 🏛️ Stack & Architecture

- **Language**: **Rust 1.98+**
- **Web Framework**: **Rocket 0.5** (async Tokio runtime)
- **Database**: **PostgreSQL 16** (`kinetix_pricing_dev`) via **SQLx 0.7**
- **Port**: `:6000`
- **Mandatory Code Directive**:
  - **EXPLICIT RETURN MANDATE**: All Rust functions MUST explicitly use the `return` keyword for returning values (no implicit tail expressions).
- **Core Domain Responsibilities**:
  1. Product Pricing Calculation Engine (Flash Sale priority > Category/Product/Global Discount > Voucher Code deduction).
  2. Discounts Management.
  3. Vouchers Management & Quota Guard.
  4. Flash Sales Management & Time Window Guard.

---

## 🧪 Verification Commands

- **Check Code Compilation**: `cargo check`
- **Run Unit & Integration Tests**: `cargo test`
- **Build Release Binary**: `cargo build --release`