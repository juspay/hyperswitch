# Monex Connector - Technical Specification

## 1. System Overview

### Core Purpose
To integrate Monex (a Canadian payment processing provider) with Hyperswitch, enabling card payment processing through the Monex gateway.

### Key Workflows
1. **Card Payments Authorization**: Process card payments with optional 3DS support
2. **Payment Capture**: Support manual capture after authorization
3. **Payment Sync**: Check payment status
4. **Refund Processing**: Process full/partial refunds
5. **Refund Sync**: Check refund status

### System Architecture
The connector will follow Hyperswitch's standard connector architecture:
- **mod.rs**: Main connector implementation with trait implementations
- **transformers.rs**: Data type transformations between Hyperswitch and Monex
- **test.rs**: Integration tests for the connector

## 2. Project Structure

```
crates/hyperswitch_connectors/src/connectors/
├── monex
│   └── transformers.rs
└── monex.rs

crates/router/tests/connectors/
└── monex.rs
```

## 3. Feature Specification

### 3.1 Authentication

#### User Story
As a merchant using Hyperswitch with Monex, I need to securely authenticate my API requests to the Monex payment gateway.

#### Implementation Details
- **Auth Type**: OAuth2 Bearer Token
- **Required Credentials**: 
  - Client ID
  - Client Secret
- **Storage**: The credentials will be stored in the connector authentication configuration
- **Implementation Steps**:
  1. Define `MonexAuthType` struct with required fields
  2. Implement `TryFrom<&ConnectorAuthType>` to extract credentials
  3. Add token generation functionality to create Bearer token for requests
  4. Add `Authorization: Bearer <token>` header to all API requests

#### Error Handling
- Handle authentication failures (invalid credentials, expired tokens)
- Implement token refresh logic if token expires

### 3.2 Card Payment Processing

#### User Story
As a merchant, I want to process card payments through Monex to charge customers securely.

#### Implementation Details
- **Supported Methods**: Credit/debit cards
- **Flows**: 
  - Authorization only (manual capture)
  - Authorize and capture (automatic)
- **Request Mapping**:
  1. Map Hyperswitch `PaymentsAuthorizeRouterData` to `MonexPaymentsRequest`
  2. Convert amount to correct format (string/minor unit)
  3. Extract card details (number, expiry, CVC)
  4. Map currency and other metadata
- **Response Mapping**:
  1. Map Monex payment status to Hyperswitch `AttemptStatus`
  2. Extract connector transaction ID
  3. Handle error responses

#### Error Handling
- Card validation errors (invalid card, expired, insufficient funds)
- Processing errors from Monex gateway
- Network/communication errors

### 3.3 Payment Capture

#### User Story
As a merchant, I want to capture previously authorized payments at a later time.

#### Implementation Details
- **Request Mapping**:
  1. Map Hyperswitch `PaymentsCaptureRouterData` to `MonexPaymentsCaptureRequest`
  2. Include original payment ID from the authorization
  3. Include capture amount (support partial captures)
- **Response Mapping**:
  1. Map capture response status to Hyperswitch status
  2. Handle capture failures

#### Error Handling
- Authorization expired
- Amount exceeding authorized amount
- Invalid payment ID

### 3.4 Payment Sync (PSync)

#### User Story
As a merchant, I want to check the current status of payments processed through Monex.

#### Implementation Details
- **Request Mapping**:
  1. Map Hyperswitch `PaymentsSyncRouterData` to Monex GET request
  2. Use connector transaction ID to identify the payment
- **Response Mapping**:
  1. Map Monex payment status to Hyperswitch `AttemptStatus`
  2. Handle payment state changes

#### Error Handling
- Payment not found
- Invalid payment ID format
- Timeout/network errors

### 3.5 Refund Processing

#### User Story
As a merchant, I want to process refunds for payments made through Monex.

#### Implementation Details
- **Request Mapping**:
  1. Map Hyperswitch `RefundsRouterData` to `MonexRefundRequest`
  2. Include original payment ID
  3. Include refund amount (support partial refunds)
- **Response Mapping**:
  1. Map Monex refund status to Hyperswitch `RefundStatus`
  2. Extract connector refund ID

#### Error Handling
- Payment not found
- Refund amount exceeding original payment
- Already refunded payment

### 3.6 Refund Sync (RSync)

#### User Story
As a merchant, I want to check the current status of refunds processed through Monex.

#### Implementation Details
- **Request Mapping**:
  1. Map Hyperswitch `RefundSyncRouterData` to Monex GET request
  2. Use connector refund ID to identify the refund
- **Response Mapping**:
  1. Map Monex refund status to Hyperswitch `RefundStatus`
  2. Handle refund state changes

#### Error Handling
- Refund not found
- Invalid refund ID format
- Timeout/network errors

## 4. Data Structures

### 4.1 Request/Response Types

#### Authentication
```rust
pub struct MonexAuthType {
    pub client_id: Secret<String>,
    pub client_secret: Secret<String>,
}

pub struct MonexAccessToken {
    pub token: Secret<String>,
    pub expires_at: i64,
}
```

#### Payment Types
```rust
#[derive(Debug, Serialize)]
pub struct MonexPaymentsRequest {
    pub amount: String,
    pub currency: String,
    pub card: MonexCard,
    pub merchant_order_id: String,
}

#[derive(Debug, Serialize)]
pub struct MonexCard {
    pub number: cards::CardNumber,
    pub exp_month: Secret<String>,
    pub exp_year: Secret<String>,
    pub cvc: Secret<String>,
}

#[derive(Debug, Deserialize)]
pub struct MonexPaymentsResponse {
    pub payment_id: String,
    pub status: MonexPaymentStatus,
}

#[derive(Debug, Deserialize)]
pub enum MonexPaymentStatus {
    authorized,
    captured,
    failed,
    pending,
}
```

#### Capture Types
```rust
#[derive(Debug, Serialize)]
pub struct MonexPaymentsCaptureRequest {
    pub amount: String,
}

// Uses MonexPaymentsResponse for response
```

#### Refund Types
```rust
#[derive(Debug, Serialize)]
pub struct MonexRefundRequest {
    pub amount: String,
}

#[derive(Debug, Deserialize)]
pub struct MonexRefundResponse {
    pub payment_id: String,
    pub status: MonexRefundStatus,
}

#[derive(Debug, Deserialize)]
pub enum MonexRefundStatus {
    refunded,
    failed,
    pending,
}
```

#### Error Types
```rust
#[derive(Debug, Deserialize)]
pub struct MonexErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}
```

### 4.2 Status Mapping

#### Payment Status Mapping
| Monex Status | Hyperswitch AttemptStatus |
|--------------|---------------------------|
| authorized   | Authorized                |
| captured     | Charged                   |
| failed       | Failure                   |
| pending      | Pending                   |

#### Refund Status Mapping
| Monex Status | Hyperswitch RefundStatus |
|--------------|--------------------------|
| refunded     | Success                  |
| failed       | Failure                  |
| pending      | Pending                  |

## 5. Implementation Plan

### 5.1 Phase 1: Core Setup

1. Create basic connector structure
   - Implement `monex.rs` with required traits
   - Define auth types and token management
   - Set up base URL and headers

2. Define data structures
   - Create request/response types in `transformers.rs`
   - Implement status enums and mappings
   - Define error handling

### 5.2 Phase 2: Payment Flows

1. Implement payment authorization
   - Map card payment request/response
   - Handle status mapping
   - Implement error handling

2. Implement payment capture
   - Create capture request/response mapping
   - Handle capture-specific errors

3. Implement payment sync
   - Create payment status check functionality
   - Map response status to Hyperswitch status

### 5.3 Phase 3: Refund Flows

1. Implement refund processing
   - Create refund request/response mapping
   - Handle refund-specific errors

2. Implement refund sync
   - Create refund status check functionality
   - Map response status to Hyperswitch status

### 5.4 Phase 4: Testing & Documentation

1. Create test cases
   - Happy path tests for all flows
   - Error case tests
   - Edge case handling

2. Documentation
   - Update integration guide
   - Document connector capabilities
   - Add configuration instructions

## 6. API Endpoints

### Base URLs
- **Production**: `https://api.monexgroup.com/v1/`
- **Sandbox**: `https://sandbox.api.monexgroup.com/v1/`

### Endpoints

| Operation | Endpoint | HTTP Method | Description |
|-----------|----------|-------------|-------------|
| Authentication | `/oauth/token` | POST | Obtain access token |
| Authorize | `/payments/authorize` | POST | Authorize a payment |
| Capture | `/payments/capture/{payment_id}` | POST | Capture an authorized payment |
| Payment Status | `/payments/{payment_id}` | GET | Check payment status |
| Refund | `/payments/refund/{payment_id}` | POST | Refund a payment |
| Refund Status | `/payments/{payment_id}` | GET | Check refund status |

## 7. Error Handling

### Common Errors
1. **Authentication Errors**
   - Invalid credentials
   - Expired token
   - Missing authentication

2. **Validation Errors**
   - Invalid card details
   - Invalid amount
   - Unsupported currency

3. **Processing Errors**
   - Insufficient funds
   - Card declined
   - Risk assessment failure

4. **System Errors**
   - Network failures
   - Timeout errors
   - Internal server errors

### Error Response Mapping
Map Monex error responses to appropriate Hyperswitch error types to ensure consistent error handling across connectors.

## 8. Testing Strategy

### Unit Tests
- Test data transformations between Hyperswitch and Monex formats
- Test status mappings for all scenarios

### Integration Tests
- Test authentication flow
- Test payment authorization (with and without 3DS)
- Test payment capture
- Test payment sync
- Test refund processing
- Test refund sync
- Test error handling for common error cases

### Manual Testing
- Verify successful payment processing through Monex sandbox
- Verify successful refund processing
- Verify proper error handling

## 9. Security Considerations

1. **PCI Compliance**
   - Ensure proper handling of card data
   - Use tokenization where possible
   - Avoid logging sensitive data

2. **Authentication Security**
   - Secure storage of credentials
   - Proper token management
   - Timeout handling

3. **Data Validation**
   - Validate all input data
   - Sanitize outputs
   - Prevent injection attacks

## 10. Future Enhancements

1. **Additional Payment Methods**
   - Support for additional Monex payment methods beyond cards

2. **Webhook Support**
   - Implement webhook handlers for asynchronous updates

3. **Dispute Handling**
   - Add support for dispute management via API

4. **Reporting Features**
   - Implement reporting and analytics features

## 11. Implementation Notes

- Use existing patterns from similar connectors as references
- Reuse common utility functions for amount conversion
- Follow Hyperswitch coding standards and error handling patterns
- Ensure proper logging for debugging and monitoring
- Use appropriate masking for sensitive data (PII, card details)
