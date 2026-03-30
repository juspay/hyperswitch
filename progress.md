# Progress Tracker - Major Refactoring

## Overall Status
**Started:** 2026-03-27
**Strategy:** Move Conversion trait and all implementations from hyperswitch_domain_models to storage_impl

## Completed
- [x] Created Conversion trait in storage_impl/src/behaviour.rs
- [x] Created ReverseConversion trait in storage_impl
- [x] Exported Conversion and ReverseConversion from storage_impl
- [x] Moved customer.rs implementations (v1 and v2) to storage_impl
- [x] Moved merchant_key_store.rs implementation to storage_impl
- [x] Moved merchant_account.rs implementations (v1 and v2) to storage_impl
- [x] Fixed imports in storage_impl files (invoice.rs, payment_method.rs, etc.)
- [x] Moved shared types to common_types (TaxDetails, OrderDetailsWithAmount)
- [x] Removed Conversion implementations from customer.rs in domain_models
- [x] Removed Conversion implementations from merchant_key_store.rs in domain_models
- [x] Removed Conversion implementations from merchant_account.rs in domain_models

## In Progress
- [ ] Moving remaining Conversion implementations to storage_impl:
  - [ ] business_profile.rs (2 implementations)
  - [ ] payment_methods.rs (3 implementations)
  - [ ] merchant_connector_account.rs (2 implementations)
  - [ ] tokenization.rs (1 implementation)
  - [ ] relay.rs (1 implementation)
  - [ ] invoice.rs (2 implementations)
  - [ ] subscription.rs (2 implementations)
  - [ ] authentication.rs (1 implementation)
  - [ ] payments/payment_attempt.rs (2 implementations)
  - [ ] payments/payment_intent.rs (2 implementations)

## Technical Details

### Why This Approach Works
By moving both the `Conversion` trait AND its implementations to storage_impl:
- Trait is local to storage_impl ✅
- Implementations are in the same crate as the trait ✅
- No orphan rule violations ✅

### Remaining Work
Total of ~20 more Conversion implementations need to be moved. Each requires:
1. Copy implementation from domain_models to storage_impl/conversions/
2. Update imports in the conversion file
3. Remove implementation from domain_models
4. Test compilation

### Current Compilation Status
Errors are expected since:
- storage_impl/conversions/ files exist but implementations are incomplete
- domain_models implementations have been removed
- Other crates still reference the old locations

### Next Steps
1. Continue moving remaining Conversion implementations (estimated 20 more)
2. Update router crate imports to use storage_impl::behaviour
3. Remove diesel_models dependency from hyperswitch_domain_models/Cargo.toml
4. Remove behaviour module from hyperswitch_domain_models
5. Test compilation with `just run`

### Estimated Effort
- Each Conversion implementation: 15-30 minutes to copy and fix imports
- Total remaining: ~20 implementations
- Estimated time: 5-10 hours of focused work
