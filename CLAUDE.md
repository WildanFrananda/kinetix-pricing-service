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
