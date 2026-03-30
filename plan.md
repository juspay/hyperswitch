# Plan: Remove diesel_models dependency from hyperswitch_domain_models

## Goal
Remove the `diesel_models` dependency from `hyperswitch_domain_models` to allow parallel compilation with `hyperswitch_connectors`.

## Current Dependency Chain
```
hyperswitch_connectors → hyperswitch_domain_models → diesel_models
```

## Target Dependency Chain
```
hyperswitch_connectors → hyperswitch_domain_models
                        ↓
                   common_types ← diesel_models
                        ↓
                   storage_impl
```

## Implementation Phases (v1 only, module by module)

### Phase 1: Move Types to common_types
Move shared types from diesel_models to common_types so both crates can use them without circular dependencies.

**Types to move:**
1. `TaxDetails` (from diesel_models::payment_intent)
2. `FeatureMetadata` (from diesel_models::types)
3. `OrderDetailsWithAmount` (from diesel_models::types)
4. `PaymentLinkConfigRequestForPayments` (from diesel_models)

### Phase 2: Move Conversion Trait to storage_impl
Move the `Conversion` and `ReverseConversion` traits from hyperswitch_domain_models to storage_impl.

### Phase 3: Move Conversion Implementations Module by Module
Move Conversion implementations from hyperswitch_domain_models to storage_impl using ForeignFrom pattern:

**Module order:**
1. customer.rs (simplest, good starting point)
2. merchant_key_store.rs
3. merchant_account.rs
4. business_profile.rs
5. payment_methods.rs
6. merchant_connector_account.rs
7. tokenization.rs
8. relay.rs
9. invoice.rs
10. subscription.rs
11. payment_attempt.rs
12. payment_intent.rs (most complex, last)

### Phase 4: Remove diesel_models Dependency
Update Cargo.toml files and clean up imports.

### Phase 5: Update Router Crate
Update all call sites to use ForeignFrom from storage_impl.

## Testing Strategy
After each module migration, run `just run` to verify compilation.
