# SPEC — Wallet `card_network` missing in payments **list** API (V1)

## 1. Problem statement
The V1 payments list API (`GET /payments/list`, used by Control Center) returns
`card_network` as `null` for wallet transactions (Google Pay / Apple Pay / Samsung
Pay), even though the payments **retrieve** API (`GET /payments/{id}`) correctly
exposes the wallet's underlying `card_network`. Plain card payments show
`card_network` in both. The divergence is purely backend.

## 2. Root cause
Both flows build `payment_method_data` from the same stored column
(`payment_attempt.payment_method_data`, persisted as `AdditionalPaymentData`), but:

- **Retrieve** (`crates/router/src/core/payments/transformers.rs:~3877-3900`): parses
  the column as `AdditionalPaymentData`, then converts via
  `PaymentMethodDataResponse::from(...)`. That `From` impl
  (`crates/api_models/src/payments.rs:~9266`) maps the wallet's
  `apple_pay.network` / `google_pay` into `WalletAdditionalDataForCard.card_network`.
- **List** (`transformers.rs`, V1 `ForeignFrom<(PaymentIntent, PaymentAttempt)> for
  PaymentsResponse`): deserialized the **same raw column directly** into
  `PaymentMethodDataResponseWithBilling`, skipping the `AdditionalPaymentData` →
  response conversion. For the **Card** variant the serde field names coincide so
  `card_network` survives; for the **Wallet** variant the shapes differ
  (`AdditionalPaymentData::Wallet { apple_pay, google_pay, samsung_pay }` vs the
  externally-tagged `WalletResponseData`), so the parse drops the wallet data and
  `card_network` becomes `null`.

## 3. Acceptance criteria
- V1 `/payments/list` returns the wallet's `card_network` (nested under
  `payment_method_data.wallet.<wallet>.card_network`) for Google Pay / Apple Pay /
  Samsung Pay payments — matching the retrieve response for the same payment.
- Card payments in the list are unchanged.
- No API contract / schema change; field already exists in `PaymentsResponse`.

## 4. Affected files
- `crates/router/src/core/payments/transformers.rs` — in the V1
  `ForeignFrom<(storage::PaymentIntent, storage::PaymentAttempt)> for
  api::PaymentsResponse`, compute `payment_method_data` by parsing the stored value
  as `AdditionalPaymentData` (via
  `check_and_get_payment_method_data_based_on_encryption_strategy()`, honouring the
  encryption strategy like retrieve) and converting through
  `PaymentMethodDataResponse::from`, instead of deserializing directly into
  `PaymentMethodDataResponseWithBilling`.

## 5. Data model / API changes
None. Response shape and field names are identical; only the wallet `card_network`
value is now populated.

## 6. Risks & out-of-scope
- Frontend (Control Center) requires no change — it already reads `card_network`
  from the list payload; the value was simply `null`.
- The ClickHouse-backed analytics list (if used) is out of scope.
- `billing` inside the list item's `payment_method_data` remains `None` (unchanged
  from prior behaviour).

## 7. Test plan
- `cargo check` (default v1 features) — must be green.
- Manual: a wallet payment's list entry now carries
  `payment_method_data.wallet.<wallet>.card_network`, matching its retrieve response.
