# Analysis Report: `find_merchant_connector_account_without_encrypted_by_merchant_id_and_disabled_list` Call Sites

## Executive Summary

This report analyzes all **16 call sites** of `find_merchant_connector_account_without_encrypted_by_merchant_id_and_disabled_list` across the Hyperswitch codebase. The function fetches all merchant connector accounts (MCAs) for a given `merchant_id` without decryption (no `key_store` required), returning `MerchantConnectorAccountsWithoutEncrypted`.

**Key Findings:**

- **14 of 16 call sites** apply a profile-based filter immediately after the fetch (via `filter_based_on_profile_and_connector_type`, `filter_by_profile`, inline `.filter()`, or `filter_objects_based_on_profile_id_list`). These can be migrated to query by `profile_id` directly at the database level.

- **2 of 16 call sites** do NOT apply any profile filter after the fetch. Both are in payment flow code paths. These are the highest-priority candidates for investigation:
  1. `payments/routing.rs:4094` — `get_active_mca_ids`
  2. `payments/helpers.rs:8977` — `validate_merchant_connector_ids_in_connector_mandate_details`

- There is **no existing "without encrypted" variant for profile_id-based queries**. All existing profile_id-based query functions return the fully decrypted `MerchantConnectorAccount` and require a `key_store`. A new trait method would need to be added to support profile_id-based queries without decryption.

---

## 1. Function Under Analysis

### Trait Definition

**File:** `crates/hyperswitch_domain_models/src/merchant_connector_account.rs`, lines 1367-1376

```rust
/// Like [`Self::find_merchant_connector_account_by_merchant_id_and_disabled_list`],
/// but returns [`MerchantConnectorAccountsWithoutEncrypted`] — only the
/// non-keymanager-encrypted columns — and therefore performs no
/// decryption (no `key_store` needed, zero encryption-service calls).
/// Prefer this whenever the encrypted fields are not read.
async fn find_merchant_connector_account_without_encrypted_by_merchant_id_and_disabled_list(
    &self,
    merchant_id: &id_type::MerchantId,
    get_disabled: bool,
) -> CustomResult<MerchantConnectorAccountsWithoutEncrypted, Self::Error>;
```

**Parameters:**
- `merchant_id: &id_type::MerchantId` — the merchant to filter by
- `get_disabled: bool` — if `true`, returns ALL accounts (including disabled); if `false`, returns only non-disabled accounts

**Returns:** `MerchantConnectorAccountsWithoutEncrypted` — a list wrapper around `Vec<MerchantConnectorAccountWithoutEncrypted>`. Does NOT take a `key_store` — no decryption is performed.

**Underlying Diesel Query:** `storage::MerchantConnectorAccount::find_by_merchant_id` in `crates/diesel_models/src/query/merchant_connector_account.rs` — filters by `merchant_id.eq(...)` (and optionally `disabled.eq(false)`).

---

## 2. Available Profile-Based Alternatives

Three profile_id-based query functions already exist in the trait, but **all return the fully decrypted `MerchantConnectorAccount`** and require a `key_store`:

| Function | Scope | Return Type | Key Store Required | Feature Gate |
|---|---|---|---|---|
| `find_merchant_connector_account_by_profile_id_connector_name` | Single MCA by profile_id + connector_name | `MerchantConnectorAccount` | Yes | `v1` |
| `list_connector_account_by_profile_id` | All MCAs by profile_id | `Vec<MerchantConnectorAccount>` | Yes | `v2 + olap` |
| `list_enabled_connector_accounts_by_profile_id` | Enabled MCAs by profile_id + connector_type | `Vec<MerchantConnectorAccount>` | Yes | (none) |

**Critical Gap:** There is no `find_..._without_encrypted_..._by_profile_id` function. All profile-based queries perform decryption. To migrate the "fetch all then filter" pattern to a DB-level profile query without decryption, a new trait method + Diesel query + storage implementation would need to be added.

The Diesel building blocks already exist:
- `list_by_profile_id` (v2, `crates/diesel_models/src/query/merchant_connector_account.rs:249`) — filters by `profile_id.eq(...)`, returns raw storage rows
- `list_enabled_by_profile_id` (v1 and v2) — filters by `profile_id.eq(...) AND disabled.eq(false) AND connector_type.eq(...)`
- `TryFrom<storage::MerchantConnectorAccount> for MerchantConnectorAccountWithoutEncrypted` — conversion already exists

### In-Memory Filter Helpers

Two helper methods on `MerchantConnectorAccountsWithoutEncrypted` are used as post-fetch filters:

**`filter_based_on_profile_and_connector_type`** (line 987):
```rust
pub fn filter_based_on_profile_and_connector_type(
    self,
    profile_id: &id_type::ProfileId,
    connector_type: common_enums::ConnectorType,
) -> Self {
    self.into_iter()
        .filter(|mca| &mca.profile_id == profile_id && mca.connector_type == connector_type)
        .collect()
}
```

**`filter_by_profile`** (line 954):
```rust
pub fn filter_by_profile<'a, T>(
    &'a self,
    profile_id: &'a id_type::ProfileId,
    func: impl Fn(&'a MerchantConnectorAccount) -> T,
) -> rustc_hash::FxHashSet<T> {
    self.filter_and_map(|mca| mca.profile_id == *profile_id, func)
}
```

**`filter_objects_based_on_profile_id_list`** (`crates/router/src/core/utils.rs:2667`):
```rust
pub(super) fn filter_objects_based_on_profile_id_list<T, U>(
    profile_id_list_auth_layer: Option<Vec<ProfileId>>,
    object_list: U,
) -> U {
    if let Some(profile_id_list) = profile_id_list {
        // filter by profile_id list
    } else {
        object_list  // no filtering when None
    }
}
```

---

## 3. Complete Call Site Inventory

### Category A: Call Sites WITH Immediate Profile Filter (14 sites)

These call sites fetch all MCAs by `merchant_id` and then immediately apply a profile-based filter. They can be migrated to a profile-based DB query.

---

#### A1. `admin.rs:1569` — `PMAuthConfigValidation::validate_pm_auth`

**File:** `crates/router/src/core/admin.rs`, line 1569

**Context:** Validates payment method auth config by checking that referenced MCA IDs belong to the current profile.

**Profile Filter:** YES — iterates MCAs and checks `pm_auth_mca.profile_id != self.profile_id` (line 1585). The `profile_id` is available as `self.profile_id`.

```rust
let all_mcas = self
    .db
    .find_merchant_connector_account_without_encrypted_by_merchant_id_and_disabled_list(
        self.merchant_id,
        true,
    )
    .await?;
for conn_choice in config.enabled_payment_methods {
    let pm_auth_mca = all_mcas.iter().find(|mca| mca.get_id() == conn_choice.mca_id)?;
    if &pm_auth_mca.profile_id != self.profile_id {  // <-- profile filter
        return Err(...);
    }
}
```

**Migration Potential:** HIGH — `profile_id` is available. Could query by profile_id directly.

---

#### A2. `admin.rs:2891` — `validate_pm_auth` (standalone v1 function)

**File:** `crates/router/src/core/admin.rs`, line 2891

**Context:** Identical logic to A1 but as a standalone function. Validates PM auth config.

**Profile Filter:** YES — checks `pm_auth_mca.profile_id != profile_id` (line 2913).

```rust
let all_mcas = state.store
    .find_merchant_connector_account_without_encrypted_by_merchant_id_and_disabled_list(
        merchant_id,
        true,
    )
    .await?;
for conn_choice in config.enabled_payment_methods {
    let pm_auth_mca = all_mcas.iter().find(|mca| mca.get_id() == conn_choice.mca_id)?;
    if &pm_auth_mca.profile_id != profile_id {  // <-- profile filter
        return Err(...);
    }
}
```

**Migration Potential:** HIGH — `profile_id` is available as a parameter.

---

#### A3. `admin.rs:3013` — `list_payment_connectors`

**File:** `crates/router/src/core/admin.rs`, line 3013

**Context:** Admin API to list payment connectors for a merchant. Optionally filtered by profile_id_list.

**Profile Filter:** CONDITIONAL — applies `filter_objects_based_on_profile_id_list(profile_id_list, ...)` (line 3019). When `profile_id_list` is `None`, NO filtering is applied and ALL MCAs across ALL profiles are returned.

```rust
let merchant_connector_accounts = store
    .find_merchant_connector_account_without_encrypted_by_merchant_id_and_disabled_list(
        processor.get_account().get_id(),
        true,
    )
    .await?;
let merchant_connector_accounts = core_utils::filter_objects_based_on_profile_id_list(
    profile_id_list,  // Option<Vec<ProfileId>>
    merchant_connector_accounts,
);
```

**Migration Potential:** MODERATE — When `profile_id_list` is `Some`, could use a profile-based query. When `None`, the current behavior (return all) is likely intentional for the admin listing API. This is an admin/OLAP endpoint, not a payment flow.

---

#### A4. `payment_methods/client.rs:203` — Client-side PM listing

**File:** `crates/router/src/core/payment_methods/client.rs`, line 203

**Context:** Fetches all MCAs to check whether mandate tokens are still active for the current profile.

**Profile Filter:** YES — filters by `mca.profile_id == self.profile_id` (line 218).

```rust
let merchant_connector_accounts = state.store
    .find_merchant_connector_account_without_encrypted_by_merchant_id_and_disabled_list(
        &merchant_id,
        true,
    )
    .await?;
let active_mca_ids: HashSet<_> = merchant_connector_accounts
    .iter()
    .filter(|mca| {
        mca.disabled.is_some_and(|disabled| !disabled)
            && mca.profile_id == self.profile_id  // <-- profile filter
    })
    .map(|mca| mca.get_id())
    .collect();
```

**Migration Potential:** HIGH — `profile_id` is available as `self.profile_id`.

---

#### A5. `payment_methods/cards.rs:3906` — `build_merchant_enabled_pms_context`

**File:** `crates/router/src/core/payment_methods/cards.rs`, line 3906

**Context:** Builds the context of enabled payment methods for a merchant, used in payment method listing for checkout.

**Profile Filter:** YES — applies `filter_based_on_profile_and_connector_type(&profile_id, ConnectorType::PaymentProcessor)` (line 3922).

```rust
let all_mcas = db
    .find_merchant_connector_account_without_encrypted_by_merchant_id_and_disabled_list(
        platform.get_processor().get_account().get_id(),
        false,
    )
    .await?;
let profile_id = business_profile.get_id().clone();
let filtered_mcas = all_mcas
    .clone()
    .filter_based_on_profile_and_connector_type(&profile_id, ConnectorType::PaymentProcessor);
```

**Migration Potential:** HIGH — `profile_id` from `business_profile.get_id()`.

---

#### A6. `payment_methods/cards.rs:5705` — `list_customer_payment_method`

**File:** `crates/router/src/core/payment_methods/cards.rs`, line 5705

**Context:** Lists saved payment methods for a customer. Uses MCAs to check mandate/recurring payment status via `get_mca_status`.

**Profile Filter:** YES — indirectly. The `profile_id` is extracted from `payment_intent` (line 5679-5689), and `get_mca_status` (line 5766) passes `profile_id` to `is_merchant_connector_account_id_in_connector_mandate_details` which filters by `profile_id`.

```rust
let merchant_connector_accounts = state.store
    .find_merchant_connector_account_without_encrypted_by_merchant_id_and_disabled_list(
        platform.get_processor().get_account().get_id(),
        true,
    )
    .await?;
// ... later:
let mca_enabled = get_mca_status(
    profile_id.clone(),       // <-- profile_id passed to get_mca_status
    is_connector_agnostic_mit_enabled,
    Some(connector_mandate_details),
    pm.network_transaction_id.as_ref(),
    &merchant_connector_accounts,
).await?;
```

**Migration Potential:** HIGH — `profile_id` is available from `payment_intent`.

---

#### A7. `routing/helpers.rs:476` — `validate_connectors_in_routing_config`

**File:** `crates/router/src/core/routing/helpers.rs`, line 476

**Context:** Validates that connectors referenced in a routing config belong to the given profile.

**Profile Filter:** YES — filters by `mca.profile_id == *profile_id` (lines 486, 492).

```rust
let all_mcas = state.store
    .find_merchant_connector_account_without_encrypted_by_merchant_id_and_disabled_list(
        merchant_id,
        true,
    )
    .await?;
let name_mca_id_set = all_mcas
    .iter()
    .filter(|mca| mca.profile_id == *profile_id)  // <-- profile filter
    .map(|mca| (&mca.connector_name, mca.get_id()))
    .collect::<FxHashSet<_>>();
let name_set = all_mcas
    .iter()
    .filter(|mca| mca.profile_id == *profile_id)  // <-- profile filter
    .map(|mca| &mca.connector_name)
    .collect::<FxHashSet<_>>();
```

**Migration Potential:** HIGH — `profile_id` is available as a parameter.

---

#### A8. `routing.rs:297` — `create_routing_config`

**File:** `crates/router/src/core/routing.rs`, line 297

**Context:** Creates a new routing configuration. Validates connectors against the profile.

**Profile Filter:** YES — uses `all_mcas.filter_by_profile(business_profile.get_id(), ...)` (lines 307, 313).

```rust
let all_mcas = state.store
    .find_merchant_connector_account_without_encrypted_by_merchant_id_and_disabled_list(
        processor_merchant_id,
        true,
    )
    .await?;
let name_mca_id_set = helpers::ConnectNameAndMCAIdForProfile(
    all_mcas.filter_by_profile(business_profile.get_id(), |mca| {
        (&mca.connector_name, mca.get_id())
    }),
);
let name_set = helpers::ConnectNameForProfile(
    all_mcas.filter_by_profile(business_profile.get_id(), |mca| &mca.connector_name),
);
```

**Migration Potential:** HIGH — `business_profile.get_id()` is available.

---

#### A9. `payout_link.rs:340` — `filter_payout_methods`

**File:** `crates/router/src/core/payout_link.rs`, line 340

**Context:** Filters payout methods for payout link based on profile and connector type.

**Profile Filter:** YES — applies `filter_based_on_profile_and_connector_type(&payout.profile_id, ConnectorType::PayoutProcessor)` (line 347).

```rust
let all_mcas = db
    .find_merchant_connector_account_without_encrypted_by_merchant_id_and_disabled_list(
        platform.get_processor().get_account().get_id(),
        false,
    )
    .await?;
let filtered_mcas = all_mcas.filter_based_on_profile_and_connector_type(
    &payout.profile_id,
    common_enums::ConnectorType::PayoutProcessor,
);
```

**Migration Potential:** HIGH — `payout.profile_id` is available.

---

#### A10. `payments/helpers.rs:6956` — `get_apple_pay_retryable_connectors`

**File:** `crates/router/src/core/payments/helpers.rs`, line 6956

**Context:** Gets Apple Pay retryable connectors for the payment flow. This is a payment flow.

**Profile Filter:** YES — applies `filter_based_on_profile_and_connector_type(profile_id, ConnectorType::PaymentProcessor)` (line 6964).

```rust
let merchant_connector_account_list = state.store
    .find_merchant_connector_account_without_encrypted_by_merchant_id_and_disabled_list(
        processor.get_account().get_id(),
        false,
    )
    .await?;
let profile_specific_merchant_connector_account_list = merchant_connector_account_list
    .filter_based_on_profile_and_connector_type(
        profile_id,
        ConnectorType::PaymentProcessor,
    );
```

**Migration Potential:** HIGH — `profile_id` from `business_profile.get_id()`.

---

#### A11. `payments/helpers.rs:9285` — `validate_allowed_payment_method_types_request`

**File:** `crates/router/src/core/payments/helpers.rs`, line 9285

**Context:** Validates that requested payment method types are supported by the merchant's connectors for the given profile. This is a payment flow.

**Profile Filter:** YES — applies `filter_based_on_profile_and_connector_type(profile_id, ConnectorType::PaymentProcessor)` (line 9294).

```rust
let all_connector_accounts = db
    .find_merchant_connector_account_without_encrypted_by_merchant_id_and_disabled_list(
        processor.get_account().get_id(),
        false,
    )
    .await?;
let filtered_connector_accounts = all_connector_accounts
    .filter_based_on_profile_and_connector_type(
        profile_id,
        ConnectorType::PaymentProcessor,
    );
```

**Migration Potential:** HIGH — `profile_id` is available as a parameter.

---

#### A12. `payments/operations/payment_session_intent.rs:279` — `perform_routing`

**File:** `crates/router/src/core/payments/operations/payment_session_intent.rs`, line 279

**Context:** Payment session intent routing — gets all MCAs and filters by profile to determine which connectors to use for session tokens. This is a core payment flow.

**Profile Filter:** YES — applies `filter_based_on_profile_and_connector_type(profile_id, ConnectorType::PaymentProcessor)` (line 288).

```rust
let all_connector_accounts = db
    .find_merchant_connector_account_without_encrypted_by_merchant_id_and_disabled_list(
        platform.get_processor().get_account().get_id(),
        false,
    )
    .await?;
let profile_id = business_profile.get_id();
let filtered_connector_accounts = all_connector_accounts
    .filter_based_on_profile_and_connector_type(
        profile_id,
        common_enums::ConnectorType::PaymentProcessor,
    );
```

**Migration Potential:** HIGH — `business_profile.get_id()` is available.

---

#### A13. `payments/operations/payment_session.rs:411` — `get_connector`

**File:** `crates/router/src/core/payments/operations/payment_session.rs`, line 411

**Context:** Payment session connector retrieval — gets all MCAs and filters by profile_id from payment_intent to build session tokens. This is a core payment flow.

**Profile Filter:** YES — applies `filter_based_on_profile_and_connector_type(&profile_id, ConnectorType::PaymentProcessor)` (line 427).

```rust
let all_connector_accounts = db
    .find_merchant_connector_account_without_encrypted_by_merchant_id_and_disabled_list(
        processor.get_account().get_id(),
        false,
    )
    .await?;
let profile_id = payment_intent
    .profile_id
    .clone()
    .get_required_value("profile_id")?;
let filtered_connector_accounts = all_connector_accounts
    .filter_based_on_profile_and_connector_type(
        &profile_id,
        common_enums::ConnectorType::PaymentProcessor,
    );
```

**Migration Potential:** HIGH — `profile_id` from `payment_intent.profile_id`.

---

#### A14. `superposition_sdk_config.rs:156` — SDK config retrieval

**File:** `crates/router/src/core/superposition_sdk_config.rs`, line 156

**Context:** Fetches MCA data to build the SDK config response (payment experiences, card networks, banks, etc.).

**Profile Filter:** YES — filters by `mca.profile_id == profile_id_typed` (line 165).

```rust
let all_mcas = db
    .find_merchant_connector_account_without_encrypted_by_merchant_id_and_disabled_list(
        platform.get_processor().get_account().get_id(),
        false,
    )
    .await?;
let filtered_mcas: Vec<_> = all_mcas
    .into_iter()
    .filter(|mca| mca.profile_id == profile_id_typed)  // <-- profile filter
    .collect();
```

**Migration Potential:** HIGH — `profile_id_typed` is available.

---

### Category B: Call Sites WITHOUT Profile Filter (2 sites)

These call sites fetch all MCAs by `merchant_id` and do NOT apply any profile-based filter. These are the highest-priority sites for investigation.

---

#### B1. `payments/routing.rs:4094` — `get_active_mca_ids`

**File:** `crates/router/src/core/payments/routing.rs`, line 4088-4108

**Context:** Returns a `HashSet` of ALL active MCA IDs for a merchant, across ALL profiles. Used by the routing engine to filter which connectors are eligible for routing decisions.

```rust
pub async fn get_active_mca_ids(
    state: &SessionState,
    key_store: &domain::MerchantKeyStore,
) -> RoutingResult<std::collections::HashSet<id_type::MerchantConnectorAccountId>> {
    let db_mcas = state.store
        .find_merchant_connector_account_without_encrypted_by_merchant_id_and_disabled_list(
            &key_store.merchant_id,
            false,
        )
        .await
        .unwrap_or_else(|_| {
            MerchantConnectorAccountsWithoutEncrypted::new(vec![])
        });

    let active_mca_ids: std::collections::HashSet<_> =
        db_mcas.iter().map(|mca| mca.get_id().clone()).collect();
    Ok(active_mca_ids)
}
```

**Profile Filter:** NONE — collects ALL MCA IDs across ALL profiles into a single HashSet.

**Callers (5 sites, all in payment routing):**

| Caller | File | Line | Profile ID Available? |
|---|---|---|---|
| `perform_cgraph_routing` | `payments/routing.rs` | 2341 | YES — `profile_id: &ProfileId` parameter |
| `perform_fallback_routing` | `payments/routing.rs` | 2391 | YES — `business_profile.get_id()` |
| `perform_session_routing` | `payments/routing.rs` | 2543 | YES — `profile_id` in scope |
| `perform_contract_based_session_routing` | `payments/routing.rs` | 2707 | YES — `profile_id` in scope |
| `payments_create_session` | `payments.rs` | 12389 | YES — `business_profile` in scope |

**Analysis:**

Every caller of `get_active_mca_ids` has a `profile_id` available in its scope. The function is used to build a set of "active" MCA IDs that are then passed to the Euclid/constraint-graph routing engine. The routing engine uses this set to filter which connectors are eligible — connectors whose MCA IDs are not in this set are excluded.

**Why profile filtering matters here:**
- A merchant with multiple profiles may have different MCAs configured per profile. An MCA that is active in profile A might not be relevant (or might not even belong) to profile B.
- The current implementation includes MCAs from ALL profiles in the active set, which means the routing engine receives MCA IDs that don't belong to the current profile.
- Downstream, the routing engine also performs its own filtering via `perform_cgraph_filtering` which takes `profile_id` as a parameter, so the cross-profile MCA IDs may be filtered out later. However, this is inefficient — fetching and processing MCAs from unrelated profiles adds unnecessary overhead.
- There is no correctness issue per se (the routing algorithm also receives `profile_id` and uses it for filtering), but there IS a performance issue: the DB fetches all MCAs for the merchant, the HashSet includes all of them, and they are only filtered later.

**Recommendation:** Add an optional `profile_id: Option<&ProfileId>` parameter to `get_active_mca_ids` and filter at the DB level when `Some`. Since all callers have `profile_id` available, this would be a straightforward migration. The `Option` preserves backward compatibility for any future callers that might not have a profile_id.

---

#### B2. `payments/helpers.rs:8977` — `validate_merchant_connector_ids_in_connector_mandate_details`

**File:** `crates/router/src/core/payments/helpers.rs`, lines 8968-9042

**Context:** Validates that merchant connector IDs referenced in `connector_mandate_details` are valid MCAs belonging to the merchant. Used during payment method migration to ensure mandate references are valid.

```rust
pub async fn validate_merchant_connector_ids_in_connector_mandate_details(
    state: &SessionState,
    _key_store: &domain::MerchantKeyStore,
    connector_mandate_details: &api_models::payment_methods::CommonMandateReference,
    merchant_id: &id_type::MerchantId,
    card_network: Option<api_enums::CardNetwork>,
) -> CustomResult<(), errors::ApiErrorResponse> {
    let db = &*state.store;
    let merchant_connector_account_list = db
        .find_merchant_connector_account_without_encrypted_by_merchant_id_and_disabled_list(
            merchant_id,
            true,  // <-- fetches disabled MCAs too
        )
        .await
        .to_not_found_response(errors::ApiErrorResponse::InternalServerError)?;

    let merchant_connector_account_details_hash_map: HashMap<
        id_type::MerchantConnectorAccountId,
        domain::MerchantConnectorAccountWithoutEncrypted,
    > = merchant_connector_account_list
        .iter()
        .map(|mca| (mca.get_id(), mca.clone()))
        .collect();

    // Then validates that each MCA ID in connector_mandate_details exists in the HashMap
    if let Some(payment_mandate_reference) = &connector_mandate_details.payments {
        for (migrating_mca_id, ...) in payment_mandate_reference.0.iter() {
            match (..., merchant_connector_account_details_hash_map.get(migrating_mca_id)) {
                (..., Some(merchant_connector_account_details)) => { /* validate */ }
                (..., None) => Err(InvalidDataValue { field_name: "merchant_connector_id" })?,
            }
        }
    }
}
```

**Profile Filter:** NONE — builds a HashMap of ALL MCAs for the merchant (including disabled ones, since `get_disabled=true`), with no profile scoping.

**Function Signature:** Does NOT accept a `profile_id` parameter. Only takes `merchant_id`.

**Callers (3 sites):**

| Caller | File | Line | Profile ID Available? |
|---|---|---|---|
| `Cards` impl delegate | `payment_methods/cards.rs` | 1371 | NO (not passed through) |
| `migrate_payment_method` (card) | `payment_methods/src/core/migration/payment_methods.rs` | 89 | NO (not available in scope) |
| `migrate_payment_method` (data) | `payment_methods/src/core/migration/payment_methods.rs` | 207 | NO (not available in scope) |

**Analysis:**

This is a **validation function** that checks whether MCA IDs provided in `connector_mandate_details` (during payment method migration) are valid MCAs belonging to the merchant. The function:

1. Fetches ALL MCAs for the merchant (including disabled ones, `get_disabled=true`)
2. Builds a HashMap keyed by MCA ID
3. Iterates over the mandate reference details and checks that each referenced MCA ID exists in the HashMap
4. For Discover card network + cybersource connectors, additionally validates that amount and currency are provided

**Why profile filtering is absent:**
- The function does not receive a `profile_id` parameter — none of its callers pass one.
- The validation is purely "does this MCA ID belong to this merchant?" — it's a merchant-level validation, not a profile-level one.
- Payment method migration is a merchant-scoped operation where the client provides `connector_mandate_details` containing MCA IDs. The mandate references may span multiple profiles if the merchant is migrating payment methods that were set up under different profiles.

**Should this be profile-scoped?**

This is a design decision:
- **If mandates should be profile-scoped** (i.e., a mandate set up under profile A should not be referenced when migrating a payment method that will be used under profile B), then the function should accept and filter by `profile_id`. This would require updating the function signature and all callers to pass `profile_id`.
- **If mandates are merchant-scoped** (i.e., a mandate is valid for the merchant regardless of which profile the payment method is used under), then the current behavior is correct. This seems to be the current design assumption.
- **Note:** The function fetches MCAs with `get_disabled=true` (including disabled accounts), which makes sense for validation — a mandate might have been set up when an MCA was active, and the MCA may have been disabled since. The validation should still confirm the MCA exists, even if disabled.

**Recommendation:** This is likely intentionally merchant-scoped for mandate validation. However, if the product requirement is that mandates should not cross profile boundaries, then `profile_id` should be added as a parameter. This requires a product-level decision.

---

## 4. Summary Table

| # | File | Line | Function | Flow Type | Has Profile Filter? | Profile Source | Migration Potential |
|---|---|---|---|---|---|---|---|
| A1 | admin.rs | 1569 | `PMAuthConfigValidation::validate_pm_auth` | Admin | YES | `self.profile_id` | HIGH |
| A2 | admin.rs | 2891 | `validate_pm_auth` | Admin | YES | `profile_id` param | HIGH |
| A3 | admin.rs | 3013 | `list_payment_connectors` | Admin | CONDITIONAL | `profile_id_list` (Optional) | MODERATE |
| A4 | payment_methods/client.rs | 203 | Client PM listing | PM Flow | YES | `self.profile_id` | HIGH |
| A5 | payment_methods/cards.rs | 3906 | `build_merchant_enabled_pms_context` | PM Flow | YES | `business_profile.get_id()` | HIGH |
| A6 | payment_methods/cards.rs | 5705 | `list_customer_payment_method` | PM Flow | YES (indirect) | `payment_intent.profile_id` | HIGH |
| A7 | routing/helpers.rs | 476 | `validate_connectors_in_routing_config` | Routing | YES | `profile_id` param | HIGH |
| A8 | routing.rs | 297 | `create_routing_config` | Routing | YES | `business_profile.get_id()` | HIGH |
| A9 | payout_link.rs | 340 | `filter_payout_methods` | Payout | YES | `payout.profile_id` | HIGH |
| A10 | payments/helpers.rs | 6956 | `get_apple_pay_retryable_connectors` | Payment | YES | `profile_id` (business_profile) | HIGH |
| A11 | payments/helpers.rs | 9285 | `validate_allowed_payment_method_types_request` | Payment | YES | `profile_id` param | HIGH |
| A12 | payment_session_intent.rs | 279 | `perform_routing` | Payment | YES | `business_profile.get_id()` | HIGH |
| A13 | payment_session.rs | 411 | `get_connector` | Payment | YES | `payment_intent.profile_id` | HIGH |
| A14 | superposition_sdk_config.rs | 156 | SDK config retrieval | SDK Config | YES | `profile_id_typed` | HIGH |
| **B1** | **payments/routing.rs** | **4094** | **`get_active_mca_ids`** | **Payment Routing** | **NO** | **N/A** | **Needs profile_id param** |
| **B2** | **payments/helpers.rs** | **8977** | **`validate_merchant_connector_ids_in_connector_mandate_details`** | **PM Migration** | **NO** | **N/A** | **Needs product decision** |

---

## 5. Detailed Analysis of No-Profile-Filter Sites

### B1: `get_active_mca_ids` — Payment Routing

**The Problem:**
The function fetches ALL MCAs for a merchant across ALL profiles and returns their IDs as a HashSet. This set is passed to the routing engine (Euclid/constraint-graph) which uses it to determine which connectors are eligible for routing.

**Impact:**
- **Performance:** For merchants with many profiles and many MCAs, this fetches significantly more data than needed. The routing engine only cares about MCAs for the current profile.
- **Correctness:** The downstream routing functions (`perform_cgraph_filtering`, etc.) also receive `profile_id` and use it for their own filtering, so cross-profile MCA IDs in the active set are likely filtered out before affecting routing decisions. However, this is redundant work — the filtering should happen at the DB level.
- **Potential Risk:** If any downstream consumer of the `active_mca_ids` set does NOT also filter by profile_id, it could incorrectly consider MCAs from other profiles as "active" and eligible for routing.

**Why it currently works:**
All 5 callers of `get_active_mca_ids` pass the result to `perform_cgraph_filtering` (or similar) along with a `profile_id`. The cgraph filtering likely intersects the active_mca_ids with the profile-scoped connector list, effectively filtering out cross-profile MCAs. So the current behavior is correct but inefficient.

**Migration Path:**
1. Add `profile_id: &ProfileId` parameter to `get_active_mca_ids`
2. Filter MCAs by `profile_id` before collecting into the HashSet
3. Update all 5 callers to pass their available `profile_id`
4. This can be done as an in-memory filter initially (no DB change needed), or ideally as a new DB-level query

### B2: `validate_merchant_connector_ids_in_connector_mandate_details` — PM Migration

**The Problem:**
The function validates that MCA IDs in `connector_mandate_details` are valid MCAs for the merchant. It fetches ALL MCAs (including disabled) with no profile scoping.

**Impact:**
- **Correctness:** A mandate reference to an MCA in a different profile would pass validation. If mandates should be profile-scoped, this is a validation gap.
- **Performance:** Fetches all MCAs including disabled ones, but this is a migration operation (not hot path), so performance is less critical.

**Why it currently works:**
The function's contract is "validate these MCA IDs belong to this merchant" — it's intentionally merchant-scoped. Mandates in Hyperswitch are tied to the merchant, not to a specific profile. When a payment method is migrated with mandate references, the mandates are validated at the merchant level.

**Key Questions for Product Team:**
1. Should mandate references be restricted to the same profile? i.e., if a payment method is being migrated for use under profile A, should mandate references to MCAs in profile B be rejected?
2. If yes, the function needs a `profile_id` parameter and all callers need to provide it.
3. If no (mandates are merchant-scoped), the current behavior is correct and no change is needed.

**Migration Path (if profile-scoping is needed):**
1. Add `profile_id: &ProfileId` parameter to the function
2. Filter the HashMap by `profile_id` after fetching MCAs
3. Update the 3 callers to pass `profile_id` — this requires checking if `profile_id` is available in the migration flow context (it may need to be threaded through from the API layer)

---

## 6. Recommendations

### Short Term (Low Risk)

1. **B1 (`get_active_mca_ids`):** Add `profile_id: &ProfileId` parameter and filter the MCA list by profile before collecting IDs. All 5 callers have `profile_id` available. This is a safe, backward-compatible change (the function is only called internally).

2. **Category A sites (14 sites):** These are already correct (they filter by profile in memory), but they are inefficient. They fetch ALL merchant MCAs from the DB and then discard most of them. Consider migrating them to a profile-based DB query in a future optimization pass. This requires adding a new `find_..._without_encrypted_..._by_profile_id` trait method.

### Medium Term (Requires New Infrastructure)

3. **Add `find_merchant_connector_account_without_encrypted_by_profile_id` to the storage trait.** This would be a new trait method that queries MCAs by `profile_id` (and optionally `connector_type` and `get_disabled`) without decryption. The Diesel building blocks already exist (`list_by_profile_id`, `list_enabled_by_profile_id`). This would enable all Category A sites to skip the in-memory filtering and fetch only the relevant MCAs from the DB.

### Long Term (Requires Product Decision)

4. **B2 (`validate_merchant_connector_ids_in_connector_mandate_details`):** Requires a product decision on whether mandates should be profile-scoped. If yes, add `profile_id` parameter and thread it through the migration flow. If no, document the current behavior as intentional.

---

## 7. Infrastructure Notes

### Existing Profile-Based DB Queries (Diesel Layer)

All in `crates/diesel_models/src/query/merchant_connector_account.rs`:

| Query | Line | Filters | Feature Gate |
|---|---|---|---|
| `find_by_profile_id_connector_name` | 73 | `profile_id + connector_name` | v1 |
| `list_enabled_by_profile_id` | 156 | `profile_id + disabled=false + connector_type` | v1 |
| `list_by_profile_id` | 249 | `profile_id` | v2 |
| `list_enabled_by_profile_id` | 263 | `profile_id + disabled=false + connector_type` | v2 |

### Missing: Without-Encrypted Profile Query

To add a without-encrypted variant for profile-based queries:
1. Add trait method to `MerchantConnectorAccountInterface` in `crates/hyperswitch_domain_models/src/merchant_connector_account.rs`
2. Implement in `KVRouterStore` in `crates/storage_impl/src/merchant_connector_account.rs` — reuse the existing `list_by_profile_id` or `list_enabled_by_profile_id` Diesel query and convert via `MerchantConnectorAccountWithoutEncrypted::try_from`
3. Implement in `MockDb` in `crates/storage_impl/src/merchant_connector_account.rs`
4. Implement in `KafkaStore` in `crates/router/src/db/kafka_store.rs` (delegate to underlying store)
