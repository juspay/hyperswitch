# Maxpay Connector Technical Specification

## 1. System Overview

### Core Purpose and Value Proposition
Maxpay is an international payment gateway specializing in secure card payment processing for businesses, particularly in high-risk industries. This integration enables Hyperswitch merchants to leverage Maxpay's robust payment infrastructure with features including 3D Secure authentication, tokenization for recurring payments, and comprehensive chargeback protection.

### Key Workflows
1. **Payment Authorization (AUTH)** - Reserve funds on customer's card
2. **Payment Capture (SETTLE)** - Capture previously authorized funds
3. **Payment Status Sync (CHECK)** - Query current transaction status
4. **Refund Processing** - Process full refunds
5. **Card Tokenization (TOKENIZE)** - Store card details securely for recurring payments
6. **3D Secure Authentication** - Enhanced security with AUTH3D and SALE3D flows
7. **Webhook Processing** - Real-time transaction status updates

### System Architecture
```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   Hyperswitch   │────▶│ Maxpay Connector│────▶│   Maxpay API    │
│     Router      │◀────│   Implementation│◀────│  (Host-to-Host) │
└─────────────────┘     └─────────────────┘     └─────────────────┘
         │                      │                         │
         │                      ▼                         │
         │              ┌─────────────────┐              │
         │              │  Transformers   │              │
         │              │ (Type Mapping)  │              │
         │              └─────────────────┘              │
         │                                                │
         └──────────────── Webhooks ─────────────────────┘
```

## 2. Project Structure

### File Organization
```
hyperswitch_connectors/src/connectors/
├── maxpay/
│   └── transformers.rs    # Maxpay-specific request/response types and conversions
└── maxpay.rs             # Main connector implementation with trait implementations

crates/router/tests/connectors/
└── maxpay.rs             # Integration tests (moved from hyperswitch_connectors)
```

### Module Dependencies
```rust
// Core dependencies from hyperswitch
use hyperswitch_domain_models::{
    router_data::{ConnectorAuthType, RouterData},
    router_request_types::*,
    router_response_types::*,
    types::{PaymentsAuthorizeData, PaymentsCaptureData, RefundsData},
};
use common_utils::{
    crypto,
    ext_traits::ValueExt,
    request::RequestContent,
    types::{AmountConvertor, MinorUnit, MinorUnitForConnector},
};
use masking::{PeekInterface, Secret};
use error_stack::{Report, ResultExt};
```

## 3. Feature Specification

### 3.1 Payment Authorization
**User Story**: As a merchant, I want to authorize card payments through Maxpay to reserve funds on customer cards.

**Implementation Steps**:
1. Transform Hyperswitch `PaymentsAuthorizeData` to `MaxpayAuthRequest`
2. Set `transactionType` to "AUTH" or "AUTH3D" based on 3DS requirements
3. Include merchant credentials in request body
4. Convert amount from minor units to decimal format
5. Map card details preserving PCI compliance
6. Handle response and map status codes

**Request Structure**:
```rust
#[derive(Debug, Serialize)]
pub struct MaxpayAuthRequest {
    pub merchant_account: Secret<String>,
    pub merchant_password: Secret<String>,
    #[serde(rename = "transactionType")]
    pub transaction_type: MaxpayTransactionType,
    pub amount: f64,
    pub currency: String, // ISO 4217 alpha-3
    pub card_number: Secret<String>,
    pub card_expiry: Secret<String>, // MM/YYYY format
    pub card_cvv: Secret<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callback_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redirect_url: Option<String>,
}
```

**Response Handling**:
```rust
#[derive(Debug, Deserialize)]
pub struct MaxpayAuthResponse {
    #[serde(rename = "transactionId")]
    pub transaction_id: String,
    pub reference: String,
    pub status: MaxpayStatus,
    pub code: i32,
    #[serde(rename = "redirectUrl")]
    pub redirect_url: Option<String>,
}
```

**Error Handling**:
- Invalid credentials (code: 1001)
- Declined transaction (code: 3100 in test mode)
- 3DS required but URLs not provided
- Network timeouts

### 3.2 Payment Capture
**User Story**: As a merchant, I want to capture previously authorized payments to complete the transaction.

**Implementation Steps**:
1. Extract reference from previous authorization
2. Create capture request with `transactionType: "SETTLE"`
3. Send request to `/api/cc` endpoint
4. Map response status to Hyperswitch payment status

**Request Structure**:
```rust
#[derive(Debug, Serialize)]
pub struct MaxpayCaptureRequest {
    pub merchant_account: Secret<String>,
    pub merchant_password: Secret<String>,
    #[serde(rename = "transactionType")]
    pub transaction_type: MaxpayTransactionType, // "SETTLE"
    pub reference: String,
}
```

### 3.3 Payment Status Sync
**User Story**: As a merchant, I want to check the current status of a payment transaction.

**Implementation Steps**:
1. Use transaction reference from connector metadata
2. Set `transactionType: "CHECK"`
3. Parse response to determine current payment state
4. Update Hyperswitch payment status accordingly

**Request Structure**:
```rust
#[derive(Debug, Serialize)]
pub struct MaxpaySyncRequest {
    pub merchant_account: Secret<String>,
    pub merchant_password: Secret<String>,
    #[serde(rename = "transactionType")]
    pub transaction_type: MaxpayTransactionType, // "CHECK"
    pub reference: String,
}
```

### 3.4 Refund Processing
**User Story**: As a merchant, I want to refund payments to customers.

**Implementation Steps**:
1. Transform refund data to Maxpay refund request
2. Use `/api/refund` endpoint
3. Include original transaction reference
4. Handle full refund amount (partial refunds depend on acquirer)

**Request Structure**:
```rust
#[derive(Debug, Serialize)]
pub struct MaxpayRefundRequest {
    pub merchant_account: Secret<String>,
    pub merchant_password: Secret<String>,
    pub reference: String,
    pub amount: f64,
}
```

### 3.5 Card Tokenization
**User Story**: As a merchant, I want to tokenize cards for recurring payments.

**Implementation Steps**:
1. Set `transactionType: "TOKENIZE"`
2. Send card details to generate `billToken`
3. Store token mapping in Hyperswitch vault
4. Use token for subsequent payments

**Request Structure**:
```rust
#[derive(Debug, Serialize)]
pub struct MaxpayTokenizeRequest {
    pub merchant_account: Secret<String>,
    pub merchant_password: Secret<String>,
    #[serde(rename = "transactionType")]
    pub transaction_type: MaxpayTransactionType, // "TOKENIZE"
    pub card_number: Secret<String>,
    pub card_expiry: Secret<String>,
    pub card_cvv: Secret<String>,
}
```

**Response**:
```rust
#[derive(Debug, Deserialize)]
pub struct MaxpayTokenizeResponse {
    #[serde(rename = "billToken")]
    pub bill_token: String,
    pub status: MaxpayStatus,
    pub code: i32,
}
```

### 3.6 3D Secure Handling
**User Story**: As a merchant, I want to support 3D Secure authentication for enhanced security.

**Implementation Steps**:
1. Detect 3DS requirement based on merchant configuration
2. Use AUTH3D or SALE3D transaction types
3. Include HTTPS callback_url and redirect_url
4. Handle redirect flow for customer authentication
5. Process callback to complete transaction

**3DS Flow**:
```
Customer ──▶ Hyperswitch ──▶ Maxpay (AUTH3D) ──▶ 3DS Page
                │                                     │
                │◀──────── Callback ◀─────────────────┘
                │
                └──▶ Complete Transaction
```

## 4. Data Type Definitions

### 4.1 Core Enums
```rust
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum MaxpayTransactionType {
    Auth,
    Auth3d,
    Sale,
    Sale3d,
    Settle,
    Check,
    Tokenize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MaxpayStatus {
    Success,
    Decline,
    Error,
    #[serde(other)]
    Unknown,
}
```

### 4.2 Status Mapping
```rust
impl From<MaxpayStatus> for enums::AttemptStatus {
    fn from(status: MaxpayStatus) -> Self {
        match status {
            MaxpayStatus::Success => Self::Charged,
            MaxpayStatus::Decline => Self::Failure,
            MaxpayStatus::Error => Self::Failure,
            MaxpayStatus::Unknown => Self::Pending,
        }
    }
}
```

## 5. API Integration Details

### 5.1 HTTP Client Configuration
```rust
impl ConnectorCommon for Maxpay {
    fn id(&self) -> &'static str {
        "maxpay"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Base
    }

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.maxpay.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        _auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        Ok(vec![])  // Auth credentials in request body, not headers
    }
}
```

### 5.2 Request Building
```rust
impl ConnectorIntegration<api::Authorize, PaymentsAuthorizeData, PaymentsResponseData> for Maxpay {
    fn get_url(
        &self,
        _req: &RouterData<api::Authorize, PaymentsAuthorizeData, PaymentsResponseData>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}/api/cc", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &RouterData<api::Authorize, PaymentsAuthorizeData, PaymentsResponseData>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_auth = MaxpayAuth::try_from(&req.connector_auth_type)?;
        let amount = common_utils::types::get_amount_as_f64(
            &req.request.minor_amount,
            req.request.currency,
        )?;
        
        let maxpay_req = MaxpayAuthRequest {
            merchant_account: connector_auth.merchant_account,
            merchant_password: connector_auth.merchant_password,
            transaction_type: if req.request.is_three_ds() {
                MaxpayTransactionType::Auth3d
            } else {
                MaxpayTransactionType::Auth
            },
            amount,
            currency: req.request.currency.to_string(),
            // ... map other fields
        };

        Ok(RequestContent::Json(Box::new(maxpay_req)))
    }
}
```

### 5.3 Webhook Processing
**Webhook Setup**:
1. Configure callback URL in Maxpay dashboard
2. Ensure HTTPS with valid SSL certificate
3. Whitelist production URL

**Signature Verification**:
```rust
fn verify_webhook_signature(
    payload: &[u8],
    signature: &str,
    secret: &Secret<String>,
) -> Result<(), errors::ConnectorError> {
    let calculated_signature = crypto::Sha256
        .generate_hash(
            payload,
            secret.peek().as_bytes(),
        )?;
    
    if calculated_signature.as_bytes() == signature.as_bytes() {
        Ok(())
    } else {
        Err(errors::ConnectorError::WebhookSignatureValidationFailed)
    }
}
```

**Webhook Handlers**:
```rust
// Callback 1.0 (form-urlencoded)
#[derive(Debug, Deserialize)]
pub struct MaxpayWebhookV1 {
    #[serde(rename = "transactionId")]
    pub transaction_id: String,
    pub reference: String,
    pub status: MaxpayStatus,
    pub code: i32,
    #[serde(rename = "checkSum")]
    pub check_sum: String,
}

// Callback 2.0 (JSON)
#[derive(Debug, Deserialize)]
pub struct MaxpayWebhookV2 {
    #[serde(rename = "uniqueTransactionId")]
    pub unique_transaction_id: String,
    pub reference: String,
    pub status: MaxpayStatus,
    pub code: i32,
}
```

## 6. Currency and Amount Handling

### Currency Conversion
```rust
// Use existing common_utils for amount conversion
use common_utils::types::{MinorUnit, MinorUnitForConnector};

// Convert from minor units to decimal for Maxpay
let amount_in_base_unit = common_utils::types::get_amount_as_f64(
    &minor_unit,
    currency,
)?;

// Example: 1000 cents (MinorUnit) -> 10.00 (f64)
```

### Supported Currencies
- All ISO 4217 alpha-3 currency codes
- Common: USD, EUR, GBP, CAD, AUD, JPY
- Special handling for zero-decimal currencies (e.g., JPY)

## 7. Error Handling

### Error Code Mapping
```rust
#[derive(Debug)]
pub enum MaxpayErrorCode {
    InvalidCredentials = 1001,
    DeclinedTransaction = 3100,
    InsufficientFunds = 3101,
    InvalidCard = 3102,
    ExpiredCard = 3103,
    // ... other codes
}

impl From<i32> for MaxpayErrorCode {
    fn from(code: i32) -> Self {
        match code {
            1001 => Self::InvalidCredentials,
            3100 => Self::DeclinedTransaction,
            // ... other mappings
            _ => Self::UnknownError,
        }
    }
}

impl From<MaxpayErrorCode> for errors::ConnectorError {
    fn from(code: MaxpayErrorCode) -> Self {
        match code {
            MaxpayErrorCode::InvalidCredentials => {
                errors::ConnectorError::InvalidConnectorConfig { 
                    config: "Invalid merchant credentials".to_string() 
                }
            },
            MaxpayErrorCode::DeclinedTransaction => {
                errors::ConnectorError::ProcessingError { 
                    code: "3100".to_string(),
                    reason: "Transaction declined by bank".to_string()
                }
            },
            // ... other mappings
        }
    }
}
```

### Retry Strategy
- Network errors: Retry with exponential backoff
- Authentication errors: No retry
- Processing errors: No retry
- Timeout: Sync status before retry

## 8. Testing Strategy

### 8.1 Unit Tests
Location: `hyperswitch_connectors/src/connectors/maxpay/transformers.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_request_serialization() {
        let auth_req = MaxpayAuthRequest {
            merchant_account: Secret::new("test_account".to_string()),
            merchant_password: Secret::new("test_password".to_string()),
            transaction_type: MaxpayTransactionType::Auth,
            amount: 10.50,
            currency: "USD".to_string(),
            // ... other fields
        };

        let json = serde_json::to_string(&auth_req).unwrap();
        assert!(json.contains(r#""transactionType":"AUTH""#));
        assert!(json.contains(r#""amount":10.5"#));
    }

    #[test]
    fn test_status_mapping() {
        assert_eq!(
            enums::AttemptStatus::from(MaxpayStatus::Success),
            enums::AttemptStatus::Charged
        );
        assert_eq!(
            enums::AttemptStatus::from(MaxpayStatus::Decline),
            enums::AttemptStatus::Failure
        );
    }

    #[test]
    fn test_currency_conversion() {
        let minor_unit = MinorUnit::new(1050);
        let amount = common_utils::types::get_amount_as_f64(
            &minor_unit,
            enums::Currency::USD,
        ).unwrap();
        assert_eq!(amount, 10.50);
    }
}
```

### 8.2 Integration Tests
Location: `crates/router/tests/connectors/maxpay.rs`

```rust
use router::connector::maxpay;
use router::types::{self, api, storage::enums};
use test_utils::connector_auth;

#[test]
fn test_maxpay_authorization() {
    let connector = maxpay::Maxpay;
    let auth = connector_auth::ConnectorAuthentication::new()
        .maxpay
        .unwrap();

    let authorize_data = types::PaymentsAuthorizeData {
        amount: 1000,
        currency: enums::Currency::USD,
        payment_method_data: types::domain::PaymentMethodData::Card(
            types::domain::Card {
                card_number: cards::CardNumber::from("4111111111111111"),
                card_exp_month: Secret::new("12".to_string()),
                card_exp_year: Secret::new("2025".to_string()),
                card_cvc: Secret::new("123".to_string()),
                // ... other fields
            }
        ),
        // ... other fields
    };

    let response = connector.authorize_payment(authorize_data, auth);
    assert!(response.is_ok());
}
```

### 8.3 Test Cards
```rust
// Test card constants
pub const TEST_CARDS: &[(&str, &str)] = &[
    ("4111111111111111", "Visa 2D - Success"),
    ("5555555555554444", "Mastercard - Success"),
    ("378282246310005", "Amex - Success"),
    ("4000000000000002", "Visa - Decline"),
];

// Test mode handling
if is_test_mode {
    // Add "+" to phone number to avoid test declines
    if let Some(phone) = &mut request.user_phone {
        *phone = format!("+{}", phone);
    }
}
```

## 9. Configuration

### 9.1 Connector Configuration
In `config/development.toml`:
```toml
[connectors.maxpay]
base_url = "https://sbx.maxpay.com"  # Test environment
secondary_base_url = "https://gateway.maxpay.com"  # Live environment

[connectors.supported.maxpay]
payment_method_type = ["credit", "debit"]
credit = ["visa", "mastercard", "american_express"]
debit = ["visa", "mastercard"]
```

### 9.2 Merchant Configuration
```rust
#[derive(Debug, Deserialize)]
pub struct MaxpayMerchantConfig {
    pub merchant_account: Secret<String>,
    pub merchant_password: Secret<String>,
    pub enable_3ds: bool,
    pub callback_version: MaxpayCallbackVersion,  // V1 or V2
}

#[derive(Debug, Deserialize)]
pub enum MaxpayCallbackVersion {
    V1,  // form-urlencoded
    V2,  // JSON
}
```

## 10. Implementation Checklist

### Phase 1: Basic Integration
- [ ] Implement connector boilerplate using `add_connector.sh`
- [ ] Define request/response types in `transformers.rs`
- [ ] Implement `ConnectorCommon` trait
- [ ] Implement authorization flow
- [ ] Add unit tests for transformers

### Phase 2: Core Features
- [ ] Implement capture flow
- [ ] Implement sync/status check
- [ ] Implement refund flow
- [ ] Add error handling and mapping
- [ ] Add integration tests

### Phase 3: Advanced Features
- [ ] Implement 3D Secure flows
- [ ] Implement tokenization
- [ ] Add webhook support (v1.0 and v2.0)
- [ ] Implement webhook signature verification
- [ ] Add comprehensive test coverage

### Phase 4: Production Readiness
- [ ] Complete error code mapping
- [ ] Add retry logic
- [ ] Performance optimization
- [ ] Documentation updates
- [ ] Security review
- [ ] Move test file to correct location

## 11. Security Considerations

### PCI Compliance
- Never log card details
- Use `Secret<T>` wrapper for sensitive data
- Implement proper masking for logs
- Ensure TLS 1.2+ for all API calls

### Credential Management
- Store credentials encrypted
- Rotate passwords regularly
- Use separate credentials for test/live
- Implement credential validation

### Webhook Security
- Verify signatures on all callbacks
- Use HTTPS only for callback URLs
- Implement replay attack protection
- Return proper HTTP status codes

## 12. Monitoring and Observability

### Logging
```rust
use router_env::logger;

logger::info!("Maxpay payment authorization initiated");
logger::error!("Maxpay API error: {:?}", error);
```

### Metrics
- Track API response times
- Monitor success/failure rates
- Alert on high error rates
- Track 3DS conversion rates

### Health Checks
- Implement connector health endpoint
- Monitor API availability
- Track webhook delivery success
