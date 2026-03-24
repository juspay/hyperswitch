# Technical Specification: Profile-Level Configurable Retry Rules

**Document Version:** 1.0  
**Date:** 2026-03-16  
**Status:** Draft

---

## 1. Executive Summary

### Current State
HyperSwitch uses a **Gateway Status Map (GSM)** system to determine retry behavior. GSM rules are stored in a database table and map PSP error codes to retry decisions (`Retry` or `DoDefault`). However, these rules are **global** - all merchants use the same mappings maintained by HyperSwitch.

**Key Files:**
- `crates/router/src/core/payments/retry.rs` - Retry orchestration
- `crates/router/src/core/payments/helpers.rs` - GSM lookup (`get_gsm_record`)
- `crates/hyperswitch_domain_models/src/gsm.rs` - GSM domain model
- `crates/diesel_models/src/gsm.rs` - GSM database model

### Problem Statement
As adoption grows with more merchants using multiple PSPs:
- Merchants have different risk tolerances
- Some merchants want to retry on specific errors; others prefer not to
- Current global GSM rules don't accommodate merchant-specific preferences
- No way for merchants to customize retry behavior per-error-code

### Proposed Solution
Introduce **Profile-Level Configurable Retry Rules** that:
1. Extend the existing GSM system with profile-specific overrides
2. Allow merchants to define custom retry rules per error code
3. Support granular control over retry behavior per profile
4. Maintain backward compatibility with existing global GSM rules

---

## 2. Architecture Overview

### 2.1 Current Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Payment Flow                              │
├─────────────────────────────────────────────────────────────┤
│  Payment Fails → Extract Error Code → GSM Lookup            │
│                                              ↓               │
│                              ┌─────────────────────┐        │
│                              │   GSM Table         │        │
│                              │   (Global Rules)    │        │
│                              │   - connector       │        │
│                              │   - error_code      │        │
│                              │   - decision        │        │
│                              │   - feature_data    │        │
│                              └─────────────────────┘        │
│                                              ↓               │
│                              GsmDecision::Retry or DoDefault│
└─────────────────────────────────────────────────────────────┘
```

### 2.2 Proposed Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                    Enhanced Payment Flow                             │
├─────────────────────────────────────────────────────────────────────┤
│  Payment Fails → Extract Error Code → Retry Rule Lookup             │
│                                              ↓                       │
│                    ┌───────────────────────────────────────┐        │
│                    │   1. Profile GSM Override Lookup      │        │
│                    │      (profile_id + connector + code)  │        │
│                    └───────────────────────────────────────┘        │
│                              ↓ Not Found                             │
│                    ┌───────────────────────────────────────┐        │
│                    │   2. Global GSM Lookup (existing)     │        │
│                    └───────────────────────────────────────┘        │
│                              ↓                                       │
│                    ┌───────────────────────────────────────┐        │
│                    │   3. Apply RetryDecision              │        │
│                    │      - Check profile settings         │        │
│                    │      - Honor max_retries limit        │        │
│                    │      - Execute retry strategy         │        │
│                    └───────────────────────────────────────┘        │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 3. Data Model Changes

### 3.1 New Table: `profile_gsm_override`

Create a new table for profile-specific retry rules:

```sql
-- Migration: Add profile_gsm_override table
CREATE TABLE profile_gsm_override (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    profile_id VARCHAR(64) NOT NULL,
    connector VARCHAR(64) NOT NULL,
    flow VARCHAR(64) NOT NULL DEFAULT 'payments',
    sub_flow VARCHAR(64) NOT NULL DEFAULT 'authorize',
    error_code VARCHAR(255) NOT NULL,
    error_message_pattern VARCHAR(255),           -- Optional regex pattern
    decision VARCHAR(32) NOT NULL DEFAULT 'do_default',  -- 'retry' or 'do_default'
    max_retries SMALLINT,                         -- Override profile default
    retry_strategy JSONB,                         -- Custom retry strategy
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    modified_at TIMESTAMP NOT NULL DEFAULT NOW(),
    created_by VARCHAR(64),
    
    CONSTRAINT fk_profile FOREIGN KEY (profile_id) 
        REFERENCES business_profile(profile_id) ON DELETE CASCADE,
    CONSTRAINT unique_profile_error_mapping 
        UNIQUE (profile_id, connector, flow, error_code)
);

CREATE INDEX idx_profile_gsm_profile_id ON profile_gsm_override(profile_id);
CREATE INDEX idx_profile_gsm_connector_code ON profile_gsm_override(connector, error_code);
```

### 3.2 Rust Data Models

**File:** `crates/diesel_models/src/profile_gsm_override.rs`

```rust
use diesel::{AsChangeset, Insertable, Queryable};
use time::PrimitiveDateTime;

#[derive(Debug, Clone, Queryable, Insertable)]
#[diesel(table_name = crate::schema::profile_gsm_override)]
pub struct ProfileGsmOverride {
    pub id: uuid::Uuid,
    pub profile_id: String,
    pub connector: String,
    pub flow: String,
    pub sub_flow: String,
    pub error_code: String,
    pub error_message_pattern: Option<String>,
    pub decision: String,
    pub max_retries: Option<i16>,
    pub retry_strategy: Option<serde_json::Value>,
    pub created_at: PrimitiveDateTime,
    pub modified_at: PrimitiveDateTime,
    pub created_by: String,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = crate::schema::profile_gsm_override)]
pub struct ProfileGsmOverrideNew {
    pub profile_id: String,
    pub connector: String,
    pub flow: String,
    pub sub_flow: String,
    pub error_code: String,
    pub error_message_pattern: Option<String>,
    pub decision: String,
    pub max_retries: Option<i16>,
    pub retry_strategy: Option<serde_json::Value>,
    pub created_by: String,
}

#[derive(Debug, Clone, AsChangeset)]
#[diesel(table_name = crate::schema::profile_gsm_override)]
pub struct ProfileGsmOverrideUpdate {
    pub decision: Option<String>,
    pub max_retries: Option<i16>,
    pub retry_strategy: Option<serde_json::Value>,
    pub modified_at: PrimitiveDateTime,
}
```

### 3.3 Retry Strategy Schema

```rust
// crates/common_types/src/retry.rs
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Retry strategy configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RetryStrategy {
    /// Type of retry strategy
    pub strategy_type: RetryStrategyType,
    /// Delay in seconds before first retry
    pub initial_delay_seconds: u32,
    /// Multiplier for subsequent retries (exponential backoff)
    pub backoff_multiplier: Option<f32>,
    /// Maximum delay between retries in seconds
    pub max_delay_seconds: Option<u32>,
    /// Enable step-up (3DS) retry
    pub step_up_enabled: bool,
    /// Enable clear PAN retry
    pub clear_pan_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RetryStrategyType {
    /// Retry with same connector
    SameConnector,
    /// Retry with next available connector (routing)
    NextConnector,
    /// Retry with specific connector
    SpecificConnector(String),
    /// Step-up authentication then retry
    StepUp,
}
```

### 3.4 Profile Table Extension

```sql
-- Add retry_rules_enabled flag to business_profile
ALTER TABLE business_profile 
ADD COLUMN retry_rules_enabled BOOLEAN DEFAULT FALSE;
```

---

## 4. API Design

### 4.1 Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/profiles/{profile_id}/retry-rules` | Create a retry rule |
| `GET` | `/profiles/{profile_id}/retry-rules` | List all retry rules |
| `GET` | `/profiles/{profile_id}/retry-rules/{rule_id}` | Get specific rule |
| `PUT` | `/profiles/{profile_id}/retry-rules/{rule_id}` | Update a retry rule |
| `DELETE` | `/profiles/{profile_id}/retry-rules/{rule_id}` | Delete a retry rule |
| `POST` | `/profiles/{profile_id}/retry-rules/bulk-import` | Bulk import rules |
| `GET` | `/profiles/{profile_id}/retry-rules/export` | Export rules |

### 4.2 Request/Response Models

**File:** `crates/api_models/src/retry_rules.rs`

```rust
use common_enums::GsmDecision;
use common_types::retry::RetryStrategy;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Request to create a retry rule
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RetryRuleCreateRequest {
    /// Connector name (e.g., "stripe", "adyen")
    pub connector: String,
    
    /// Flow type (default: "payments")
    #[serde(default = "default_flow")]
    pub flow: String,
    
    /// Sub-flow type (default: "authorize")
    #[serde(default = "default_sub_flow")]
    pub sub_flow: String,
    
    /// Error code to match
    pub error_code: String,
    
    /// Optional regex pattern for error message matching
    pub error_message_pattern: Option<String>,
    
    /// Decision: "retry" or "do_default"
    pub decision: GsmDecision,
    
    /// Override max retries for this rule
    #[schema(value_type = Option<u8>, example = 2)]
    pub max_retries: Option<i16>,
    
    /// Custom retry strategy
    pub retry_strategy: Option<RetryStrategy>,
}

fn default_flow() -> String { "payments".to_string() }
fn default_sub_flow() -> String { "authorize".to_string() }

/// Request to update a retry rule
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RetryRuleUpdateRequest {
    pub decision: Option<GsmDecision>,
    #[schema(value_type = Option<u8>)]
    pub max_retries: Option<i16>,
    pub retry_strategy: Option<RetryStrategy>,
}

/// Response for a retry rule
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RetryRuleResponse {
    pub id: String,
    pub profile_id: String,
    pub connector: String,
    pub flow: String,
    pub sub_flow: String,
    pub error_code: String,
    pub error_message_pattern: Option<String>,
    pub decision: GsmDecision,
    #[schema(value_type = Option<u8>)]
    pub max_retries: Option<i16>,
    pub retry_strategy: Option<RetryStrategy>,
    pub created_at: String,
    pub modified_at: String,
}

/// List of retry rules
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RetryRulesListResponse {
    pub count: u64,
    pub data: Vec<RetryRuleResponse>,
}

/// Bulk import request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RetryRulesBulkImportRequest {
    pub rules: Vec<RetryRuleCreateRequest>,
    /// Strategy for handling conflicts: "skip", "overwrite", "fail"
    #[serde(default = "default_conflict_strategy")]
    pub on_conflict: String,
}

fn default_conflict_strategy() -> String { "skip".to_string() }

/// Bulk import response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RetryRulesBulkImportResponse {
    pub imported_count: u64,
    pub skipped_count: u64,
    pub errors: Vec<BulkImportError>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BulkImportError {
    pub row_index: usize,
    pub error: String,
}
```

### 4.3 Example API Calls

#### Create Retry Rule
```http
POST /profiles/prof_abc123/retry-rules
Content-Type: application/json

{
    "connector": "stripe",
    "error_code": "card_declined",
    "decision": "retry",
    "max_retries": 2,
    "retry_strategy": {
        "strategy_type": "next_connector",
        "initial_delay_seconds": 5,
        "backoff_multiplier": 2.0,
        "step_up_enabled": false,
        "clear_pan_enabled": false
    }
}
```

**Response:**
```json
{
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "profile_id": "prof_abc123",
    "connector": "stripe",
    "flow": "payments",
    "sub_flow": "authorize",
    "error_code": "card_declined",
    "decision": "retry",
    "max_retries": 2,
    "retry_strategy": {
        "strategy_type": "next_connector",
        "initial_delay_seconds": 5,
        "backoff_multiplier": 2.0,
        "step_up_enabled": false,
        "clear_pan_enabled": false
    },
    "created_at": "2026-03-16T10:30:00Z",
    "modified_at": "2026-03-16T10:30:00Z"
}
```

#### List Retry Rules
```http
GET /profiles/prof_abc123/retry-rules?connector=stripe

Response:
{
    "count": 3,
    "data": [
        {
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "connector": "stripe",
            "error_code": "card_declined",
            "decision": "retry",
            ...
        },
        ...
    ]
}
```

---

## 5. Implementation Details

### 5.1 Database Interface

**File:** `crates/router/src/db/profile_gsm_override.rs`

```rust
use async_trait::async_trait;
use common_utils::errors::CustomResult;
use diesel_models::profile_gsm_override::{
    ProfileGsmOverride, ProfileGsmOverrideNew, ProfileGsmOverrideUpdate,
};
use error_stack::ResultExt;
use router_env::logger;

use crate::{errors::StorageError, StorageInterface};

#[async_trait::async_trait]
pub trait ProfileGsmOverrideInterface {
    async fn find_profile_gsm_override(
        &self,
        profile_id: &str,
        connector: &str,
        flow: &str,
        error_code: &str,
    ) -> CustomResult<Option<ProfileGsmOverride>, StorageError>;

    async fn insert_profile_gsm_override(
        &self,
        rule: ProfileGsmOverrideNew,
    ) -> CustomResult<ProfileGsmOverride, StorageError>;

    async fn update_profile_gsm_override(
        &self,
        id: &uuid::Uuid,
        update: ProfileGsmOverrideUpdate,
    ) -> CustomResult<ProfileGsmOverride, StorageError>;

    async fn delete_profile_gsm_override(
        &self,
        id: &uuid::Uuid,
    ) -> CustomResult<bool, StorageError>;

    async fn list_profile_gsm_overrides(
        &self,
        profile_id: &str,
        connector: Option<&str>,
        limit: Option<u64>,
        offset: Option<u64>,
    ) -> CustomResult<Vec<ProfileGsmOverride>, StorageError>;

    async fn count_profile_gsm_overrides(
        &self,
        profile_id: &str,
    ) -> CustomResult<u64, StorageError>;
}

#[async_trait::async_trait]
impl ProfileGsmOverrideInterface for StorageInterface {
    async fn find_profile_gsm_override(
        &self,
        profile_id: &str,
        connector: &str,
        flow: &str,
        error_code: &str,
    ) -> CustomResult<Option<ProfileGsmOverride>, StorageError> {
        // Implementation using diesel queries
        todo!()
    }

    // ... other implementations
}
```

### 5.2 Modified GSM Lookup

**File:** `crates/router/src/core/payments/helpers.rs`

Add new function after existing `get_gsm_record`:

```rust
/// Enhanced GSM lookup with profile override support
/// 
/// Priority order:
/// 1. Profile-specific override (if retry_rules_enabled)
/// 2. Global GSM lookup (existing behavior)
#[instrument(skip_all)]
pub async fn get_gsm_record_with_profile_override(
    state: &app::SessionState,
    connector: String,
    flow: &str,
    subflow: &str,
    error_code: Option<String>,
    error_message: Option<String>,
    issuer_error_code: Option<String>,
    card_network: Option<common_enums::CardNetwork>,
    profile: &domain::Profile,
) -> RouterResult<Option<hyperswitch_domain_models::gsm::GatewayStatusMap>> {
    let db = &*state.store;

    // Step 1: Check if profile has retry rules enabled
    if !profile.retry_rules_enabled.unwrap_or(false) {
        // Skip override lookup, use global GSM directly
        return get_gsm_record(
            state,
            connector,
            flow,
            subflow,
            error_code.clone(),
            error_message,
            issuer_error_code,
            card_network,
        )
        .await;
    }

    // Step 2: Check for profile-specific override
    if let (Some(code), Some(profile_id)) = (&error_code, &profile.get_id()) {
        let override_result = db
            .find_profile_gsm_override(
                profile_id.as_str(),
                &connector,
                flow,
                code,
            )
            .await
            .map_err(|e| {
                logger::error!(error=?e, "Failed to lookup profile GSM override");
                errors::ApiErrorResponse::InternalServerError
            })?;

        if let Some(override_rule) = override_result {
            logger::info!(
                profile_id = %profile_id,
                connector = %connector,
                error_code = %code,
                rule_id = %override_rule.id,
                "Using profile GSM override"
            );
            metrics::PROFILE_GSM_OVERRIDE_MATCH_COUNT.add(1, &[]);
            return Ok(Some(map_override_to_gsm(override_rule)));
        }
    }

    // Step 3: Fall back to global GSM (existing logic)
    metrics::PROFILE_GSM_OVERRIDE_FALLBACK_COUNT.add(1, &[]);
    get_gsm_record(
        state,
        connector,
        flow,
        subflow,
        error_code,
        error_message,
        issuer_error_code,
        card_network,
    )
    .await
}

/// Map profile override to GSM structure for consistent handling
fn map_override_to_gsm(
    override_rule: diesel_models::profile_gsm_override::ProfileGsmOverride,
) -> hyperswitch_domain_models::gsm::GatewayStatusMap {
    use common_types::domain::{GsmFeatureData, RetryFeatureData};
    use common_enums::GsmDecision;

    let decision: GsmDecision = override_rule
        .decision
        .parse()
        .unwrap_or(GsmDecision::DoDefault);

    let retry_feature_data = RetryFeatureData {
        step_up_possible: override_rule
            .retry_strategy
            .as_ref()
            .and_then(|s| s.get("step_up_enabled").and_then(|v| v.as_bool()))
            .unwrap_or(false),
        clear_pan_possible: override_rule
            .retry_strategy
            .as_ref()
            .and_then(|s| s.get("clear_pan_enabled").and_then(|v| v.as_bool()))
            .unwrap_or(false),
        alternate_network_possible: false,
        decision: decision.clone(),
    };

    hyperswitch_domain_models::gsm::GatewayStatusMap {
        connector: override_rule.connector,
        flow: override_rule.flow,
        sub_flow: override_rule.sub_flow,
        code: override_rule.error_code,
        message: String::new(),
        status: String::new(),
        router_error: None,
        unified_code: None,
        unified_message: None,
        error_category: None,
        feature_data: GsmFeatureData::Retry(retry_feature_data),
        feature: common_enums::GsmFeature::Retry,
        standardised_code: None,
        description: None,
        user_guidance_message: None,
    }
}
```

### 5.3 Updated Retry Logic

**File:** `crates/router/src/core/payments/retry.rs`

Modify the `get_gsm` function to accept profile and use override lookup:

```rust
#[instrument(skip_all)]
pub async fn get_gsm<F, FData>(
    state: &app::SessionState,
    router_data: &types::RouterData<F, FData, types::PaymentsResponseData>,
    card_network: Option<common_enums::CardNetwork>,
    profile: &domain::Profile,  // Add profile parameter
) -> RouterResult<Option<hyperswitch_domain_models::gsm::GatewayStatusMap>> {
    let error_response = router_data.response.as_ref().err();
    let subflow = get_flow_name::<F>()?;
    let error_code = error_response.map(|err| err.code.to_owned());
    let err_message = error_response.map(|err| err.message.to_owned());
    let issuer_error_code = error_response.and_then(|err| err.network_decline_code.clone());
    let connector_str = router_data.connector.to_string();

    // Use enhanced lookup with profile override support
    Ok(payments::helpers::get_gsm_record_with_profile_override(
        state,
        connector_str,
        consts::PAYMENT_FLOW_STR,
        &subflow,
        error_code,
        err_message,
        issuer_error_code,
        card_network,
        profile,
    )
    .await)
}

/// Enhanced get_retries to consider rule-specific max_retries
#[instrument(skip_all)]
pub async fn get_retries_with_override(
    state: &app::SessionState,
    retries: Option<i32>,
    merchant_id: &common_utils::id_type::MerchantId,
    profile: &domain::Profile,
    gsm_override_max_retries: Option<i16>,
) -> Option<i32> {
    match retries {
        Some(retries) => Some(retries),
        None => {
            // Priority: Rule-specific max_retries > Merchant config > Profile config
            if let Some(max) = gsm_override_max_retries {
                return Some(max as i32);
            }
            get_merchant_max_auto_retries_enabled(state.store.as_ref(), merchant_id)
                .await
                .or(profile.max_auto_retries_enabled.map(i32::from))
        }
    }
}
```

### 5.4 API Handler

**File:** `crates/router/src/core/retry_rules.rs`

```rust
use api_models::retry_rules::*;
use common_utils::ext_traits::ValueExt;
use error_stack::ResultExt;
use router_env::logger;

use crate::{
    core::errors::{ApiErrorResponse, RouterResult},
    db::profile_gsm_override::ProfileGsmOverrideInterface,
    routes::app::SessionState,
    types::transformers::ForeignFrom,
};

pub async fn create_retry_rule(
    state: &SessionState,
    profile_id: String,
    request: RetryRuleCreateRequest,
    merchant_id: String,
) -> RouterResult<RetryRuleResponse> {
    let db = &*state.store;

    // Validate max_retries limit per profile
    let existing_count = db
        .count_profile_gsm_overrides(&profile_id)
        .await
        .change_context(ApiErrorResponse::InternalServerError)?;

    if existing_count >= 100 {
        return Err(ApiErrorResponse::InvalidRequest {
            message: "Maximum retry rules limit (100) reached for this profile".to_string(),
        }
        .into());
    }

    let rule_new = diesel_models::profile_gsm_override::ProfileGsmOverrideNew {
        profile_id: profile_id.clone(),
        connector: request.connector.clone(),
        flow: request.flow,
        sub_flow: request.sub_flow,
        error_code: request.error_code.clone(),
        error_message_pattern: request.error_message_pattern,
        decision: request.decision.to_string(),
        max_retries: request.max_retries,
        retry_strategy: request.retry_strategy.map(|s| s.into()),
        created_by: merchant_id,
    };

    let rule = db
        .insert_profile_gsm_override(rule_new)
        .await
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to insert retry rule")?;

    Ok(ForeignFrom::foreign_from(rule))
}

pub async fn list_retry_rules(
    state: &SessionState,
    profile_id: String,
    connector: Option<String>,
    limit: Option<u64>,
    offset: Option<u64>,
) -> RouterResult<RetryRulesListResponse> {
    let db = &*state.store;

    let rules = db
        .list_profile_gsm_overrides(&profile_id, connector.as_deref(), limit, offset)
        .await
        .change_context(ApiErrorResponse::InternalServerError)?;

    let count = rules.len() as u64;

    Ok(RetryRulesListResponse {
        count,
        data: rules.into_iter().map(ForeignFrom::foreign_from).collect(),
    })
}

pub async fn update_retry_rule(
    state: &SessionState,
    rule_id: uuid::Uuid,
    request: RetryRuleUpdateRequest,
) -> RouterResult<RetryRuleResponse> {
    let db = &*state.store;

    let update = diesel_models::profile_gsm_override::ProfileGsmOverrideUpdate {
        decision: request.decision.map(|d| d.to_string()),
        max_retries: request.max_retries,
        retry_strategy: request.retry_strategy.map(|s| s.into()),
        modified_at: common_utils::date_time::now(),
    };

    let rule = db
        .update_profile_gsm_override(&rule_id, update)
        .await
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to update retry rule")?;

    Ok(ForeignFrom::foreign_from(rule))
}

pub async fn delete_retry_rule(
    state: &SessionState,
    rule_id: uuid::Uuid,
) -> RouterResult<()> {
    let db = &*state.store;

    db.delete_profile_gsm_override(&rule_id)
        .await
        .change_context(ApiErrorResponse::InternalServerError)
        .attach_printable("Failed to delete retry rule")?;

    Ok(())
}
```

---

## 6. Files to Create/Modify

### New Files

| File | Purpose |
|------|---------|
| `crates/diesel_models/src/profile_gsm_override.rs` | Database model |
| `crates/api_models/src/retry_rules.rs` | API request/response types |
| `crates/common_types/src/retry.rs` | Retry strategy types |
| `crates/router/src/db/profile_gsm_override.rs` | Database interface |
| `crates/router/src/core/retry_rules.rs` | API handlers |
| `migrations/YYYY-MM-DD-HHMMSS_add_profile_gsm_override/up.sql` | Migration |
| `migrations/YYYY-MM-DD-HHMMSS_add_profile_gsm_override/down.sql` | Rollback |

### Modified Files

| File | Changes |
|------|---------|
| `crates/router/src/core/payments/helpers.rs` | Add `get_gsm_record_with_profile_override` |
| `crates/router/src/core/payments/retry.rs` | Use profile-aware GSM lookup |
| `crates/diesel_models/src/schema.rs` | Add `profile_gsm_override` table |
| `crates/diesel_models/src/lib.rs` | Export new module |
| `crates/diesel_models/src/business_profile.rs` | Add `retry_rules_enabled` field |
| `crates/api_models/src/admin.rs` | Add `retry_rules_enabled` to ProfileCreate/Update/Response |
| `crates/api_models/src/lib.rs` | Export new module |
| `crates/router/src/routes/app.rs` | Add new API routes |
| `crates/router/src/db/mod.rs` | Export new interface |
| `crates/router/src/consts.rs` | Add constants for limits |
| `crates/router/src/metrics.rs` | Add new metrics |

---

## 7. Backward Compatibility

### 7.1 Default Behavior
- Profiles without custom retry rules continue using global GSM
- Existing `is_auto_retries_enabled` and `max_auto_retries_enabled` remain functional
- No breaking changes to existing APIs
- Feature disabled by default (`retry_rules_enabled = false`)

### 7.2 Migration Path
1. Deploy database migration (additive, no data loss)
2. Deploy application changes (feature behind flag)
3. Enable feature per-profile via API
4. Merchants configure custom rules

### 7.3 Rollback
```sql
-- Down migration
DROP TABLE IF EXISTS profile_gsm_override;
ALTER TABLE business_profile DROP COLUMN IF EXISTS retry_rules_enabled;
```

---

## 8. Security Considerations

### 8.1 Access Control
- Only merchant admins can create/update retry rules
- Validate that user has write access to the profile
- Log all rule changes for audit trail

### 8.2 Input Validation
```rust
pub fn validate_retry_rule_request(request: &RetryRuleCreateRequest) -> RouterResult<()> {
    // Validate connector name
    if request.connector.len() > 64 {
        return Err(ApiErrorResponse::InvalidRequest {
            message: "Connector name too long".to_string(),
        }
        .into());
    }

    // Validate error code
    if request.error_code.is_empty() || request.error_code.len() > 255 {
        return Err(ApiErrorResponse::InvalidRequest {
            message: "Error code must be 1-255 characters".to_string(),
        }
        .into());
    }

    // Validate max_retries
    if let Some(max) = request.max_retries {
        if max < 0 || max > 10 {
            return Err(ApiErrorResponse::InvalidRequest {
                message: "max_retries must be between 0 and 10".to_string(),
            }
            .into());
        }
    }

    // Validate regex pattern if provided
    if let Some(pattern) = &request.error_message_pattern {
        if let Err(e) = regex::Regex::new(pattern) {
            return Err(ApiErrorResponse::InvalidRequest {
                message: format!("Invalid regex pattern: {}", e),
            }
            .into());
        }
    }

    Ok(())
}
```

### 8.3 Rate Limiting
- Maximum 100 rules per profile
- Bulk import limited to 50 rules per request
- Standard API rate limits apply

---

## 9. Monitoring & Observability

### 9.1 Metrics

```rust
// In crates/router/src/metrics.rs

// Counter for profile override lookups
pub static PROFILE_GSM_OVERRIDE_LOOKUP_COUNT: Lazy<Counter> = Lazy::new(|| {
    register_counter!(opts!("profile_gsm_override_lookup_count", "Total profile GSM override lookups")).unwrap()
});

// Counter for matches found
pub static PROFILE_GSM_OVERRIDE_MATCH_COUNT: Lazy<Counter> = Lazy::new(|| {
    register_counter!(opts!("profile_gsm_override_match_count", "Profile GSM override matches")).unwrap()
});

// Counter for fallback to global GSM
pub static PROFILE_GSM_OVERRIDE_FALLBACK_COUNT: Lazy<Counter> = Lazy::new(|| {
    register_counter!(opts!("profile_gsm_override_fallback_count", "Fallback to global GSM")).unwrap()
});

// Histogram for lookup latency
pub static PROFILE_GSM_OVERRIDE_LOOKUP_LATENCY: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(histogram_opts!("profile_gsm_override_lookup_latency_seconds", "Profile GSM override lookup latency")).unwrap()
});
```

### 9.2 Logging

```rust
// Log when profile override is applied
logger::info!(
    profile_id = %profile_id,
    connector = %connector,
    error_code = %error_code,
    rule_id = %override_rule.id,
    decision = %override_rule.decision,
    "Applied profile GSM override"
);

// Include in payment attempt metadata
payment_attempt_metadata.insert("gsm_rule_source", "profile_override");
payment_attempt_metadata.insert("gsm_rule_id", rule_id.to_string());
```

---

## 10. Testing Strategy

### 10.1 Unit Tests

| Test | Description |
|------|-------------|
| `test_profile_override_takes_precedence` | Profile rule overrides global GSM |
| `test_fallback_to_global_gsm` | Falls back when no override exists |
| `test_feature_disabled_skips_lookup` | Skips lookup when flag is false |
| `test_max_retries_override` | Rule max_retracts overrides profile default |
| `test_invalid_regex_rejected` | Validation rejects bad patterns |
| `test_max_rules_limit` | Enforces 100 rules per profile |

### 10.2 Integration Tests

| Test | Description |
|------|-------------|
| `test_e2e_retry_with_profile_rule` | Full payment flow with custom rule |
| `test_rule_crud_operations` | Create, read, update, delete |
| `test_bulk_import` | Import multiple rules |
| `test_profile_deletion_cascades` | Rules deleted with profile |
| `test_concurrent_rule_creation` | Handle race conditions |

### 10.3 Test Scenarios

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_profile_override_takes_precedence() {
        // Setup: Create profile with retry_rules_enabled=true
        // Create profile override for stripe/card_declined -> retry
        // Verify: GSM lookup returns profile override, not global GSM
    }

    #[tokio::test]
    async fn test_fallback_to_global_gsm() {
        // Setup: Create profile with retry_rules_enabled=true
        // No override for specific error code
        // Verify: Falls back to global GSM
    }

    #[tokio::test]
    async fn test_max_retries_priority() {
        // Setup: 
        // - Profile max_auto_retries_enabled = 3
        // - Rule max_retries = 1
        // Verify: Uses rule's max_retries (1)
    }
}
```

---

## 11. Migration Timeline

### Phase 1: Database Schema (Week 1)
- [ ] Create migration for `profile_gsm_override` table
- [ ] Add `retry_rules_enabled` to `business_profile`
- [ ] Run migration in development environment
- [ ] Verify schema in staging

### Phase 2: Core Implementation (Week 2-3)
- [ ] Implement database interface (`ProfileGsmOverrideInterface`)
- [ ] Implement common types (`RetryStrategy`, etc.)
- [ ] Modify GSM lookup to check profile overrides
- [ ] Update retry logic to use override max_retries
- [ ] Add metrics and logging

### Phase 3: API & Integration (Week 4)
- [ ] Implement API handlers
- [ ] Add API routes
- [ ] Update OpenAPI specification
- [ ] Add API tests
- [ ] Integration testing

### Phase 4: Documentation & Release (Week 5)
- [ ] Write user documentation
- [ ] Create merchant guide
- [ ] Update API reference
- [ ] Release notes
- [ ] Deploy to production

---

## 12. Future Enhancements

### Phase 2 Features
1. **Conditional Rules**: Apply rules based on payment amount, currency, or other attributes
2. **Time-Based Rules**: Different retry behavior during business hours vs off-hours
3. **A/B Testing**: Test different retry strategies for same error codes
4. **ML-Based Suggestions**: Suggest retry rules based on merchant's historical data
5. **Rule Templates**: Pre-built rule sets for common use cases (high-risk, conservative, aggressive)

### Example Future Schema
```json
{
    "conditions": {
        "amount_min": 1000,
        "amount_max": 10000,
        "currencies": ["USD", "EUR"],
        "payment_methods": ["card"]
    },
    "time_restrictions": {
        "business_hours_only": true,
        "timezone": "America/New_York"
    }
}
```

---

## 13. Appendix

### A. Existing GSM Table Schema

```sql
CREATE TABLE gateway_status_map (
    id UUID PRIMARY KEY,
    connector VARCHAR(255) NOT NULL,
    flow VARCHAR(255) NOT NULL,
    sub_flow VARCHAR(255) NOT NULL,
    code VARCHAR(255) NOT NULL,
    message TEXT,
    status VARCHAR(255) NOT NULL,
    decision VARCHAR(255) NOT NULL,
    step_up_possible BOOLEAN DEFAULT FALSE,
    unified_code VARCHAR(255),
    unified_message TEXT,
    error_category VARCHAR(255),
    clear_pan_possible BOOLEAN DEFAULT FALSE,
    feature_data JSONB,
    created_at TIMESTAMP DEFAULT NOW(),
    modified_at TIMESTAMP DEFAULT NOW()
);
```

### B. Example Error Codes by Connector

| Connector | Error Code | Typical Meaning | Suggested Default Decision |
|-----------|------------|-----------------|---------------------------|
| Stripe | `card_declined` | Generic card decline | DoDefault |
| Stripe | `insufficient_funds` | Not enough balance | DoDefault |
| Stripe | `lost_card` | Card reported lost | DoDefault |
| Stripe | `stolen_card` | Card reported stolen | DoDefault |
| Adyen | `IssuerSuspectedFraud` | Fraud suspected | DoDefault |
| Adyen | `Referral` | Issuer referral | Retry |
| Adyen | `AcquirerError` | Acquirer issue | Retry |
| Braintree | `gateway_rejected` | Gateway rejection | DoDefault |
| PayPal | `INSUFFICIENT_FUNDS` | Not enough balance | DoDefault |
| Checkout.com | `card_declined` | Generic decline | DoDefault |

### C. Retry Strategy Examples

```json
// Example 1: Simple retry with next available connector
{
    "strategy_type": "next_connector",
    "initial_delay_seconds": 5,
    "step_up_enabled": false,
    "clear_pan_enabled": false
}

// Example 2: Step-up authentication retry
{
    "strategy_type": "step_up",
    "initial_delay_seconds": 0,
    "step_up_enabled": true
}

// Example 3: Exponential backoff with same connector
{
    "strategy_type": "same_connector",
    "initial_delay_seconds": 10,
    "backoff_multiplier": 2.0,
    "max_delay_seconds": 300
}

// Example 4: Retry with specific fallback connector
{
    "strategy_type": "specific_connector",
    "initial_delay_seconds": 5,
    "connector": "adyen"
}
```

### D. Database Query Examples

```sql
-- Find all retry rules for a profile
SELECT * FROM profile_gsm_override 
WHERE profile_id = 'prof_abc123'
ORDER BY connector, error_code;

-- Find specific override for an error
SELECT * FROM profile_gsm_override 
WHERE profile_id = 'prof_abc123' 
  AND connector = 'stripe' 
  AND error_code = 'card_declined';

-- Count rules per profile
SELECT profile_id, COUNT(*) as rule_count
FROM profile_gsm_override
GROUP BY profile_id
HAVING COUNT(*) > 50;

-- Find most commonly overridden error codes
SELECT connector, error_code, COUNT(*) as override_count
FROM profile_gsm_override
GROUP BY connector, error_code
ORDER BY override_count DESC
LIMIT 20;
```

---

**End of Technical Specification**
