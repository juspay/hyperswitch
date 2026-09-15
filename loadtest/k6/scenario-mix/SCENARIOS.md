# Scenarios covered by `scenario-mix.js`

Each `config.json` entry picks one of the following scenarios (`scenario` field) and a
`merchant_path` (`non_modular` or `modular`). The table below is the behavior table
defined in `SCENARIOS` (ported from `loadtest/runner/lib/scenarios.js`).

## `guest`

- No customer is created (`requiresCustomer: false`).
- No card is saved (`setupFutureUsage: null`).
- Modular payment-method sessions, if used, stay volatile (`storageType: "volatile"`).
- Supported on both merchant paths.

## `cit_on_session`

- Customer-initiated save-card flow.
- Customer is created; card is saved with `setup_future_usage: "on_session"`.
- Saved persistently (`storageType: "persistent"`).
- Supported on both merchant paths.

## `cit_off_session`

- Customer-initiated save-card flow.
- Customer is created; card is saved with `setup_future_usage: "off_session"`.
- Saved persistently (`storageType: "persistent"`).
- Supported on both merchant paths.

## `ptv_on_session`

- Payment-to-vault flow: the payment-method session stays volatile, the customer's
  acceptance is recorded, and the card is promoted to persistent storage only after
  authorization succeeds.
- `setup_future_usage: "on_session"`, `storageType: "volatile"`.
- Modular merchant path only (`modularOnly: true`).

## `ptv_off_session`

- Same payment-to-vault behavior as `ptv_on_session`, with
  `setup_future_usage: "off_session"`.
- Modular merchant path only (`modularOnly: true`).

## `cit_metadata_changed`

- Exercises vault metadata replacement: the iteration first saves a card in a baseline
  step (`requiresSavedCard: true`), then the measured confirm resubmits the same PAN
  with changed metadata (expiry/holder name, from `payment.metadata_update`).
- Customer is created; card is saved with `setup_future_usage: "off_session"`,
  persisted (`storageType: "persistent"`).
- Non-modular merchant path only.

## `sdk_checkout`

- Replicates the Hyperswitch SDK's pre-confirm calls: after `payment_create`, the
  iteration lists payment methods (`payment_method_list`), fetches wallet session
  tokens (`session`), and runs a BIN eligibility check (`eligibility`) before the
  measured confirm — mirroring what a real Web/Mobile SDK integration does between
  showing the checkout intent and submitting the card.
- No customer, no saved card (`requiresCustomer: false`, `setupFutureUsage: null`).
- The three new calls authenticate with a base64-encoded SDK Authorization header
  (`profile_id=...,publishable_key=...,client_secret=...,payment_id=...`), the same
  scheme the JS SDK uses, instead of the merchant `api-key`.
- Non-modular merchant path only (`nonModularOnly: true`) — the v2/modular
  payment-method service doesn't expose 1:1 equivalents of these endpoints yet.

## `mit`

- Merchant-initiated transaction: the customer is not present.
- Two-stage iteration, like `cit_metadata_changed`: a baseline step first
  saves a card via a CIT confirm (`setup_future_usage: "off_session"`),
  then the measured request charges the saved payment method.
- The measured request is a single `POST /payments` with `confirm: true`,
  `off_session: true`, and `recurring_details: { type: "payment_method_id",
  data: <saved payment_method_id> }` — no card data, no separate
  `payment_create` call (unlike every other scenario here).
- Customer is created; card is saved persistently in the baseline step.
- Non-modular merchant path only (router's recurring-payments API).

## `saved_card_checkout`

- Two-stage iteration, like `cit_metadata_changed`/`mit`: a baseline step first saves a
  card via a CIT confirm (`setup_future_usage: "off_session"`), then the measured
  request creates a fresh payment intent and calls the **combined**
  payment-method-list endpoint (`GET /payments/{payment_id}/client` — the same
  SDK-facing call `sdk_checkout`'s `payment_method_list` step makes) to obtain the
  saved card's `payment_token` from its `customer_payment_methods`, then confirms
  with that token — no card data on the wire for the measured confirm.
- Mirrors cypress-tests' `14-SaveCardFlow.cy.js` (`listCustomerPMCallTest` +
  `saveCardConfirmCallTest`), adapted to the combined list endpoint (the payment's
  own `customer_id` drives which customer's saved methods come back) rather than
  the merchant-facing `/customers/{customer_id}/payment_methods` endpoint.
- Customer is created; card is saved persistently in the baseline step
  (`requiresSavedCard: true`, `storageType: "persistent"`).
- `setupFutureUsage: null` on the measured payment — it spends a previously saved
  card, it doesn't save a new one.
- Non-modular merchant path only. Requires `merchant.publishable_key`, same as
  `sdk_checkout`, for the SDK Authorization header the combined list endpoint
  authenticates with.

## Merchant path constraints

- `ptv_on_session` and `ptv_off_session` require `merchant_path: "modular"`.
- `cit_metadata_changed`, `sdk_checkout`, `mit`, and `saved_card_checkout` require
  `merchant_path: "non_modular"`.
- `guest`, `cit_on_session`, and `cit_off_session` support both merchant paths.
