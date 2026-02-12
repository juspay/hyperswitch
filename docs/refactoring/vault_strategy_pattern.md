# Vault Strategy Pattern Refactoring

## Overview
This document outlines the refactoring plan to consolidate V1 and V2 vault operations using design patterns to reduce code duplication and improve maintainability.

## Current Issues

### Code Duplication
1. **JWE/JWS Creation**: 
   - V1: `mk_vault_req` in `transformers.rs`
   - V2: `create_jwe_body_for_vault` in `transformers.rs`
   - Both functions do the same thing

2. **Vault API Calls**:
   - V1: `call_vault_api` in `transformers.rs` (card-specific)
   - V2: `call_to_vault` in `vault.rs` (generic)

3. **Card-Specific Vault Logic**:
   - V1 has card-specific functions scattered across:
     - `add_card_to_locker` in `cards.rs`
     - `add_card_to_vault` in `cards.rs`
     - `add_generic_payment_method_to_locker` in `cards.rs`

### Architectural Issues
1. No clear separation between card-specific and generic vault operations
2. V1 doesn't leverage the `Vaultable` trait effectively
3. Internal vs External vault logic is mixed

## Proposed Solution

### 1. Consolidate JWE/JWS Creation

**Action**: Remove duplicate `mk_vault_req` and use a single `create_jwe_body` function

```rust
// In transformers.rs - Single unified function
pub async fn create_jwe_body(
    jwekey: &settings::Jwekey,
    jws: &str,
) -> CustomResult<encryption::JweBody, errors::VaultError>
```

### 2. Create VaultStrategy Trait

```rust
// In vault.rs
#[async_trait::async_trait]
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
    
    async fn delete_payment_method(
        &self,
        vault_id: &str,
        customer_id: &id_type::CustomerId,
    ) -> CustomResult<(), errors::VaultError>;
}

pub struct InternalVaultStrategy<'a> {
    state: &'a routes::SessionState,
    merchant_key_store: &'a domain::MerchantKeyStore,
}

pub struct ExternalVaultStrategy<'a> {
    state: &'a routes::SessionState,
    merchant_account: &'a domain::MerchantAccount,
    merchant_connector_account: &'a domain::MerchantConnectorAccount,
}
```

### 3. Create VaultFacade

```rust
// In vault.rs
pub struct VaultFacade<'a> {
    strategy: Box<dyn VaultStrategy + 'a>,
}

impl<'a> VaultFacade<'a> {
    pub fn new_internal(
        state: &'a routes::SessionState,
        merchant_key_store: &'a domain::MerchantKeyStore,
    ) -> Self {
        Self {
            strategy: Box::new(InternalVaultStrategy {
                state,
                merchant_key_store,
            }),
        }
    }
    
    pub fn new_external(
        state: &'a routes::SessionState,
        merchant_account: &'a domain::MerchantAccount,
        merchant_connector_account: &'a domain::MerchantConnectorAccount,
    ) -> Self {
        Self {
            strategy: Box::new(ExternalVaultStrategy {
                state,
                merchant_account,
                merchant_connector_account,
            }),
        }
    }
    
    pub async fn store<T: Vaultable + Send>(
        &self,
        payment_method: &T,
        customer_id: Option<id_type::CustomerId>,
    ) -> CustomResult<VaultResponse, errors::VaultError> {
        self.strategy.store_payment_method(payment_method, customer_id).await
    }
    
    pub async fn retrieve<T: Vaultable>(
        &self,
        vault_id: &str,
        customer_id: &id_type::CustomerId,
    ) -> CustomResult<(T, SupplementaryVaultData), errors::VaultError> {
        self.strategy.retrieve_payment_method(vault_id, customer_id).await
    }
    
    pub async fn delete(
        &self,
        vault_id: &str,
        customer_id: &id_type::CustomerId,
    ) -> CustomResult<(), errors::VaultError> {
        self.strategy.delete_payment_method(vault_id, customer_id).await
    }
}
```

### 4. Refactor V1 to Use Generic Vault Operations

**Current V1 Flow (Card-Specific)**:
```
save_in_locker → save_in_locker_internal → add_card_to_locker → add_card_to_vault → call_vault_api
```

**New V1 Flow (Generic)**:
```
save_in_locker → VaultFacade::store<T: Vaultable>
```

### 5. Update Cards.rs

**Remove**:
- `add_card_to_locker` (use generic vault operations)
- `add_generic_payment_method_to_locker` (merge with card logic)

**Keep**:
- Card-specific validation (expiry validation, BIN lookup)
- Card response construction (`mk_add_card_response_hs`)

**New Structure**:
```rust
// In cards.rs
impl PmCards<'_> {
    pub async fn add_payment_method_to_vault<T: Vaultable + Send>(
        &self,
        req: api::PaymentMethodCreate,
        payment_method_data: &T,
        customer_id: &id_type::CustomerId,
    ) -> errors::CustomResult<
        (api::PaymentMethodResponse, Option<DataDuplicationCheck>),
        errors::VaultError,
    > {
        // Use VaultFacade instead of card-specific logic
        let vault = match &business_profile.external_vault_details {
            ExternalVaultDetails::ExternalVaultEnabled(details) => {
                VaultFacade::new_external(state, merchant_account, mca)
            }
            ExternalVaultDetails::Skip => {
                VaultFacade::new_internal(state, key_store)
            }
        };
        
        let vault_response = vault.store(payment_method_data, Some(customer_id.clone())).await?;
        
        // Card-specific response construction remains here
        self.construct_payment_method_response(vault_response, req)
    }
}
```

## Implementation Steps

### Phase 1: Foundation (Week 1)
- [ ] Consolidate `mk_vault_req` and `create_jwe_body_for_vault` into single `create_jwe_body`
- [ ] Create `VaultStrategy` trait
- [ ] Implement `InternalVaultStrategy`
- [ ] Implement `ExternalVaultStrategy`

### Phase 2: Facade (Week 2)
- [ ] Create `VaultFacade`
- [ ] Add tests for VaultFacade with both strategies
- [ ] Create migration guide for existing code

### Phase 3: V1 Refactoring (Week 3-4)
- [ ] Update `save_in_locker` to use VaultFacade
- [ ] Refactor `add_payment_method_to_locker` to be generic
- [ ] Remove card-specific vault operations from cards.rs
- [ ] Update all V1 vault call sites

### Phase 4: Testing & Documentation (Week 5)
- [ ] Add comprehensive unit tests
- [ ] Add integration tests
- [ ] Update documentation
- [ ] Performance benchmarking

## Benefits

1. **Reduced Duplication**: Single implementation of JWE/JWS creation and vault API calls
2. **Clear Separation**: Card-specific logic isolated to validation and response construction
3. **Extensibility**: Easy to add new payment methods using `Vaultable` trait
4. **Testability**: Each strategy can be tested independently
5. **Maintainability**: Changes to vault logic happen in one place

## Migration Example

**Before (V1)**:
```rust
let (res, dc) = Box::pin(
    PmCards { state, provider }
        .add_card_to_locker(req.clone(), card, &customer_id, None)
)
.await?;
```

**After (V1 with new pattern)**:
```rust
let vault = VaultFacade::new_internal(state, key_store);
let vault_response = vault.store(&domain::PaymentMethodData::Card(card), Some(customer_id)).await?;
let (res, dc) = construct_payment_method_response(vault_response, req)?;
```

## Backwards Compatibility

- All existing V1 APIs remain unchanged
- Internal refactoring only
- Gradual migration path
- Feature flags for rollout control

## Success Metrics

1. Code reduction: Target 30% reduction in vault-related code
2. Test coverage: Maintain >80% coverage
3. Performance: No degradation in latency
4. Zero breaking changes to external APIs