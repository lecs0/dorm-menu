# API schema 1

Menu endpoints require `Authorization: Bearer TOKEN`. TLS terminates at the public tunnel; the local API is HTTP and should remain private.

| Route | Response |
|---|---|
| `GET /health` | Public locally, liveness only: `{"status":"ok"}`; no NAS/source details. Tunnel example does not publish it. |
| `GET /api/v1/menus` | Snapshot envelope with schema version, Budapest `as_of`, source check/update times, source status, menus, current/next selections, unpublished entries |
| `GET /api/v1/menus/current` | Selected menu, or latest expired fallback; 404 if neither exists |
| `GET /api/v1/menus/next` | Earliest future published menu; 404 otherwise |

First start returns 200 from the collection with empty menus, null selections and `source_status: unavailable` (or `error` if its check failed). Collection menus contain `status`: `current`, `upcoming`, `expired`, or `unknown_validity`. An absent item is unavailable; unlinked entries appear separately in `unpublished`, never as invented meals. Statuses follow the inclusive actual start/end dates. An expired fallback is explicitly marked expired.

Menu fields: stable `menu_id`, PDF SHA-256 `version`, institution attribution, full page entry with date evidence and source URL, source page, retrieval/update times, days, notes. Meals preserve original text, visual-line components, raw source allergen code groups and salt text. `source_codes_only` means codes were found, **not** that meanings/completeness were confirmed; otherwise `unknown`. Calendar dates come from page evidence; graphical PDF-header checks are a documented limitation.

The complete Rust schema is in `shared/src/lib.rs`. Clients must check `schema_version`; cached data with an unsupported schema is rejected. Unknown extra JSON fields are allowed.

Successful menu responses include a quoted SHA-256 ETag, `Cache-Control: private, no-cache`, and `Vary: Authorization`. Send `If-None-Match` for 304. Weak/comma-separated tags are supported. Authentication and rate limiting run before conditional handling, so revocation also applies to cached requests.

Errors are small JSON objects with an `error` code: 401 invalid token (also WWW-Authenticate), 404 unavailable selection, 429 rate limited (per-token responses include Retry-After), 413 body not allowed, 408 handler timeout, or 503 unavailable oversized data. GET bodies are refused. Response cap is 4 MB. The collection is the recommended app endpoint; it avoids extra calls when navigating days or menus.

Rate limits default to 30 authenticated requests/minute/device plus a global cap ten times that, including invalid tokens. Counters expire each minute and have bounded cardinality. They reset on restart. This is lightweight single-instance protection, not a distributed limiter. No forwarded IP headers are trusted or used; clients behind the tunnel are limited by token, not spoofable client addresses. Add Cloudflare rate/WAF controls for internet-scale abuse if available under your plan. Logs include source-check events and sanitized public errors; authorization headers are never logged by this implementation.
