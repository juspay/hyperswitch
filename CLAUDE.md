# Hyperswitch Feature Extraction Project

## What This Is
An exhaustive feature inventory of Hyperswitch categorized into 3 testing buckets with cypress test coverage mapping.

## Script
```bash
python3 scripts/extract_features.py
```
Generates 3 deterministic CSV files (same codebase = same output, byte-for-byte):
- `bucket_1_connector_features.csv` — connector + flow features (163 rows)
- `bucket_2_connector_pm_features.csv` — connector + PM + PMT features (798 rows)
- `bucket_3_core_features.csv` — core features (99 rows)

## Bucket Definitions
- **Bucket 1 (Connector + Flow)**: Behavior differs per connector, PM-agnostic. Test per connector.
- **Bucket 2 (Connector + PM + PMT)**: Behavior differs per connector AND per payment method/type. Test per connector+PM+PMT.
- **Bucket 3 (Core Flow)**: Same behavior regardless of connector. Test once.

Decision tree:
```
Does behavior differ per connector?
├── NO  → Bucket 3
└── YES → Does it additionally differ per PM/PMT?
            ├── NO  → Bucket 1
            └── YES → Bucket 2
```

## Detection Methods (Bucket 1 — 4 layers)
1. **default_implementations.rs macro exclusion** (most reliable): Connectors NOT in `default_imp_for_X!()` macro = real impl. The macros generate no-op trait impls for ~66 flows. Counting `ConnectorIntegration` trait impls is WRONG — all connectors get default no-ops via macros.
2. **connector_enums.rs enum methods**: `is_overcapture_supported_by_connector()` etc.
3. **ConnectorSpecifications trait overrides**: `should_call_connector_customer`, `generate_connector_customer_id`, etc.
4. **Transformer field usage**: `billing_descriptor`, `network_transaction_id`, etc. in transformer files. Must filter out stubs (connectors setting field to `None` with TODO).

## Detection Method (Bucket 2)
Parse `SupportedPaymentMethods` static (LazyLock) in each connector. Each `.add()` declares (PaymentMethod, PaymentMethodType, PaymentMethodDetails{mandates, refunds, supported_capture_methods}).

## Cypress Coverage Check
Parse `cypress-tests/cypress/e2e/configs/Payment/*.js` per-connector configs + `Utils.js` INCLUDE lists.

## Rules
- No compile-time features (`#[cfg(feature)]` excluded)
- No `partially_covered` — cypress status is `covered`, `not_covered`, or `no_cypress_config`
- Bucket 2 only has Supported rows (NotSupported dropped, no status column)
- Scan connector Rust code first (source of truth), then check cypress (coverage check) — NOT the reverse
- Features that work generically (like SDK Client Token Generation) belong in Bucket 3, not Bucket 1

## Key Codebase Locations
- Connector implementations: `crates/hyperswitch_connectors/src/connectors/`
- Default no-op impls: `crates/hyperswitch_connectors/src/default_implementations.rs`
- ConnectorSpecifications trait: `crates/hyperswitch_interfaces/src/api.rs` (~line 431)
- Connector enum methods: `crates/common_enums/src/connector_enums.rs`
- PaymentMethodDetails type: `crates/hyperswitch_domain_models/src/router_response_types.rs` (~line 700)
- Configs DB keys: `crates/common_utils/src/id_type/merchant.rs` (~line 97)
- Superposition keys: `crates/router/src/consts.rs` (~line 355)
- Business profile flags: `crates/diesel_models/src/business_profile.rs`
- MCA table: `crates/diesel_models/src/merchant_connector_account.rs`
- Cypress configs: `cypress-tests/cypress/e2e/configs/Payment/`
