# Spreedly Connector Technical Specification for Hyperswitch

## 1. System Overview

### Core Purpose and Value Proposition
Spreedly is a global payments orchestration platform that enables businesses to connect with multiple payment gateways and services through a single API. The integration will add Spreedly as a payment connector to Hyperswitch, enabling:
- Card payment processing (major card brands)
- 3DS Authentication support
- Tokenization capabilities
- Full and partial refunds
- Webhook event notifications

### Key Workflows
1. **Authorization Flow**: Process card payments with direct authorization
2. **Capture Flow**: Capture previously authorized transactions
3. **Refund Flow**: Process full or partial refunds
4. **Sync Flow**: Retrieve transaction status and details
5. **Webhook Flow**: Handle transaction event notifications

### System Architecture
- **Base URL**: https://core.spreedly.com/v1/
- **Authentication**: HTTP Basic Authentication (Environment Key + Access Secret)
- **Data Format**: JSON
- **Rate Limits**: 30 requests per minute per environment

## 2. Project Structure

```
hyperswitch_connectors/src/connectors/
├── spreedly/
│   └── transformers.rs      # Request/Response transformations
└── spreedly.rs             # Main connector implementation

crates/router/tests/connectors/
└── spreedly.rs             # Connector tests
```

## 3. Feature Specification

### 3.1 Card Payment Authorization
**User Story**: As a merchant, I want to process card payments through Spreedly

**Implementation Steps**:
1. Convert Hyperswitch payment request to Spreedly authorize format
2. Include card details in the `credit_card` object
3. Convert amount to minor units (cents)
4. Handle tokenization (automatic during transaction)
5. Parse response and map to Hyperswitch format

**API Details**:
- **Endpoint**: `/v1/gateways/{gateway_token}/authorize.json`
- **Method**: POST
- **Required Fields**:
  - `credit_card.number`
  - `credit_card.verification_value`
  - `credit_card.month`
  - `credit_card.year`
  - `credit_card.first_name`
  - `credit_card.last_name`
  - `amount` (in minor units)
  - `currency_code`

**Error Handling**:
- 401: Authentication failed - Invalid credentials
- 422: Validation errors - Invalid card data or missing fields
- 500: Server error - Retry with exponential backoff

### 3.2 Transaction Capture
**User Story**: As a merchant, I want to capture authorized transactions

**Implementation Steps**:
1. Extract transaction token from authorization response
2. Convert capture amount to minor units
3. Send capture request to Spreedly
4. Map response to Hyperswitch capture response

**API Details**:
- **Endpoint**: `/v1/transactions/{transaction_token}/capture.json`
- **Method**: POST
- **Required Fields**:
  - `amount` (optional, defaults to full amount)

### 3.3 Refund Processing
**User Story**: As a merchant, I want to refund transactions

**Implementation Steps**:
1. Extract transaction token from payment reference
2. Convert refund amount to minor units
3. Send refund request to Spreedly
4. Map response to Hyperswitch refund response

**API Details**:
- **Endpoint**: `/v1/transactions/{transaction_token}/credit.json`
- **Method**: POST
- **Required Fields**:
  - `amount` (in minor units)

### 3.4 Payment Status Sync
**User Story**: As a merchant, I want to check transaction status

**Implementation Steps**:
1. Extract transaction token from payment reference
2. Send GET request to retrieve transaction details
3. Map Spreedly status to Hyperswitch payment status

**API Details**:
- **Endpoint**: `/v1/transactions/{transaction_token}.json`
- **Method**: GET

### 3.5 Webhook Handling
**User Story**: As a merchant, I want to receive transaction updates via webhooks

**Implementation Steps**:
1. Parse webhook payload
2. Extract event type and transaction details
3. Verify webhook authenticity
4. Map to Hyperswitch webhook event

**Supported Events**:
- `transaction_succeeded`
- `transaction_failed`
- `payment_method_added`

## 4. Database Schema
Not applicable - connector integration uses external API

## 5. Server Actions

### 5.1 Authentication Setup
**Description**: Configure Spreedly authentication credentials

**Implementation**:
```rust
pub struct SpreedlyAuthType {
    pub environment_key: Secret<String>,
    pub access_secret: Secret<String>,
}
```

### 5.2 Request Transformations

#### Authorize Request
**Input**: `PaymentsAuthorizeData`
**Output**: `SpreedlyAuthorizeRequest`
**Transformation**:
- Extract card details from `payment_method_data`
- Convert amount using `MinorUnit`
- Map currency code
- Build credit_card object

#### Capture Request
**Input**: `PaymentsCaptureData`
**Output**: `SpreedlyCaptureRequest`
**Transformation**:
- Extract transaction token from connector_transaction_id
- Convert amount to minor units

#### Refund Request
**Input**: `RefundsData`
**Output**: `SpreedlyRefundRequest`
**Transformation**:
- Extract transaction token from connector_transaction_id
- Convert refund amount to minor units

### 5.3 Response Transformations

#### Authorize Response
**Input**: `SpreedlyAuthorizeResponse`
**Output**: `PaymentsResponseData`
**Mapping**:
- `transaction.token` → `connector_transaction_id`
- `transaction.succeeded` → payment status
- `transaction.payment_method.token` → `connector_payment_method_token`

#### Status Mapping
```rust
match transaction.succeeded {
    true => AttemptStatus::Charged,
    false => AttemptStatus::Failure,
}
```

## 6. Design System
Not applicable - backend connector integration

## 7. Component Architecture

### 7.1 Connector Implementation
```rust
impl Connector for Spreedly {
    // Authentication configuration
    fn get_auth_header(&self, auth_type: &ConnectorAuthType) -> CustomResult<Vec<(String, String)>>
    
    // URL construction
    fn get_url(&self, req: &RouterData<T>) -> CustomResult<String>
    
    // Request building
    fn build_request(&self, req: &RouterData<T>) -> CustomResult<Option<Request>>
}
```

### 7.2 Error Response Handling
```rust
#[derive(Deserialize)]
pub struct SpreedlyErrorResponse {
    pub errors: Vec<ErrorDetail>,
}

#[derive(Deserialize)]
pub struct ErrorDetail {
    pub key: String,
    pub message: String,
}
```

## 8. Authentication & Authorization

### HTTP Basic Authentication
- Username: Environment Key
- Password: Access Secret
- Header: `Authorization: Basic base64(environment_key:access_secret)`

## 9. Data Flow

### Authorization Flow
1. Hyperswitch receives payment request
2. Transform to Spreedly format
3. Send authorize request with card data
4. Spreedly tokenizes card and processes
5. Return transaction token and status
6. Map response to Hyperswitch format

### Capture Flow
1. Retrieve transaction token from authorization
2. Send capture request
3. Process response
4. Update payment status

## 10. 3DS Integration
- Endpoint: `/v1/sca/providers/{sca_provider_key}/authenticate`
- Requires separate SCA provider configuration
- Optional based on merchant requirements

## 11. Testing

### Unit Tests
1. **Request Transformation Tests**
   - Test card data mapping
   - Test amount conversion
   - Test currency mapping

2. **Response Transformation Tests**
   - Test success response mapping
   - Test error response mapping
   - Test status mapping

3. **Authentication Tests**
   - Test header generation
   - Test credential masking

### Integration Tests
1. **Authorization Flow**
   - Test successful authorization
   - Test failed authorization
   - Test invalid card data

2. **Capture Flow**
   - Test full capture
   - Test partial capture

3. **Refund Flow**
   - Test full refund
   - Test partial refund

4. **Webhook Tests**
   - Test event parsing
   - Test signature verification

## 12. Implementation Notes

### Key Considerations
1. **Amount Handling**: Always use minor units (cents)
2. **Gateway Token**: Required for authorization - must be configured
3. **Transaction Token**: Used for capture, refund, and status operations
4. **Rate Limiting**: Implement retry logic with exponential backoff
5. **Error Codes**: Map Spreedly-specific errors to Hyperswitch error types

### Common Utilities to Use
- `MinorUnit` for amount conversion
- `Secret` for sensitive data masking
- `common_utils` for standard transformations
- Existing error handling patterns from other connectors
