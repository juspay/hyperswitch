# Spreedly Connector Integration Technical Specification

## 1. System Overview

### Core Purpose and Value Proposition
The Spreedly connector integration enables hyperswitch to process payments through Spreedly's global payments orchestration platform. Spreedly acts as a meta-gateway that allows businesses to connect with multiple payment gateways through a single API, providing secure card vaulting, tokenization, and support for various payment methods.

### Key Workflows
1. **Payment Authorization**: Process card payments with automatic tokenization
2. **Payment Capture**: Capture previously authorized payments (full or partial)
3. **Refund Processing**: Issue full or partial refunds for captured transactions
4. **Transaction Status Sync**: Query and update transaction status

### System Architecture
```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│   Hyperswitch   │────▶│ Spreedly         │────▶│ Target Gateway  │
│   Application   │     │ Connector        │     │ (via Spreedly)  │
└─────────────────┘     └──────────────────┘     └─────────────────┘
        │                        │
        │                        ▼
        │                 ┌──────────────────┐
        └────────────────▶│ Spreedly API     │
                          │ (HTTP Basic Auth)│
                          └──────────────────┘
```

## 2. Project Structure

### Initial Setup
Use the `add_connector.sh` script to generate the boilerplate code for the Spreedly connector:
```bash
./scripts/add_connector.sh spreedly
```

This will automatically create the required directory structure and boilerplate files.

### Directory Structure
```
crates/hyperswitch_connectors/src/connectors/
├── spreedly.rs           # Main connector implementation
└── spreedly/
    └── transformers.rs   # Request/response transformations

crates/hyperswitch_connectors/src/
├── connectors.rs   # Register Spreedly connector
└── lib.rs         # Export Spreedly module
```

### Module Organization
- **spreedly.rs**: Main connector file that implements connector traits for payment operations
- **spreedly/transformers.rs**: Handles data transformation between hyperswitch and Spreedly formats

## 3. Feature Specification

### 3.1 Payment Authorization
**User Story**: As a merchant, I want to authorize card payments through Spreedly so that funds can be captured later.

**Implementation Steps**:
1. Accept card payment request with raw card data
2. Extract gateway token from connector metadata
3. Transform request to Spreedly format:
   ```json
   {
     "transaction": {
       "credit_card": {
         "number": "4111111111111111",
         "verification_value": "123",
         "month": "12",
         "year": "2025",
         "first_name": "John",
         "last_name": "Doe"
       },
       "amount": 1000,
       "currency_code": "USD"
     }
   }
   ```
4. Send POST request to `/v1/gateways/{gateway_token}/authorize.json`
5. Parse response and extract transaction token
6. Map status to hyperswitch AttemptStatus
7. Store transaction token for future operations

**Error Handling**:
- Invalid card data: Return validation error
- Gateway token missing: Return configuration error
- API failures: Map HTTP status codes to appropriate errors
- Rate limit exceeded: Return temporary failure

### 3.2 Payment Capture
**User Story**: As a merchant, I want to capture previously authorized payments to complete the transaction.

**Implementation Steps**:
1. Retrieve transaction token from authorization
2. Build capture request with amount (supports partial capture)
3. Send POST request to `/v1/transactions/{transaction_token}/capture.json`
4. Update payment status based on response

**Edge Cases**:
- Partial capture amount validation
- Already captured transactions
- Expired authorizations

### 3.3 Refund Processing
**User Story**: As a merchant, I want to refund captured payments either fully or partially.

**Implementation Steps**:
1. Use transaction token from original payment
2. Build refund request:
   ```json
   {
     "transaction": {
       "amount": 500
     }
   }
   ```
3. Send POST request to `/v1/transactions/{transaction_token}/credit.json`
4. Map refund status from response

**Error Handling**:
- Refund amount exceeds captured amount
- Transaction not eligible for refund
- Multiple partial refunds tracking

### 3.4 Payment Status Sync
**User Story**: As a system, I need to query transaction status to keep payment records up to date.

**Implementation Steps**:
1. Use transaction token to query status
2. Send GET request to `/v1/transactions/{transaction_token}.json`
3. Parse transaction details from response
4. Update local payment status accordingly

## 4. Database Schema

### 4.1 Tables
No new tables required. The integration uses existing hyperswitch tables:

**payment_attempt** (existing):
- `connector_transaction_id`: Stores Spreedly transaction token
- `connector_metadata`: Stores gateway token and additional Spreedly data

**refund** (existing):
- `connector_refund_id`: Stores Spreedly refund transaction token

## 5. Server Actions

### 5.1 Database Actions
No specific database actions required beyond standard hyperswitch operations.

### 5.2 External API Integrations

#### Authorization Endpoint
- **URL**: `POST https://core.spreedly.com/v1/gateways/{gateway_token}/authorize.json`
- **Authentication**: HTTP Basic Auth (Environment Key:Access Secret)
- **Request Format**: JSON with credit card details and amount
- **Response**: Transaction token and status

#### Capture Endpoint
- **URL**: `POST https://core.spreedly.com/v1/transactions/{transaction_token}/capture.json`
- **Authentication**: HTTP Basic Auth
- **Request Format**: JSON with amount (optional for partial capture)
- **Response**: Capture transaction details

#### Refund Endpoint
- **URL**: `POST https://core.spreedly.com/v1/transactions/{transaction_token}/credit.json`
- **Authentication**: HTTP Basic Auth
- **Request Format**: JSON with refund amount
- **Response**: Refund transaction details

#### Status Sync Endpoint
- **URL**: `GET https://core.spreedly.com/v1/transactions/{transaction_token}.json`
- **Authentication**: HTTP Basic Auth
- **Response**: Complete transaction details

## 6. Design System

Not applicable for backend connector integration.

## 7. Component Architecture

### 7.1 Connector Module Structure

```rust
// mod.rs structure
pub struct Spreedly {
    amount_converter: &'static (dyn AmountConvertor<Output = StringMinorUnit> + Sync)
}

impl Spreedly {
    pub fn new() -> &'static Self {
        &Self {
            amount_converter: &StringMinorUnitForConnector
        }
    }
}

// Trait implementations
impl api::Payment for Spreedly {}
impl api::PaymentAuthorize for Spreedly {}
impl api::PaymentSync for Spreedly {}
impl api::PaymentCapture for Spreedly {}
impl api::Refund for Spreedly {}
impl api::RefundExecute for Spreedly {}
impl api::RefundSync for Spreedly {}
```

### 7.2 Data Transformation Components

```rust
// transformers.rs structures
pub struct SpreedlyPaymentsRequest {
    transaction: SpreedlyTransaction,
}

pub struct SpreedlyTransaction {
    credit_card: Option<SpreedlyCreditCard>,
    amount: StringMinorUnit,
    currency_code: common_enums::Currency,
}

pub struct SpreedlyCreditCard {
    number: cards::CardNumber,
    verification_value: Secret<String>,
    month: Secret<String>,
    year: Secret<String>,
    first_name: Secret<String>,
    last_name: Secret<String>,
}
```

## 8. Authentication & Authorization

### Spreedly Authentication Implementation
```rust
pub struct SpreedlyAuthType {
    pub environment_key: Secret<String>,
    pub access_secret: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for SpreedlyAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::HeaderKey { api_key } => {
                // Parse "environment_key:access_secret" format
                let auth_parts: Vec<&str> = api_key.peek().split(':').collect();
                if auth_parts.len() != 2 {
                    return Err(errors::ConnectorError::FailedToObtainAuthType.into());
                }
                Ok(Self {
                    environment_key: Secret::new(auth_parts[0].to_string()),
                    access_secret: Secret::new(auth_parts[1].to_string()),
                })
            }
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
```

### HTTP Basic Auth Header Construction
```rust
fn get_auth_header(&self, auth_type: &ConnectorAuthType) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
    let auth = SpreedlyAuthType::try_from(auth_type)?;
    let credentials = format!("{}:{}", 
        auth.environment_key.expose(), 
        auth.access_secret.expose()
    );
    let encoded = base64::encode(credentials);
    Ok(vec![(
        headers::AUTHORIZATION.to_string(), 
        format!("Basic {}", encoded).into_masked()
    )])
}
```

## 9. Data Flow

### Request Flow Diagram
```mermaid
sequenceDiagram
    participant HS as Hyperswitch
    participant SC as Spreedly Connector
    participant SA as Spreedly API
    
    HS->>SC: PaymentAuthorizeRequest
    SC->>SC: Transform to Spreedly format
    SC->>SC: Add gateway_token to URL
    SC->>SA: POST /gateways/{token}/authorize.json
    SA->>SC: Transaction response
    SC->>SC: Extract transaction_token
    SC->>SC: Map status to AttemptStatus
    SC->>HS: PaymentsResponseData
```

### State Transformations
1. **Payment Status Mapping**:
   - `succeeded` → `AttemptStatus::Charged`
   - `failed` → `AttemptStatus::Failure`
   - `processing` → `AttemptStatus::Authorizing`

2. **Refund Status Mapping**:
   - `succeeded` → `RefundStatus::Success`
   - `failed` → `RefundStatus::Failure`
   - `processing` → `RefundStatus::Pending`

## 10. Payment Implementation

### Authorization Flow
```rust
fn get_request_body(
    &self,
    req: &PaymentsAuthorizeRouterData,
    _connectors: &Connectors,
) -> CustomResult<RequestContent, errors::ConnectorError> {
    let amount = utils::convert_amount(
        self.amount_converter,
        req.request.minor_amount,
        req.request.currency,
    )?;
    
    let connector_req = SpreedlyPaymentsRequest::try_from((amount, req))?;
    Ok(RequestContent::Json(Box::new(connector_req)))
}
```

### Capture Flow
```rust
fn get_url(
    &self,
    req: &PaymentsCaptureRouterData,
    connectors: &Connectors,
) -> CustomResult<String, errors::ConnectorError> {
    let transaction_token = req.request.connector_transaction_id
        .as_ref()
        .ok_or(errors::ConnectorError::MissingRequiredField)?;
    
    Ok(format!(
        "{}/v1/transactions/{}/capture.json",
        self.base_url(connectors),
        transaction_token
    ))
}
```

### Refund Flow
```rust
fn get_request_body(
    &self,
    req: &RefundsRouterData<Execute>,
    _connectors: &Connectors,
) -> CustomResult<RequestContent, errors::ConnectorError> {
    let refund_amount = utils::convert_amount(
        self.amount_converter,
        req.request.minor_refund_amount,
        req.request.currency,
    )?;
    
    let connector_req = SpreedlyRefundRequest {
        transaction: SpreedlyRefundTransaction {
            amount: refund_amount,
        }
    };
    Ok(RequestContent::Json(Box::new(connector_req)))
}
```

## 11. Analytics Implementation

### Logging Strategy
```rust
// Log all API requests
router_env::logger::info!(
    spreedly_request = ?connector_req,
    "Sending Spreedly payment request"
);

// Log all API responses
router_env::logger::info!(
    spreedly_response = ?response,
    transaction_token = ?response.transaction.token,
    "Received Spreedly response"
);

// Log errors with context
router_env::logger::error!(
    error = ?e,
    transaction_token = ?transaction_token,
    "Spreedly API error"
);
```

### Metrics Collection
- API response time tracking using hyperswitch metrics
- Success/failure rate monitoring
- Rate limit tracking (30 requests/minute limit)

### Event Tracking
```rust
event_builder.map(|event| {
    event.set_request_body(&connector_req);
    event.set_response_body(&response);
    event.set_response_time(response_time);
    event.set_status_code(res.status_code);
});
```

## 12. Testing

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spreedly_auth_parsing() {
        let auth = ConnectorAuthType::HeaderKey {
            api_key: Secret::new("env_key:access_secret".to_string())
        };
        let result = SpreedlyAuthType::try_from(&auth);
        assert!(result.is_ok());
    }

    #[test]
    fn test_amount_conversion() {
        let amount = StringMinorUnit::new("1000".to_string());
        let currency = common_enums::Currency::USD;
        let result = utils::convert_amount(&StringMinorUnitForConnector, amount, currency);
        assert_eq!(result.unwrap().get_amount_as_string(), "1000");
    }

    #[test]
    fn test_payment_status_mapping() {
        let spreedly_status = SpreedlyPaymentStatus::Succeeded;
        let attempt_status: AttemptStatus = spreedly_status.into();
        assert_eq!(attempt_status, AttemptStatus::Charged);
    }
}
```

### Integration Tests
```rust
#[tokio::test]
async fn test_spreedly_payment_flow() {
    // Test authorization
    let auth_response = mock_spreedly_authorize().await;
    assert!(auth_response.succeeded);
    
    // Test capture
    let capture_response = mock_spreedly_capture(&auth_response.token).await;
    assert!(capture_response.succeeded);
    
    // Test refund
    let refund_response = mock_spreedly_refund(&auth_response.token).await;
    assert!(refund_response.succeeded);
}
```

### Key Test Scenarios
1. **Authorization Tests**:
   - Valid card authorization
   - Invalid card handling
   - Missing gateway token error
   - Rate limit handling

2. **Capture Tests**:
   - Full capture
   - Partial capture
   - Invalid transaction token

3. **Refund Tests**:
   - Full refund
   - Partial refund
   - Multiple partial refunds

4. **Error Handling Tests**:
   - 401 Authentication errors
   - 422 Validation errors
   - 500 Server errors
   - Network timeouts

## Additional Implementation Notes

### Gateway Token Management
The gateway token is a crucial component in Spreedly's architecture. It should be:
- Stored in connector metadata when creating merchant connector account
- Retrieved from request metadata for each transaction
- Used in the authorization URL path

### Error Response Handling
Spreedly returns different error formats based on the error type:
```rust
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct SpreedlyErrorResponse {
    pub errors: Option<Vec<SpreedlyError>>,
    pub message: Option<String>,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct SpreedlyError {
    pub key: String,
    pub message: String,
}
```

### Currency Support
Spreedly supports a wide range of currencies. The connector should:
- Validate currency codes against Spreedly's supported list
- Convert amounts to minor units (cents) for all currencies
- Handle currency-specific decimal places correctly

### 3DS Implementation (Future Enhancement)
While not included in the initial implementation, 3DS support can be added later:
- Use `/v1/sca/providers/{sca_provider_key}/authenticate` endpoint
- Requires integration with supported SCA providers
- Handle redirect flows for 3DS authentication

This specification provides a complete blueprint for implementing the Spreedly connector in hyperswitch, following the project's standards and best practices.
