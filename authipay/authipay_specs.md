# Authipay Connector Integration Technical Specification

## 1. Connector Overview

### 1.1 Basic Information
- **Connector Name**: authipay
- **Base URL**: 
  - Sandbox: https://prod.emea.api.fiservapps.com/sandbox/ipp/payments-gateway/v2
  - Production: https://prod.emea.api.fiservapps.com/ipp/payments-gateway/v2
- **API Documentation**: [Fiserv Developer Portal](https://docs.fiserv.dev/public/reference/)
- **Supported Countries**: US, EU countries (full list available through `/available-iso-countries` endpoint)
- **Supported Currencies**: EUR, GBP, USD, etc. (full list available through `/available-currencies` endpoint)

### 1.2 Authentication Method
- **Type**: API Key with HMAC-SHA256 signature
- **Headers**:
  - `Api-Key`: Merchant-specific key from Fiserv developer portal
  - `Client-Request-Id`: Unique UUID for request tracking and idempotency
  - `Timestamp`: Epoch timestamp in milliseconds
  - `Message-Signature`: Base64-encoded HMAC-SHA256 hash of the request

### 1.3 Supported Features
| Feature | Supported | Notes |
|---------|-----------|-------|
| Card Payments | ✓ | Supports Visa, Mastercard, Maestro, Amex, etc. |
| Bank Transfers | ✗ | Not mentioned in documentation |
| Wallets | ✗ | Not mentioned in documentation |
| 3DS 2.0 | ✓ | Supports 3DS 1.0 and 2.1 via redirectAttributes |
| Recurring Payments | ✓ | Supports tokenization for recurring payments |
| Partial Capture | ✓ | Supported via PostAuthTransaction |
| Multiple Captures | ✓ | Supported via PostAuthTransaction |
| Instant Refunds | ✓ | Supported via ReturnTransaction |
| Partial Refunds | ✓ | Supported via ReturnTransaction |
| Webhooks | ✗ | Not mentioned in documentation |

## 2. API Endpoints

### 2.1 Payment Operations
| Operation | Method | Endpoint | Purpose |
|-----------|---------|----------|---------|
| Create Payment | POST | /payments | Create payment intent (sale or pre-auth) |
| Capture Payment | POST | /payments/{transaction-id} | Capture authorized payment |
| Cancel Payment | POST | /payments/{transaction-id} | Void/Cancel payment |
| Get Payment | GET | /payments/{transaction-id} | Retrieve payment status |

### 2.2 Refund Operations
| Operation | Method | Endpoint | Purpose |
|-----------|---------|----------|---------|
| Create Refund | POST | /payments/{transaction-id} | Initiate refund with ReturnTransaction |
| Create Order Refund | POST | /orders/{order-id} | Initiate refund for an order |
| Get Refund | GET | /payments/{transaction-id} | Check refund status |

### 2.3 Other Operations
| Operation | Method | Endpoint | Purpose |
|-----------|---------|----------|---------|
| Tokenization | POST | /payment-tokens | Create token for recurring payments |
| Card Verification | POST | /card-verification | Verify card without processing payment |

## 3. Data Models

### 3.1 Payment Request Structure
```json
{
  "requestType": "PaymentCardSaleTransaction",
  "transactionAmount": {
    "total": 12.04,
    "currency": "EUR"
  },
  "paymentMethod": {
    "paymentCard": {
      "number": "5424180279791732",
      "securityCode": "977",
      "expiryDate": {
        "month": "12",
        "year": "24"
      }
    }
  },
  "merchantTransactionId": "lsk23532djljff3",
  "storeId": "12345500000",
  "storedCredentials": {
    "sequence": "FIRST",
    "scheduled": true
  }
}
```

### 3.2 Payment Response Structure
```json
{
  "clientRequestId": "30dd879c-ee2f-11db-8314-0800200c9a66",
  "apiTraceId": "rrt-0bd552c12342d3448-b-ea-1142-12938318-7",
  "ipgTransactionId": "123978432",
  "orderId": "123456",
  "transactionTime": 1554308829345,
  "transactionState": "CAPTURED",
  "paymentType": "CREDIT_CARD",
  "transactionOrigin": "ECOM",
  "amount": {
    "total": 12.04,
    "currency": "EUR"
  },
  "storeId": "12345500000"
}
```

### 3.3 Status Mappings
| Connector Status | Hyperswitch Status | Description |
|------------------|-------------------|-------------|
| PENDING | Processing | Payment is being processed |
| CAPTURED | Charged | Payment captured successfully |
| AUTHORIZED | Authorized | Payment authorized but not yet captured |
| DECLINED | Failed | Payment declined |
| VERIFIED | Requires_Confirmation | Card verification successful |
| VOIDED | Cancelled | Payment voided successfully |
| RETURNED | Refunded | Payment refunded successfully |

### 3.4 Error Code Mappings
| HTTP Status | Connector Error | Hyperswitch Error | Description |
|-------------|----------------|------------------|-------------|
| 400 | Bad Request | InvalidRequestError | Invalid payload |
| 401 | Unauthenticated | AuthenticationFailed | Invalid Api-Key |
| 403 | Unauthorized | AuthorizationFailed | Insufficient permissions |
| 404 | Not Found | ResourceNotFound | Invalid transaction-id or order-id |
| 409 | Transaction Gateway Declined | GatewayError | Gateway declined the transaction |
| 422 | Transaction Endpoint Declined | ProcessorError | Endpoint declined the transaction |
| 500 | Server Error | ServerError | Server-side error |
| 502 | Endpoint Communication Error | ConnectorError | Communication error with endpoint |

## 4. Implementation Details

### 4.1 Request Transformations

#### 4.1.1 Authorize Request
```rust
// Transform Hyperswitch PaymentsAuthorizeData to AuthipayPaymentRequest
impl TryFrom<&PaymentsAuthorizeRouterData> for AuthipayPaymentRequest {
    type Error = Error;
    
    fn try_from(req: &PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        let amount = req.request.amount;
        let currency = req.request.currency.to_string();
        
        // Determine if it's a sale or pre-auth
        let request_type = if req.request.capture_method == Some(enums::CaptureMethod::Automatic) {
            "PaymentCardSaleTransaction"
        } else {
            "PaymentCardPreAuthTransaction"
        };
        
        // Extract card details
        let payment_method = match &req.request.payment_method_data {
            PaymentMethodData::Card(card) => {
                AuthipayPaymentMethod {
                    payment_card: AuthipayCard {
                        number: card.card_number.clone(),
                        security_code: card.card_cvc.clone(),
                        expiry_date: AuthipayExpiryDate {
                            month: card.card_exp_month.clone(),
                            year: card.card_exp_year.clone().chars().skip(2).collect(), // Convert YYYY to YY
                        },
                    },
                }
            },
            _ => return Err(errors::ConnectorError::NotImplemented("Payment method not supported".to_string()).into()),
        };
        
        Ok(AuthipayPaymentRequest {
            request_type: request_type.to_string(),
            transaction_amount: AuthipayAmount {
                total: amount,
                currency,
            },
            payment_method,
            merchant_transaction_id: Some(req.payment_id.clone()),
            store_id: req.connector_auth_type.store_id.clone(),
            stored_credentials: None, // Optional field
        })
    }
}
```

#### 4.1.2 Capture Request
```rust
// Transform Hyperswitch PaymentsCaptureData to AuthipayCaptureRequest
impl TryFrom<&PaymentsCaptureRouterData> for AuthipayCaptureRequest {
    type Error = Error;
    
    fn try_from(req: &PaymentsCaptureRouterData) -> Result<Self, Self::Error> {
        Ok(AuthipayCaptureRequest {
            request_type: "PostAuthTransaction".to_string(),
            transaction_amount: AuthipayAmount {
                total: req.request.amount_to_capture.unwrap_or(req.request.amount),
                currency: req.request.currency.to_string(),
            },
            store_id: req.connector_auth_type.store_id.clone(),
        })
    }
}
```

#### 4.1.3 Refund Request
```rust
// Transform Hyperswitch RefundsRouterData to AuthipayRefundRequest
impl TryFrom<&RefundsRouterData> for AuthipayRefundRequest {
    type Error = Error;
    
    fn try_from(req: &RefundsRouterData) -> Result<Self, Self::Error> {
        Ok(AuthipayRefundRequest {
            request_type: "ReturnTransaction".to_string(),
            transaction_amount: AuthipayAmount {
                total: req.request.refund_amount,
                currency: req.request.currency.to_string(),
            },
            store_id: req.connector_auth_type.store_id.clone(),
        })
    }
}
```

#### 4.1.4 Cancel Request
```rust
// Transform Hyperswitch PaymentsCancelData to AuthipayCancelRequest
impl TryFrom<&PaymentsCancelRouterData> for AuthipayCancelRequest {
    type Error = Error;
    
    fn try_from(req: &PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        Ok(AuthipayCancelRequest {
            request_type: "VoidTransaction".to_string(),
            store_id: req.connector_auth_type.store_id.clone(),
        })
    }
}
```

#### 4.1.5 Tokenization Request
```rust
// Transform Hyperswitch TokenizationRouterData to AuthipayTokenRequest
impl TryFrom<&TokenizationRouterData> for AuthipayTokenRequest {
    type Error = Error;
    
    fn try_from(req: &TokenizationRouterData) -> Result<Self, Self::Error> {
        // Extract card details
        let card = match &req.request.payment_method_data {
            PaymentMethodData::Card(card) => card,
            _ => return Err(errors::ConnectorError::NotImplemented("Payment method not supported".to_string()).into()),
        };
        
        Ok(AuthipayTokenRequest {
            request_type: "PaymentCardPaymentTokenizationRequest".to_string(),
            payment_card: AuthipayCard {
                number: card.card_number.clone(),
                security_code: card.card_cvc.clone(),
                expiry_date: AuthipayExpiryDate {
                    month: card.card_exp_month.clone(),
                    year: card.card_exp_year.clone().chars().skip(2).collect(), // Convert YYYY to YY
                },
            },
            store_id: req.connector_auth_type.store_id.clone(),
        })
    }
}
```

### 4.2 Response Transformations

#### 4.2.1 Authorize Response
```rust
// Transform AuthipayPaymentResponse to Hyperswitch PaymentsResponseData
impl TryFrom<ResponseRouterData<PaymentsAuthorizeRouterData, AuthipayPaymentResponse>> for PaymentsAuthorizeRouterData {
    type Error = Error;
    
    fn try_from(res: ResponseRouterData<PaymentsAuthorizeRouterData, AuthipayPaymentResponse>) -> Result<Self, Self::Error> {
        let status = match res.response.transaction_state.as_str() {
            "AUTHORIZED" => enums::AttemptStatus::Authorized,
            "CAPTURED" => enums::AttemptStatus::Charged,
            "DECLINED" => enums::AttemptStatus::Failure,
            "VOIDED" => enums::AttemptStatus::Voided,
            "PENDING" => enums::AttemptStatus::Pending,
            _ => enums::AttemptStatus::Pending,
        };
        
        Ok(PaymentsAuthorizeRouterData {
            status,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: res.response.ipg_transaction_id,
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(res.response.order_id),
                incremental_authorization_allowed: None,
            }),
            ..res.data
        })
    }
}
```

#### 4.2.2 Capture Response
```rust
// Transform AuthipayCaptureResponse to Hyperswitch PaymentsCaptureData
impl TryFrom<ResponseRouterData<PaymentsCaptureRouterData, AuthipayCaptureResponse>> for PaymentsCaptureRouterData {
    type Error = Error;
    
    fn try_from(res: ResponseRouterData<PaymentsCaptureRouterData, AuthipayCaptureResponse>) -> Result<Self, Self::Error> {
        let status = match res.response.transaction_state.as_str() {
            "CAPTURED" => enums::AttemptStatus::Charged,
            _ => enums::AttemptStatus::Pending,
        };
        
        Ok(PaymentsCaptureRouterData {
            status,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: res.response.ipg_transaction_id,
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(res.response.order_id),
                incremental_authorization_allowed: None,
            }),
            ..res.data
        })
    }
}
```

#### 4.2.3 Refund Response
```rust
// Transform AuthipayRefundResponse to Hyperswitch RefundsResponseData
impl TryFrom<ResponseRouterData<RefundsRouterData, AuthipayRefundResponse>> for RefundsRouterData {
    type Error = Error;
    
    fn try_from(res: ResponseRouterData<RefundsRouterData, AuthipayRefundResponse>) -> Result<Self, Self::Error> {
        let refund_status = match res.response.transaction_state.as_str() {
            "RETURNED" => enums::RefundStatus::Success,
            _ => enums::RefundStatus::Pending,
        };
        
        Ok(RefundsRouterData {
            response: Ok(RefundsResponseData::RefundResponse {
                connector_refund_id: res.response.ipg_transaction_id,
                refund_status,
            }),
            ..res.data
        })
    }
}
```

#### 4.2.4 Cancel Response
```rust
// Transform AuthipayCancelResponse to Hyperswitch PaymentsCancelData
impl TryFrom<ResponseRouterData<PaymentsCancelRouterData, AuthipayCancelResponse>> for PaymentsCancelRouterData {
    type Error = Error;
    
    fn try_from(res: ResponseRouterData<PaymentsCancelRouterData, AuthipayCancelResponse>) -> Result<Self, Self::Error> {
        let status = match res.response.transaction_state.as_str() {
            "VOIDED" => enums::AttemptStatus::Voided,
            _ => enums::AttemptStatus::Pending,
        };
        
        Ok(PaymentsCancelRouterData {
            status,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: res.response.ipg_transaction_id,
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(res.response.order_id),
                incremental_authorization_allowed: None,
            }),
            ..res.data
        })
    }
}
```

#### 4.2.5 Tokenization Response
```rust
// Transform AuthipayTokenResponse to Hyperswitch TokenizationRouterData
impl TryFrom<ResponseRouterData<TokenizationRouterData, AuthipayTokenResponse>> for TokenizationRouterData {
    type Error = Error;
    
    fn try_from(res: ResponseRouterData<TokenizationRouterData, AuthipayTokenResponse>) -> Result<Self, Self::Error> {
        let token = res.response.payment_token.value;
        let card_details = Some(PaymentMethodTokenData::Card(CardTokenData {
            last4: res.response.payment_token.card_last4,
            exp_month: res.response.payment_token.expiry_month,
            exp_year: res.response.payment_token.expiry_year,
            card_type: res.response.payment_token.brand.to_lowercase(),
            card_holder_name: None,
        }));
        
        Ok(TokenizationRouterData {
            response: Ok(PaymentMethodToken {
                token,
                card_details,
            }),
            ..res.data
        })
    }
}
```

### 4.3 Amount Handling
- **Format**: Authipay uses major units (e.g., 12.04 EUR)
- **Decimal Places**: 2 for most currencies
- **Conversion**: Hyperswitch uses minor units (e.g., 1204 cents) which need to be converted to major units for Authipay

### 4.4 Payment Method Transformations

#### 4.4.1 Card
```rust
// Card payment method transformation
fn transform_card_data(card: &CardData) -> AuthipayCard {
    AuthipayCard {
        number: card.card_number.clone(),
        security_code: card.card_cvc.clone(),
        expiry_date: AuthipayExpiryDate {
            month: card.card_exp_month.clone(),
            year: card.card_exp_year.clone().chars().skip(2).collect(), // Convert YYYY to YY
        },
    }
}
```

#### 4.4.2 3DS Authentication
```rust
// 3DS payment method transformation
fn transform_3ds_data(card: &CardData, browser_info: Option<&BrowserInformation>) -> (AuthipayCard, AuthipayRedirectAttributes) {
    let card_data = transform_card_data(card);
    
    // Extract browser information
    let browser_js_enabled = browser_info.map(|b| b.java_enabled).unwrap_or(true);
    let browser_java_enabled = browser_info.map(|b| b.java_script_enabled).unwrap_or(true);
    
    let redirect_attributes = AuthipayRedirectAttributes {
        authenticate_transaction: true,
        challenge_indicator: "01".to_string(), // No preference
        browser_javascript_enabled: browser_js_enabled,
        browser_java_enabled: browser_java_enabled,
        three_ds_emv_co_message_category: "01".to_string(),
    };
    
    (card_data, redirect_attributes)
}
```

## 5. Error Handling

### 5.1 API Error Response Format
```json
{
  "clientRequestId": "30dd879c-ee2f-11db-8314-0800200c9a66",
  "apiTraceId": "rrt-0bd552c12342d3448-b-ea-1142-12938318-7",
  "error": {
    "code": "invalid_request",
    "message": "Invalid card number",
    "details": [
      {
        "location": "body",
        "message": "Card number is not valid",
        "locationType": "body"
      }
    ]
  }
}
```

### 5.2 Error Handling Strategy
```rust
// Error response transformation
impl ConnectorCommon for Authipay {
    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: AuthipayErrorResponse = res
            .response
            .parse_struct("AuthipayErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response
                .error
                .code
                .unwrap_or_else(|| "".to_string()),
            message: response
                .error
                .message
                .unwrap_or_else(|| "".to_string()),
            reason: response.error.details.map(|details| {
                details
                    .iter()
                    .map(|detail| detail.message.clone())
                    .collect::<Vec<String>>()
                    .join(", ")
            }),
            attempt_status: None,
            connector_transaction_id: None,
        })
    }
}
```

## 6. Authentication Implementation

### 6.1 Auth Type Definition
```rust
// Define the authentication type for Authipay
#[derive(Debug, Clone)]
pub struct AuthipayAuthType {
    pub api_key: Secret<String>,
    pub api_secret: Secret<String>,
    pub store_id: String,
}

impl TryFrom<&ConnectorAuthType> for AuthipayAuthType {
    type Error = Error;
    
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        if let ConnectorAuthType::BodyKey { api_key, key1, api_secret } = auth_type {
            Ok(Self {
                api_key: api_key.to_owned(),
                api_secret: api_secret.to_owned(),
                store_id: key1.to_owned(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType.into())
        }
    }
}
```

### 6.2 HMAC Signature Generation
```rust
// Generate the HMAC-SHA256 signature for Authipay
fn generate_signature(
    payload: &[u8],
    api_secret: &str,
) -> CustomResult<String, errors::ConnectorError> {
    let key = hmac::Key::new(hmac::HMAC_SHA256, api_secret.as_bytes());
    let signature = hmac::sign(&key, payload);
    let b64_signature = BASE64_ENGINE.encode(signature.as_ref());
    Ok(b64_signature)
}
```

### 6.3 Header Construction
```rust
// Build headers for Authipay requests
fn build_headers(
    &self,
    req: &RouterData<Flow, Request, Response>,
    _connectors: &Connectors,
) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
    let auth = AuthipayAuthType::try_from(&req.connector_auth_type)?;
    
    // Generate timestamp
    let timestamp = chrono::Utc::now().timestamp_millis().to_string();
    
    // Generate client request ID
    let client_request_id = Uuid::new_v4().to_string();
    
    // Create payload for signature
    let payload = req.request.to_string();
    
    // Generate signature
    let signature = generate_signature(payload.as_bytes(), auth.api_secret.peek())?;
    
    // Construct headers
    let mut headers = vec![
        (
            headers::CONTENT_TYPE.to_string(),
            self.common_get_content_type().to_string().into(),
        ),
        (
            "Api-Key".to_string(),
            auth.api_key.peek().to_string().into_masked(),
        ),
        (
            "Client-Request-Id".to_string(),
            client_request_id.into_masked(),
        ),
        (
            "Timestamp".to_string(),
            timestamp.into_masked(),
        ),
        (
            "Message-Signature".to_string(),
            signature.into_masked(),
        ),
    ];
    
    Ok(headers)
}
```

## 7. Testing Strategy

### 7.1 Test Credentials
- **Test API Key**: Obtain from Fiserv Developer Portal
- **Test API Secret**: Obtain from Fiserv Developer Portal
- **Test Store ID**: Obtain from Fiserv Developer Portal
- **Test Card Numbers**:
  - Visa: 4111111111111111 (success)
  - Mastercard: 5424180279791732 (success)
  - Amex: 371449635398431 (success)

### 7.2 Test Scenarios
1. **Successful Payment Flow**
   - Authorize → Capture
   - Direct Sale (Authorize with auto-capture)
   - Tokenize → Payment with Token

2. **Failed Payment Scenarios**
   - Insufficient funds
   - Invalid card number
   - Expired card

3. **Refund Scenarios**
   - Full refund
   - Partial refund

4. **Cancel/Void Scenarios**
   - Cancel before settlement

5. **3DS Scenarios**
   - 3DS authentication required
   - 3DS authentication successful
   - 3DS authentication failed

### 7.3 Integration Test Structure
```rust
#[serial_test::serial]
#[actix_web::test]
async fn should_only_authorize_payment() {
    let response = CONNECTOR
        .authorize_payment(payment_method_details(), get_default_payment_info())
        .await
        .expect("Authorize payment response");
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
}

#[serial_test::serial]
#[actix_web::test]
async fn should_authorize_and_capture_payment() {
    let response = CONNECTOR
        .authorize_and_capture_payment(payment_method_details(), get_default_payment_info())
        .await
        .expect("Authorize and capture payment response");
    assert_eq!(response.status, enums::AttemptStatus::Charged);
}

#[serial_test::serial]
#[actix_web::test]
async fn should_authorize_and_capture_and_refund_payment() {
    let response = CONNECTOR
        .authorize_capture_and_refund_payment(payment_method_details(), get_default_payment_info(), None)
        .await
        .expect("Authorize capture and refund payment response");
    assert_eq!(response.status, enums::RefundStatus::Success);
}

#[serial_test::serial]
#[actix_web::test]
async fn should_tokenize_card() {
    let card = payment_method_details();
    let response = CONNECTOR
        .tokenize(&card, get_default_payment_info())
        .await
        .expect("Card tokenization response");
    assert!(response.token.is_some());
}
```

## 8. Connector-Specific Considerations

### 8.1 Quirks and Limitations
- The API uses major currency units, while Hyperswitch uses minor units.
- Webhooks are not mentioned in the documentation, so polling may be necessary for status updates.
- Authentication requires generating a HMAC-SHA256 signature for each request.

### 8.2 Best Practices
- Always use a unique Client-Request-Id for each request to ensure idempotency.
- Use the appropriate requestType for the intended operation (e.g., PaymentCardSaleTransaction for direct payment).
- Include the storeId in all requests.

### 8.3 Implementation Checklist

#### 8.3.1 Core Implementation
- [ ] Complete connector auth implementation (API Key and HMAC signature)
- [ ] Implement ConnectorCommon trait methods
- [ ] Implement PaymentAuthorize trait methods
- [ ] Implement PaymentCapture trait methods
- [ ] Implement PaymentSync trait methods
- [ ] Implement RefundExecute trait methods
- [ ] Implement RefundSync trait methods
- [ ] Implement PaymentCancel trait methods
- [ ] Implement Tokenize trait methods
- [ ] Add error handling

#### 8.3.2 Additional Features
- [ ] Implement 3DS handling
- [ ] Implement card verification (preprocessing)

#### 8.3.3 Testing & Documentation
- [ ] Unit tests for transformers
- [ ] Integration tests for all flows
- [ ] Error scenario tests
- [ ] Update connector documentation

## 9. References

### 9.1 External Documentation
- [Fiserv Developer Portal](https://docs.fiserv.dev/public/reference/)

### 9.2 Internal References
- Similar connector implementations: Stripe, Adyen
- Reusable patterns: Common auth handling, error mapping
