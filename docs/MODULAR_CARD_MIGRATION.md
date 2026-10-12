# Modular batch card migration

This is the optional branch of `POST /payment_methods/migrate-batch`. Omitted or
false `use_modular_customer_creation` keeps the existing legacy implementation
and response unchanged. The hotfix PR targets `hotfix-2026.09.30.0`; the matching
main PR contains the same migration changes. Confirm the actual deployed EU
image and complete the remaining local/CUG checks before deployment.

## Request and storage

Use the existing multipart request and admin authentication. Add
`use_modular_customer_creation=true` and the secret `customer_api_key` containing
a temporary merchant key with an expiry. Existing key-management APIs mint and
revoke that key. Customer requests use `Authorization: api-key=<merchant key>`,
the profile derived from the row's mapped MCA, and the configured tenant.

The CSV needs the merchant customer reference in `customer_id`, customer `name`
and `email`, raw PAN in `raw_card_number`, and card expiry. Cardholder name stays
separate from customer name. All supplied billing fields and card nickname/network
are preserved. Expiry is normalized to a two-digit month and four-digit year.

Card storage uses the existing modular PM service URL/prefix and internal
merchant/profile authentication. Its payload always uses the generated modular
customer ID and `storage_type=persistent`. CVC is absent; it is never fabricated.
No change to customer API authentication is needed.

Unknown/duplicate CSV columns are rejected in this branch. Populated bank,
network-token, connector-token/mandate, original-transaction, masked-PAN and phone
fields are explicitly unsupported. Non-card methods and subtypes other than
credit/debit are rejected. This implementation does not pretend to migrate
unsupported fields.

## Results and retries

Every modular row returns `line_number` (the record's position in the submitted
CSV), `merchant_reference_id`, generated `customer_id`, resolved `profile_id`,
`payment_method_id`, and `customer_creation_flow=modular`. Failed stages retain
already-resolved customer information and return structured `errors` with a
stage, code, retryability, uncertainty flag and relevant field names. No PAN,
CVC, name, email, billing values or API key are returned in errors.

Success requires confirmation through the saved-PM list with `include_new=true` and raw-detail
retrieve contracts: merchant/customer identity, full PAN, permanent storage,
expiry and supplied metadata/billing must match. Last four is only a prefilter.
Multiple matching cards or conflicting supplied metadata fail explicitly.
The modular service stores a freshly vaulted card as `New` until it is used;
the migration-specific list includes those cards as well as `Active` cards.
Ordinary saved-card listings retain their existing behavior.
Rows without an explicit credit/debit subtype require BIN metadata from the
router's existing card-info store. Missing or unsupported metadata returns
`migration_card_subtype_unavailable` before customer/card creation. The error is
retryable after fixing the metadata; the migration never guesses a subtype.

Customer-created/card-failed retries reuse the customer reference's generated ID.
Existing customers must match the supplied customer name/email; conflicts are
reported explicitly rather than silently changing the customer or storing cards.
Migration requests for the same merchant/customer/PAN are serialized, including
across profiles; modular payment methods are customer-scoped. A batch also caches
duplicate rows, returning each source line with the same confirmed PM ID.

Before POST, a non-expiring Redis attempt record is saved. Its key and payload
digest use keyed hashing; its value contains only the digest and optional PM ID.
`migration_operation_id` identifies this record for operational reconciliation.
Pending attempts survive the short lock's expiry. Lost responses, cancellation
and journal-write failures after POST cannot trigger another blind POST.
Explicit authentication rejections (401/403 before the modular create handler)
clear the pending attempt, allowing a safe retry after credentials are corrected.
Other error statuses are not assumed to prove that storage failed.

Migration customer/card creation opts out of the HTTP client's automatic replay
after a connection closes without a complete response. Such writes must be
reconciled using the attempt record. The opt-out defaults to false for all other
callers, preserving their existing retry policy.

Customer creation also writes a non-expiring `MODULAR_CUSTOMER_MIGRATION_*`
pending marker before POST. Its value contains only the optional generated customer
ID; no names, emails, cards or credentials. The ID is recorded from a validated
create response before waiting for list confirmation. A later response cannot
rebind the reference to another customer ID before card storage.
After a lost response/cancellation, an absent customer remains reconciliation-only
even after the lock expires. A visible matching customer can be reused; definite
customer authentication rejections clear the marker for credential renewal.

If a pending attempt becomes visible, retry confirms it. If it never becomes
visible, retry remains reconciliation-only and reports
`migration_card_storage_uncertain`. `retryable=true` permits reconciliation; it
does not authorize a second create. Operators must establish the upstream outcome
before clearing an unresolved record. Do not clear a record merely because the
lock expired or a saved-card list was empty.

## Deployment checks still required

- CUG must expose create, saved-PM listing with `include_new=true`, and raw-detail retrieval on an
  OLTP-enabled modular service. Listing is also gated by the OLAP build feature;
  a replica-only instance is unsuitable for confirming writes.
- Confirm internal service credentials, tenant propagation, billing/cardholder
  round trips, and temporary key mint/revoke in local/CUG.
- Both customer and card Redis attempt records must survive restarts/failover and must not be evicted.
  The configured lock lease must exceed the bounded per-stage execution budget.
  Loss of pending records removes protection against unresolved older writes.
  Keep the API-key hashing secret stable across a run and its resumes: journal
  identifiers use keyed hashing independently of temporary merchant keys.
- Migration locks coordinate this migration branch. Unrelated customer/PM writers
  do not acquire them. Do not concurrently create the same references/cards through
  another flow during the migration; there is no service-wide exactly-once claim.
- The matching migration-tool branch now implements enriched CSV/flag/key lifecycle
  and result handling. Its `MODULAR_INTEGRATION_TESTS.md` contains generated
  synthetic fixtures, fault proxies and the exact local/CUG sequence. Live
  end-to-end testing still precedes any production run.

## Milestone validation

The combined review passed 51 focused v1 tests, two v2 request-contract tests and
three API-key expiration serialization tests. It covers
legacy flag selection, secret handling, unsupported inputs, field preservation,
customer reuse after card failure, duplicate/concurrent cards, lost responses,
cancellation, customer retry after lock expiry, generated-ID binding across delayed
confirmation, existing customer conflicts,
and journal failures. These use fake backends; they do not replace
local/CUG service integration tests.

Compile checks passed for normal v1/v2 router test targets and the production
`release,v1,redis-rs` and `release,v2,redis-rs` binary feature combinations. These are
`cargo check` results, not a deployed image or a completed production migration.
The existing legacy migration handler body and ordinary modular create converter
were also compared against the branch base and remain unchanged.

## Local live validation and remaining checks

Isolated local v1/v2 routers, a real test vault, Postgres and Redis were exercised
with synthetic cards and customers. Both mapped profiles, new/existing customers,
duplicate and concurrent cards, permanent no-CVC storage, key rejection,
renewal/revocation, partial failures, lost batch responses and resume passed.
Legacy migration also passed against that backend. The migration-specific
`include_new` listing and missing-BIN preflight fixes were verified live; 45
router tests and 11 payment-method tests passed after those fixes.

A later dropped-card-response check exposed automatic transport replay. The
opt-out above passes the v1 binary-target compile check, but its new socket test
and rebuilt runtime verification are still pending. Earlier feature-combination
checks do not validate this latest transport change. A stored-card payment
attempt returned HTTP 501 (`IR_00`, `Payment methods is not implemented`); its
cause remains unresolved. The delayed-card-commit-after-lock-expiry case is also
pending. These are integration sign-off blockers, not successful payment evidence.

PR #14589 supplies the merchant-reference resolution used by the intended v1
payment flow. It is still open against main, and was reported separately deployed
as an EU hotfix. Verify its availability in the actual target builds. No customer
API authentication change or new key-management endpoint is required.
