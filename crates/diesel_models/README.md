# Storage Models

Database models shared across `router` and other crates.

## Blocklist query selection

`crates/router/src/db/blocklist.rs` selects the payment, payout, or all-namespace
query before calling Diesel. The query layer exposes concrete namespace methods
for lookups, lists, counts, export/clone pagination, and deletes.

Payment queries include explicit `payment` rows and legacy NULL transaction types;
payout queries match only explicit `payout` rows. Unfiltered fingerprint lookup
uses the all-namespace query. Inserted rows carry their transaction type directly.
