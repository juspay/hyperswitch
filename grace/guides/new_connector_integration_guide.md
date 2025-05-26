# Hyperswitch AI Connector Integration Guide

## Overview
This guide enables AI systems to autonomously integrate new payment connectors into Hyperswitch by following an atomic, step-by-step process. The workflow transforms connector API documentation into working code through two key artifacts: Technical Specifications and Implementation Plan.

## Phase 0: Initial Setup & Context Loading

### Step 0.1: Load Memory Bank Context
**Input**: Memory bank directory path
**Action**: Load all memory bank files in order:
1. `projectbrief.md`
2. `productContext.md` 
3. `systemPatterns.md`
4. `techContext.md`
5. `activeContext.md`
6. `progress.md`
7. `guides/connector_integration_guide.md`
8. `guides/patterns/patterns.md`
9. `guides/errors/errors.md`
10. `guides/learning/learning.md`
11. `guides/types/types.md`

**Output**: Fully loaded context understanding

### Step 0.2: Gather Connector Information
**Input**: User request
**Action**: Extract from user:
- Connector name (e.g., "ExamplePay")
- API documentation URL
- Base API URL
- Authentication type (HeaderKey/BodyKey/SignatureKey)
- Supported payment methods
- Any specific requirements

**Output**: Connector metadata object

## Phase 1: Technical Specification Generation

### Step 1.1: Create Connector Directory
**Input**: Connector name
**Action**: 
```bash
mkdir -p connector_integration/{connector_name}/
```
**Output**: Directory created

### Step 1.2: Analyze API Documentation
**Input**: API documentation URL/content
**Action**: For each supported payment method and flow, extract:

#### Authentication Analysis
```yaml
authentication:
  type: [HeaderKey|BodyKey|SignatureKey]
  fields:
    - name: api_key
      location: header
      format: "Bearer {value}"
    - name: merchant_id
      location: body
      format: plain
  signature_method: [HMAC-SHA256|RSA|etc]
  signature_payload: "{method}{path}{timestamp}{body}"
```

#### Endpoint Analysis (per flow)
```yaml
flow: [Authorize|Capture|Refund|Void|Sync]
endpoint:
  method: [POST|GET|PUT]
  path: "/v1/payments"
  headers:
    - Content-Type: "application/json"
    - X-API-Version: "2023-01-01"
  request_body:
    amount:
      type: [integer|string|float]
      format: [minor_units|major_units]
      example: 1000
    currency:
      type: string
      format: ISO-4217
      example: "USD"
    card:
      number:
        type: string
        format: "masked"
        example: "4242****4242"
      expiry_month:
        type: string
        example: "12"
      expiry_year:
        type: string
        format: [2-digit|4-digit]
        example: "2025"
  response_body:
    id:
      type: string
      example: "txn_1234"
    status:
      type: string
      enum: ["succeeded", "failed", "pending"]
    error:
      code: string
      message: string
```

### Step 1.3: Generate Technical Specification
**Input**: Analyzed API data
**Action**: Create `connector_integration/{connector_name}/tech-spec.md` using template in `connector_integration/template/tech-spec.md`

```markdown
# {ConnectorName} Technical Specification

## 1. Connector Overview
- **Connector Name**: {ConnectorName}
- **Connector Pascal Case**: {ConnectorPascalCase}
- **Connector Snake Case**: {connector_snake_case}
- **API Base URL**: {base_url}
- **API Documentation**: {doc_url}
- **Supported Payment Methods**: [Cards, Wallets, BankRedirect]
- **Supported Flows**: [Authorize, Capture, Refund, Void, Sync]

## 2. Authentication Structure
```rust
pub struct {ConnectorPascalCase}AuthType {
    pub api_key: Secret<String>,
    pub merchant_id: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for {ConnectorPascalCase}AuthType {
    // Implementation details based on auth type
}
```

## 3. Amount Handling
- **API Unit**: [MinorUnit|MajorUnit]
- **Hyperswitch Unit**: MinorUnit
- **Conversion Required**: [Yes|No]
- **Helper Struct**:
```rust
pub struct {ConnectorPascalCase}RouterData<T> {
    pub amount: {MinorUnit|StringMajorUnit},
    pub router_data: T,
}
```

## 4. Request Structures

### 4.1 Payment Request
```rust
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct {ConnectorPascalCase}PaymentsRequest {
    pub amount: {String|i64|f64},
    pub currency: common_enums::Currency,
    pub payment_method: {ConnectorPascalCase}PaymentMethod,
    // Additional fields from API analysis
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct {ConnectorPascalCase}Card {
    pub number: cards::CardNumber,
    pub expiry_month: Secret<String>,
    pub expiry_year: Secret<String>,
    pub cvv: Secret<String>,
}
```

## 5. Response Structures

### 5.1 Payment Response
```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct {ConnectorPascalCase}PaymentsResponse {
    pub id: String,
    pub status: {ConnectorPascalCase}PaymentStatus,
    pub amount: {String|i64|f64},
    // Additional fields from API analysis
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum {ConnectorPascalCase}PaymentStatus {
    Succeeded,
    Failed,
    Processing,
    // Additional statuses from API
}
```

## 6. Status Mappings
```rust
impl From<{ConnectorPascalCase}PaymentStatus> for common_enums::AttemptStatus {
    fn from(status: {ConnectorPascalCase}PaymentStatus) -> Self {
        match status {
            {ConnectorPascalCase}PaymentStatus::Succeeded => Self::Charged,
            {ConnectorPascalCase}PaymentStatus::Failed => Self::Failure,
            {ConnectorPascalCase}PaymentStatus::Processing => Self::Pending,
        }
    }
}
```

## 7. Error Structure
```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct {ConnectorPascalCase}ErrorResponse {
    pub code: String,
    pub message: String,
    pub details: Option<Vec<ErrorDetail>>,
}
```

## 8. Flow Specifications

### 8.1 Authorize Flow
- **Endpoint**: POST {base_url}/v1/payments
- **TryFrom Implementation**:
```rust
impl TryFrom<&{ConnectorPascalCase}RouterData<&PaymentsAuthorizeRouterData>> 
    for {ConnectorPascalCase}PaymentsRequest {
    // Field mapping implementation
}
```

[Repeat for each supported flow]
```

**Output**: Complete tech-spec.md file

## Phase 2: Implementation Plan Generation

### Step 2.1: Generate Implementation Plan
**Input**: tech-spec.md content
**Action**: Create `connector_integration/{connector_name}/planner-steps.md` using `connector_integration/template/planner-steps.md`

```markdown
# Implementation Plan: {ConnectorName} Integration

## Phase A: Preparation & Setup

### Step A.1: Verify Prerequisites
- [ ] Rust nightly toolchain installed
- [ ] API documentation reviewed
- [ ] Sandbox credentials obtained
**User Action**: Confirm prerequisites

### Step A.2: Generate Connector Template
```bash
sh scripts/add_connector.sh {connector_snake_case} {base_url}
```
**Expected Output**:
- `crates/hyperswitch_connectors/src/connectors/{connector_snake_case}/transformers.rs`
- `crates/hyperswitch_connectors/src/connectors/{connector_snake_case}.rs`
**User Action**: Move test file from module to tests directory

## Phase B: Transformer Implementation

### Step B.1: Define Core Types
**File**: `transformers.rs`
**Action**: Add authentication struct
```rust
{auth_struct_from_tech_spec}
```
**Validation**: Struct compiles without errors

### Step B.2: Define Request Structures
**File**: `transformers.rs`
**Action**: Add payment request structs
```rust
{request_structs_from_tech_spec}
```
**Validation**: All required fields present

### Step B.3: Define Response Structures
**File**: `transformers.rs`
**Action**: Add response structs
```rust
{response_structs_from_tech_spec}
```
**Validation**: Status enum includes all API statuses

### Step B.4: Implement Status Mappings
**File**: `transformers.rs`
**Action**: Add status conversion
```rust
{status_mapping_from_tech_spec}
```
**Validation**: All statuses mapped

### Step B.5: Implement Request TryFrom
**File**: `transformers.rs`
**Action**: Add request transformation
```rust
impl TryFrom<&{ConnectorPascalCase}RouterData<&PaymentsAuthorizeRouterData>>
    for {ConnectorPascalCase}PaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    
    fn try_from(item: &{ConnectorPascalCase}RouterData<&PaymentsAuthorizeRouterData>) 
        -> Result<Self, Self::Error> {
        // Extract payment method data
        let payment_method = match item.router_data.request.payment_method_data {
            PaymentMethodData::Card(ref card) => {
                {ConnectorPascalCase}PaymentMethod::Card({ConnectorPascalCase}Card {
                    number: card.card_number.clone(),
                    expiry_month: card.card_exp_month.clone(),
                    expiry_year: card.card_exp_year.clone(),
                    cvv: card.card_cvc.clone(),
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment method not supported"))?
        };
        
        Ok(Self {
            amount: item.amount.clone(),
            currency: item.router_data.request.currency,
            payment_method,
            // Map other fields
        })
    }
}
```
**Validation**: Transformation handles all payment methods

### Step B.6: Implement Response TryFrom
**File**: `transformers.rs`
**Action**: Add response transformation
```rust
impl TryFrom<ResponseRouterData<Authorize, {ConnectorPascalCase}PaymentsResponse, PaymentsAuthorizeData, PaymentsResponseData>>
    for PaymentsAuthorizeRouterData {
    type Error = error_stack::Report<errors::ConnectorError>;
    
    fn try_from(item: ResponseRouterData<Authorize, {ConnectorPascalCase}PaymentsResponse, PaymentsAuthorizeData, PaymentsResponseData>) 
        -> Result<Self, Self::Error> {
        Ok(Self {
            status: common_enums::AttemptStatus::from(item.response.status),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                charge_id: None,
            }),
            ..item.data
        })
    }
}
```
**Validation**: All response fields mapped

## Phase C: Main Logic Implementation

### Step C.1: Implement ConnectorCommon
**File**: `{connector_snake_case}.rs`
**Action**: Fill trait methods
```rust
impl ConnectorCommon for {ConnectorPascalCase} {
    fn id(&self) -> &'static str {
        "{connector_snake_case}"
    }
    
    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::{MinorUnit|Base}
    }
    
    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }
    
    fn base_url<'a>(&self, connectors: &'a Connectors) -> CustomResult<&'a str, errors::ConnectorError> {
        Ok(connectors.{connector_snake_case}.base_url.as_ref())
    }
    
    fn get_auth_header(&self, auth_type: &ConnectorAuthType) 
        -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        let auth = {connector_snake_case}::{ConnectorPascalCase}AuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(
            headers::AUTHORIZATION.to_string(),
            format!("Bearer {}", auth.api_key.peek()).into_masked()
        )])
    }
    
    fn build_error_response(&self, res: types::Response, event_builder: Option<&mut ConnectorEvent>) 
        -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: {connector_snake_case}::{ConnectorPascalCase}ErrorResponse = res.response
            .parse_struct("{ConnectorPascalCase}ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        
        event_builder.map(|e| e.set_response_body(&response));
        
        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.code,
            message: response.message,
            reason: response.details.map(|d| d.to_string()),
            attempt_status: None,
            connector_transaction_id: None,
        })
    }
}
```
**Validation**: All methods implemented

### Step C.2: Implement Authorize Flow
**File**: `{connector_snake_case}.rs`
**Action**: Implement ConnectorIntegration for Authorize
```rust
impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for {ConnectorPascalCase} {
    fn get_url(&self, _req: &PaymentsAuthorizeRouterData, connectors: &Connectors) 
        -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}/v1/payments", self.base_url(connectors)?))
    }
    
    fn get_request_body(&self, req: &PaymentsAuthorizeRouterData, _connectors: &Connectors) 
        -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount = utils::convert_amount(
            self.amount_converter,
            req.request.minor_amount,
            req.request.currency,
        )?;
        
        let connector_router_data = {connector_snake_case}::{ConnectorPascalCase}RouterData::from((
            amount,
            req,
        ));
        
        let connector_req = {connector_snake_case}::{ConnectorPascalCase}PaymentsRequest::try_from(&connector_router_data)?;
        
        Ok(RequestContent::Json(Box::new(connector_req)))
    }
    
    fn handle_response(&self, data: &PaymentsAuthorizeRouterData, event_builder: Option<&mut ConnectorEvent>, res: types::Response) 
        -> CustomResult<PaymentsAuthorizeRouterData, errors::ConnectorError> {
        let response: {connector_snake_case}::{ConnectorPascalCase}PaymentsResponse = res.response
            .parse_struct("{ConnectorPascalCase}PaymentsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        
        event_builder.map(|e| e.set_response_body(&response));
        
        PaymentsAuthorizeRouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
        })
    }
}
```
**Validation**: Request/response cycle works

[Repeat Step C.2 for each supported flow]

## Phase D: Registration & Configuration

### Step D.1: Update Core Enums
**File**: `crates/common_enums/src/connector_enums.rs`
**Action**: Add to Connector enum
```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Connector {
    // ... existing connectors
    {ConnectorPascalCase},
}
```
**Action**: Update From implementations
**Validation**: Enum compiles

### Step D.2: Backend Configuration
**File**: `crates/connector_configs/toml/development.toml`
**Action**: Add configuration
```toml
[{connector_snake_case}]
base_url = "{base_url}"

[{connector_snake_case}.connector_auth.{AuthType}]
api_key = "test_api_key"
merchant_id = "test_merchant"
```
**Validation**: Config loads correctly

### Step D.3: Test Configuration
**File**: `crates/router/tests/connectors/sample_auth.toml`
**Action**: Add test credentials
```toml
[{connector_snake_case}]
api_key = "sandbox_key"
merchant_id = "sandbox_merchant"
```
**Validation**: Tests can read auth

## Phase E: Testing

### Step E.1: Run Unit Tests
```bash
cargo test --package hyperswitch_connectors --lib {connector_snake_case}
```
**Expected**: All unit tests pass

### Step E.2: Run Integration Tests
```bash
export CONNECTOR_AUTH_FILE_PATH="crates/router/tests/connectors/sample_auth.toml"
cargo test --package router --test connectors -- {connector_snake_case} --test-threads=1
```
**Expected**: Basic flow tests pass

### Step E.3: Manual Testing
**Action**: Test each flow with curl/Postman
**Validation**: Responses match expectations
```

**Output**: Complete planner-steps.md file

## Phase 3: Code Generation & Implementation

### Step 3.1: Execute Implementation Plan
**Input**: planner-steps.md
**Action**: For each step in the plan:
1. Read the step action
2. Generate the specified code
3. Write to the specified file
4. Run the validation command
5. Record completion status

**Output**: Fully implemented connector

## Phase 4: Validation & Testing

### Step 4.1: Compile Check
```bash
cargo check --package hyperswitch_connectors
```
**Expected**: No compilation errors

### Step 4.2: Lint Check
```bash
cargo clippy --package hyperswitch_connectors -- -D warnings
```
**Expected**: No clippy warnings

### Step 4.3: Format Check
```bash
cargo +nightly fmt --package hyperswitch_connectors -- --check
```
**Expected**: Code properly formatted

### Step 4.4: Test Execution
```bash
cargo test --package router --test connectors -- {connector_name} --test-threads=1
```
**Expected**: All tests pass

## Common Patterns Reference

### Authentication Patterns
```rust
// HeaderKey - API key in header
headers::AUTHORIZATION => format!("Bearer {}", auth.api_key.peek())

// BodyKey - Multiple keys
auth.api_key => "X-API-Key" header
auth.key1 => "X-Merchant-ID" header

// SignatureKey - HMAC signature
let signature = crypto::HmacSha256::sign_message(
    &crypto::HmacSha256,
    auth.api_secret.peek().as_bytes(),
    payload.as_bytes(),
)?;
```

### Amount Conversion Patterns
```rust
// Minor to Major (cents to dollars)
let amount_f64 = (amount_i64 as f64) / 100.0;

// Using utility
let amount = utils::to_currency_base_unit(amount_minor, currency)?;
```

### Error Handling Patterns
```rust
// Field validation
.ok_or_else(|| errors::ConnectorError::MissingRequiredField { 
    field_name: "email" 
})?

// Parse errors
.change_context(errors::ConnectorError::ResponseDeserializationFailed)?

// Not implemented
Err(errors::ConnectorError::NotImplemented("Feature not supported"))?
```

### Common Type Imports
```rust
use common_utils::{
    pii::{Email, IpAddress},
    types::{MinorUnit, StringMajorUnit},
};
use hyperswitch_domain_models::{
    payment_method_data::{PaymentMethodData, Card},
    router_request_types::PaymentsAuthorizeData,
    router_response_types::PaymentsResponseData,
};
use masking::{ExposeInterface, PeekInterface, Secret};
```

## Success Criteria
1. All code compiles without errors
2. All tests pass
3. API calls succeed in sandbox environment
4. Status mappings work correctly
5. Error handling covers all cases
6. Amount conversions are accurate
7. Authentication works for all supported methods

## Troubleshooting Guide

### Common Errors
1. **Import not found**: Check `guides/types/types.md` for correct import paths
2. **Type mismatch**: Verify amount unit conversions
3. **Authentication failure**: Confirm auth type matches API documentation
4. **Status mapping**: Ensure all API statuses are covered
5. **Missing fields**: Add `#[serde(skip_serializing_if = "Option::is_none")]`

### Debug Commands
```bash
# Check specific connector compilation
cargo check --package hyperswitch_connectors --features "{connector_name}"

# Run specific test
cargo test --package router --test connectors {connector_name}::test_payment_authorize

# View generated code
bat crates/hyperswitch_connectors/src/connectors/{connector_name}/
```