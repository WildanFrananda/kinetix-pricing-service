# CLAUDE.md — kinetix-pricing-service

The workspace `AGENTS.md` holds the stack and the verification commands. This file holds decisions that the
code states but cannot explain.

## An unknown signing key is looked for at most every ten seconds

`JwtVerifier::verify_access` answers a token's `kid` from the keys it holds. A `kid` it does not hold makes it
fetch identity's JWKS again only if the last successful fetch is older than `UNKNOWN_KEY_REFRESH` (ten
seconds); otherwise the token is invalid. `JwksBreaker` opened only after a failed fetch, so until 2026-10-19
a stream of forged tokens with made-up key ids, while identity was up, made identity answer once per request.
A failed fetch still opens the breaker for `JWKS_COOLDOWN`, during which an unknown key is `Unavailable`, not
invalid.

## No promotion can raise a bill

`DefaultVoucherEvaluator` returns a discount between zero and the subtotal, and `DefaultDiscountEvaluator` a
saving between zero and the unit price, whatever a stored row says. `PricingService` bounds the voucher
discount again, because an evaluator is injected and need not be the default, and applies a voucher only when
it takes something off, so a voucher that cannot discount never spends its quota. A row with terms that could
not discount (value ≤ 0, a percentage over 100, a cap ≤ 0) is not eligible, and a flash price below zero is not
applied. Until 2026-10-21 a negative `max_discount` produced a negative discount and the bill grew by it.
`tests/evaluator_property_test.rs` holds both bounds and the whole bill to them; it failed on the old
evaluators.

A 100% discount prices the goods at zero and leaves the shipping fee: `CalculatePrice` then answers a subtotal
of 0 and a `final_total` equal to the shipping. Refusing a zero total is order's decision (O-19), not pricing's.

## A create refuses what the columns cannot hold

`services/promotion_terms.rs` checks every field before identity, catalog or Postgres are asked, and its limits
are the migration's: `NUMERIC(12,2)` holds less than 10^10 and two places, and an amount with more places is
refused rather than rounded (`10.000` is two places; `10.005` is three); `VARCHAR(n)` counts characters, not
bytes; Postgres stores no NUL in text, and no time before 4714-11-24 BC (`EARLIEST_STORABLE_UNIX_SECONDS`). A
window must end in the future and after it starts; a voucher must expire in the future. `CalculatePrice`
refuses a line with no units or a negative price.

## Creates are deduplicated by `Idempotency-Key`

Every create needs the header (at most 255 characters) and answers 201. The key, the caller's principal, the
serialized request and the new promotion's id go into `promotion_creations` in the same transaction as the
promotion. The same key with the same body answers 201 with the first promotion and `Idempotent-Replayed:
true`; with a different body, or for another kind of promotion, it is 422. Keys belong to the principal that
sent them. A create that fails, a taken voucher code included, leaves no ledger row, so its key stays free.
The body is compared as sent, so a retry must resend the same bytes, `expires_at` included. Two racing requests
with one key make one promotion: the second insert waits on the ledger's primary key.

Voucher codes are global across merchants (the user's ruling, 2026-10-21): a taken code is 409, never the
unique-violation 500 it was. They are also case-insensitive: `normalized_voucher_code` upper-cases a code on
create and on every lookup (GET, apply, `CalculatePrice`, `RedeemVoucher`, `ReleaseVoucherRedemption`), and the
unique index `vouchers_code_upper_key` on `upper(code)` keeps a second case of one code out even from a writer
that forgot to normalise.

## Who sees a voucher's quota

`GET /vouchers/{code}` and `POST /vouchers/apply` show a buyer only code, title, type, value, `min_spend`,
`max_discount`, `expires_at` and the merchant. An admin, or the seller whose merchant identity names as the
owner, sees the whole row, quota and use included. The token is optional on these reads: none, or one that does
not verify, reads as anonymous; a token pricing cannot check because identity's keys cannot be fetched is 503.
An exhausted voucher is not found on GET, as apply already treated it.

## A refusal keeps its reason

Rocket hands a failed request guard to the catcher for its status and drops the guard's error. The guards
therefore record a `GuardRefusal` (code and message) in the request's local cache, and the default catcher
answers with it: a JWKS outage reads `IDENTITY_UNAVAILABLE`, not "this request was refused".

Rocket 0.5 has no 405. The 404 catcher compares the path with every mounted route's template and answers 405
with `Allow` when the path exists under other methods.

## A peer's refusal is not an outage

`PeerError::from_status` makes catalog's or identity's `InvalidArgument` a refusal and every other failure
`Unavailable`; `NotFound` and `found = false` are absence. A product id catalog refuses is 400, an account
identity refuses is 403, and only an unreachable or failing peer is 503. An admin's promotion of a product now
asks catalog too, so a product catalog does not know is 404 for everyone.

## The flash-sale lookup is deterministic

`find_active_for_product` takes the units wanted and picks, among active sales in their window with that much
stock left, the lowest price, then the oldest, then the id. It used to take any row, sold out or not. A cart is
priced with `find_active_for_cart`, which considers only the platform's sales and the cart merchant's own, so a
seller's cheaper sale never hides a platform sale from another merchant's cart. The public read keeps every
owner: a product's sales all apply to its own merchant's cart.

## A redemption of an expired voucher is refused

`RedeemVoucher` answers `VOUCHER_EXPIRED` for a voucher past `expires_at` and `VOUCHER_NOT_FOUND` for one switched
off, which it used to call exhausted. A redemption the order already holds still replays as `already_redeemed`,
expired or not, so a saga retry never fails on the clock.

## No amount becomes zero on the wire

`to_money` answers `None` for an amount `Money` cannot carry; it used to send 0. An amount echoed from the
request is then `InvalidArgument`, a computed one `Internal`. `CalculatePrice` first refuses a cart whose lines
and caller fee add up past what `Money` carries, and `ShippingRate::fee_for` uses checked arithmetic, so a journey
too long or heavy to price is a 400 refusal instead of a panic.

## One error vocabulary

`error_code.rs` holds every `error` value this service writes: `AppError::code`, the catchers, the guards and
`JsonBody` use only those. A database pool that cannot hand out a connection is 503 `DATABASE_UNAVAILABLE`,
not 500. `JsonBody` replaces Rocket's `Json` as the data guard on every route that reads a body, so a body that
does not parse names the field (`quota: invalid type …`, `missing field \`title\``): 422 `INVALID_BODY`, or
400 `INVALID_JSON` for a body that is not JSON.
