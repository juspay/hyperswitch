# SPEC — Normalize wallet card_network so the payments-list filter matches

## Problem statement
In the V1 payments list, wallet transactions (Apple Pay / Google Pay / Samsung Pay)
display their card network as the raw provider string (e.g. `AmEx`, `AMEX`), but
filtering the list by "American Express" returns none of them. The POST payments
list filter matches on the `payment_attempt.card_network` **column** (a canonical
`CardNetwork` enum, `dsl::card_network.eq_any(...)`). That column is only written
for card payments — for wallets it stays NULL — so wallet rows can never match a
card-network filter, and the value surfaced to the UI is a non-canonical string.

## Root cause
`payment_attempt.card_network` is populated at write time from
`PaymentAttempt::extract_card_network()` → `AdditionalPaymentData::get_additional_card_info()`,
which only matches the `Card` variant and returns `None` for the `Wallet` variant.
So wallets never write the filterable column.

- `crates/api_models/src/payments.rs` — `AdditionalPaymentData::get_additional_card_info()` (Card-only).
- `crates/hyperswitch_domain_models/src/payments/payment_attempt.rs` — V1 `extract_card_network()`.
- Filter match: `crates/diesel_models/src/query/payment_attempt.rs:478,537` and
  `crates/storage_impl/src/payments/payment_intent.rs:1276,1500` — `card_network.eq_any(...)`.

## Acceptance criteria
- For wallet payments, `extract_card_network()` returns the canonical `CardNetwork`
  enum, normalized case-insensitively (`AmEx`/`AMEX`/`amex` → `AmericanExpress`).
- The `payment_attempt.card_network` column is written for new wallet payments.
- Filtering the payments list by a card network returns matching wallet payments.
- Card-payment behaviour is unchanged (card path evaluated first).
- `cargo check --features v1` passes.

## Affected files
- `crates/api_models/src/payments.rs` — add `AdditionalPaymentData::get_wallet_card_network()`:
  reads the wallet provider network string (google_pay/samsung_pay `card_network`,
  apple_pay `network`) and normalizes it to `CardNetwork` via its UPPERCASE serde
  aliases. Case-insensitive, no bespoke mapping table.
- `crates/hyperswitch_domain_models/src/payments/payment_attempt.rs` — V1
  `extract_card_network()` now falls back to `get_wallet_card_network()` when the
  card path yields nothing.

## Risks & out of scope
- Write-path only: fixes **new** wallet payments. Existing rows have a NULL
  `card_network` column and need a separate backfill to be filterable — out of scope.
- V2 `extract_card_network()` (`todo!()`) is untouched.
- No API schema change; `WalletAdditionalDataForCard.card_network` stays `Option<String>`.

## Test plan
- `cargo check -p api_models -p hyperswitch_domain_models --features v1` — GREEN.
- Manual: create a Google Pay / Apple Pay payment, confirm `payment_attempt.card_network`
  is set, then filter the payments list by that network and confirm the payment appears.
