# V1 vs V2 Payment Method Architecture Analysis

## Executive Summary

This document analyzes the differences between V1 and V2 payment method saving flows and proposes architectural improvements to reduce code duplication and improve maintainability.

## Current State Analysis

### V2 Payment Method Saving Flow

```
CreatePaymentMethodCore
  -> CreatePersistentPaymentMethodCore (handles billing, shipping)
    -> CreatePaymentMethodCardCore
      - Validates card expiry
      - Creates payment method for intent
      - Network tokenize and vault the PMD
    -> VaultPaymentMethod (internal/external vault branching)
      -> VaultPaymentMethodInternal
        - get_fingerprint (throws error on duplicate)
        - add_payment_method_to_vault
          - Creates AddVaultRequest
          - Calls connector API (real or mock vault)
        -> create_vault_request
          - Signs payload
          - Creates JWE body
```

### V1 Payment Method Saving Flow

```
SavePaymentMethod
  -> get_payment_method_create_request
    -> Returns api_models::PaymentMethodCreate request

SaveCardAndNetworkTokenInLocker
  -> save_in_locker
    -> save_in_locker_internal
      -> add_payment_method_to_locker (cards.rs)
        -> add_card_to_vault (cards.rs)
          - Calls real or mock locker
        -> call_vault_api (transformers.rs)
          - Signs payload (mk_vault_req)
          - Creates JWE body
```

## Code Duplication Identified

### 1. JWE/JWS Creation Functions

**Location**: `crates/router/src/core/payment_methods/transformers.rs`

| Function | Version | Purpose |
|----------|---------|---------|
| `mk_vault_req` | V1 | Creates JWE body from JWS |
| `create_jwe_body_for_vault` | V2 | Creates JWE body from JWS |

**Issue**: Identical logic duplicated across two functions.

**Resolution**: ✅ **COMPLETED** - Consolidated into single `create_jwe_body` function with wrapper functions for backward compatibility.

### 2. Vault API Calls

**Equivalent Functions**:
- V1: `call_vault_api` in `transformers.rs` (card-specific)
- V2: `call_to_vault` in `vault.rs` (generic)

**Key Difference**: V1's version is card-specific, while V2's is generic using trait-based approach.

### 3. Card-Specific Vault Operations

**V1 has redundant implementations**:
- `add_card_to_locker` (card-specific entry point)
- `add_generic_payment_method_to_locker` (generic but not consistently used)
- `add_card_to_vault` (card-specific vault call)

**V2 uses generic approach**:
- Single `add_payment_method_to_vault` function
- Works with any `Vaultable` type

## Existing Good Architecture: The `Vaultable` Trait

The `Vaultable` trait in `vault.rs` is well-designed and provides:

```rust
pub trait Vaultable: Sized {
    fn get_value1(
        &self,
        customer_id: Option<id_type::CustomerId>,
    ) -> CustomResult<String, errors::VaultError>;
    
    fn get_value2(
        &self,
        _customer_id: Option<id_type::CustomerId>,
    ) -> CustomResult<String, errors::VaultError> {
        Ok(String::new())
    }
    
    fn from_values(
        value1: String,
        value2: String,
    ) -> CustomResult<(Self, SupplementaryVaultData), errors::VaultError>;
}
```

**Implemented for**:
- `domain::Card`
- `domain::BankTransferData`
- `domain::WalletData`
- `domain::BankRedirectData`
- `domain::BankDebitData`
- `domain::PaymentMethodData` (enum wrapper)
- `api::CardPayout`, `api::WalletPayout`, `api::BankPayout`, etc.

## Architectural Improvements Needed

### Priority 1: Make V1 Use Generic Vault Operations (High Impact)

**Current Issue**: V1 has card-specific functions when generic operations already exist.

**Solution**: Refactor `PmCards::add_payment_method_to_locker` to be generic:

```rust
// Current (V1) - Card-specific:
async fn add_payment_method_to_locker(
    &self,
    req: api::PaymentMethodCreate,
    customer_id: &id_type::CustomerId,
) -> errors::CustomResult<
    (api::PaymentMethodResponse, Option<DataDuplicationCheck>),
    errors::VaultError,
>

// Proposed - Generic:
async fn add_payment_method_to_locker<T: Vaultable + Send>(
    &self,
    req: api::PaymentMethodCreate,
    payment_method_data: &T,
    customer_id: &id_type::CustomerId,
) -> errors::CustomResult<
    (api::PaymentMethodResponse, Option<DataDuplicationCheck>),
    errors::VaultError,
>
```

**Benefits**:
- Single implementation for all payment methods
- V1 becomes payment-method-agnostic like V2
- Easier to add new payment methods
- Better testability

### Priority 2: Remove Card-Specific Vault Functions

**Functions to remove/refactor**:
- `add_card_to_locker` → Use generic `add_payment_method_to_locker`
- `add_generic_payment_method_to_locker` → Merge into generic version
- `add_card_to_vault` → Use `Vault::store_payment_method_data_in_locker`

### Priority 3: Consider Strategy Pattern for Vault Selection

**When needed**: If external vault support grows complex.

**Pattern**:
```rust
pub trait VaultStrategy {
    async fn store_payment_method<T: Vaultable + Send>(
        &self,
        payment_method: &T,
        customer_id: Option<id_type::CustomerId>,
    ) -> CustomResult<VaultResponse, errors::VaultError>;
    
    async fn retrieve_payment_method<T: Vaultable>(
        &self,
        vault_id: &str,
        customer_id: &id_type::CustomerId,
    ) -> CustomResult<(T, SupplementaryVaultData), errors::VaultError>;
}

pub struct VaultFacade<'a> {
    strategy: Box<dyn VaultStrategy + 'a>,
}
```

## Implementation Roadmap

### Phase 1: Foundation (Completed ✅)
- [x] Consolidate `mk_vault_req` and `create_jwe_body_for_vault`
- [x] Document findings

### Phase 2: V1 Generic Refactoring (Recommended Next Step)
1. Update `PmCards::add_payment_method_to_locker` to accept `Vaultable` trait
2. Refactor `add_card_to_vault` to use `Vault::store_payment_method_data_in_locker`
3. Remove card-specific wrapper functions
4. Update all call sites in V1

### Phase 3: Testing & Validation
1. Add unit tests for generic vault operations
2. Integration tests for all payment method types
3. Performance benchmarking

## Key Insights

### 1. V2 Architecture is Superior

V2 demonstrates better architectural design:
- Uses trait-based polymorphism (`Vaultable`)
- Generic vault operations
- Clear separation of concerns

### 2. V1 Can Be Improved Without Breaking Changes

V1 can adopt V2 patterns internally:
- Generic implementations behind existing API
- Backward compatible interfaces
- Gradual migration path

### 3. The `Vaultable` Trait is Well-Designed

It already handles:
- Multiple payment method types
- Serialization/deserialization
- Value1/Value2 pattern for sensitive/non-sensitive data

## Code Metrics

### Potential Reductions

| Metric | Before | After Target | Reduction |
|--------|--------|--------------|-----------|
| Duplicate JWE creation | 2 functions | 1 function | 50% |
| Card-specific vault ops | 3 functions | 1 generic | 67% |
| Total vault-related code | ~500 lines | ~350 lines | ~30% |

## Recommendations

### Immediate Actions
1. ✅ Consolidate JWE creation (DONE)
2. Refactor V1 to use generic `Vaultable` trait
3. Remove card-specific vault functions

### Future Considerations
1. Implement VaultFacade if external vault complexity increases
2. Consider Builder pattern for complex payment method creation
3. Evaluate separating concerns further (validation vs vaulting)

## Conclusion

The analysis reveals that:
1. V2 has superior architecture with generic, trait-based design
2. V1 has unnecessary card-specific implementations
3. Significant code reduction (~30%) possible by consolidating
4. The `Vaultable` trait provides excellent foundation for generic operations

The primary improvement is making V1 use the existing generic vault operations, which aligns V1 with V2's superior architecture while maintaining backward compatibility.