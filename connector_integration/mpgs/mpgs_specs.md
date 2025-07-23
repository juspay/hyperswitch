# Mpgs Connector Integration Technical Specification

## 1. Connector Overview

### 1.1 Basic Information
- **Connector Name**: Mpgs
- **Base URL**: `https://ap-gateway.mastercard.com`
- **API Documentation**: [https://ap-gateway.mastercard.com/api/documentation/apiDocumentation.html](https://ap-gateway.mastercard.com/api/documentation/apiDocumentation.html)
- **Supported Countries**: Global
- **Supported Currencies**: All major currencies

### 1.2 Authentication Method
- **Type**: HTTP Basic Auth
- **Header Format**: `Authorization: Basic {base64_encoded_credentials}`
- **Credentials Format**: `merchant.{merchant_id}:{password}`

### 1.3 Supported Features
| Feature | Supported | Notes |
|---|---|---|
| Card Payments | ✓ | |
| Bank Transfers | ✗ | |
| Wallets | ✗ | |
| 3DS 2.0 | ✓ | Requires additional fields in the request |
| Recurring Payments | ✗ | |
| Partial Capture | ✓ | |
| Multiple Captures | ✗ | |
| Instant Refunds | ✓ | |
| Partial Refunds | ✓ | |
| Webhooks | ✗ | |

## 2. API Endpoints

### 2.1 Payment Operations
| Operation | Method | Endpoint | Purpose |
|---|---|---|---|
| Authorize/Pay | PUT | `/api/rest/version/100/merchant/{merchant_id}/order/{order_id}/transaction/{transaction_id}` | Create a payment or authorization |
| Capture | PUT | `/api/rest/version/100/merchant/{merchant_id}/order/{order_id}/transaction/{transaction_id}` | Capture an authorized payment |
| Void | PUT | `/api/rest/version/100/merchant/{merchant_id}/order/{order_id}/transaction/{transaction_id}` | Void an authorized payment |
| Get Payment | GET | `/api/rest/version/100/merchant/{merchant_id}/order/{order_id}` | Retrieve payment status |
| Get Transaction | GET | `/api/rest/version/100/merchant/{merchant_id}/order/{order_id}/transaction/{transaction_id}` | Retrieve transaction status |

### 2.2 Refund Operations
| Operation | Method | Endpoint | Purpose |
|---|---|---|---|
| Create Refund | PUT | `/api/rest/version/100/merchant/{merchant_id}/order/{order_id}/transaction/{transaction_id}` | Initiate a refund |

## 3. Data Models

### 3.1 Payment Request Structure
```json
{
  "apiOperation": "PAY",
  "order": {
    "amount": "100.00",
    "currency": "USD"
  },
  "sourceOfFunds": {
    "type": "CARD",
    "provided": {
      "card": {
        "number": "...",
        "expiry": {
          "month": "...",
          "year": "..."
        },
        "securityCode": "..."
      }
    }
  },
  "transaction": {
    "reference": "..."
  }
}
```

### 3.2 Payment Response Structure
```json
{
  "result": "SUCCESS",
  "transaction": {
    "id": "...",
    "type": "PAYMENT",
    "result": "SUCCESS"
  },
  "order": {
    "amount": 100.00,
    "currency": "USD"
  },
  "response": {
    "gatewayCode": "APPROVED"
  }
}
```

### 3.3 Status Mappings
| Connector Status (`result`, `response.gatewayCode`, `transaction.type`) | Hyperswitch Status | Description |
|---|---|---|
| `SUCCESS`, `APPROVED`, `AUTHORIZATION` | `Authorized` | Payment authorized successfully |
| `SUCCESS`, `APPROVED`, `PAYMENT` / `CAPTURE` | `Charged` | Payment captured successfully |
| `SUCCESS`, `APPROVED`, `VOID` | `Voided` | Payment voided successfully |
| `PENDING`, _, _ | `Pending` | Payment is pending |
| _, `AUTHENTICATION_REQUIRED`, _ | `AuthenticationPending` | 3DS authentication required |
| `FAILURE`, _, _ | `Failure` | Payment failed |

### 3.4 Error Code Mappings
| Connector Error Code (`error.cause`) | Hyperswitch Error | Description |
|---|---|---|
| `INVALID_REQUEST` | `InvalidRequest` | The request was invalid |
| `AUTHENTICATION_FAILED` | `AuthenticationFailed` | Authentication failed |
| `INSUFFICIENT_FUNDS` | `InsufficientFunds` | Card has insufficient funds |

## 4. Implementation Details

### 4.1 Request Transformations

#### 4.1.1 Authorize/Pay Request
```rust
// Pseudo-code showing transformation logic
MpgsPaymentsRequest {
    api_operation: if req.request.capture_method == Some(CaptureMethod::Automatic) { MpgsApiOperation::Pay } else { MpgsApiOperation::Authorize },
    order: MpgsOrder {
        amount: req.amount.to_string(), // Convert to major unit string
        currency: req.request.currency.to_string(),
    },
    source_of_funds: MpgsSourceOfFunds {
        r#type: "CARD".to_string(),
        provided: Some(MpgsProvidedSourceOfFunds {
            card: MpgsCard {
                number: req.request.payment_method_data.card.card_number.clone(),
                expiry: MpgsExpiry {
                    month: req.request.payment_method_data.card.card_exp_month.clone(),
                    year: req.request.payment_method_data.card.card_exp_year.clone(),
                },
                security_code: Some(req.request.payment_method_data.card.card_cvc.clone()),
            },
        }),
    },
    transaction: MpgsTransaction {
        reference: req.payment_id.clone(),
    },
    customer: None,
}
```

### 4.2 Response Transformations
- Status is determined by a combination of `result`, `response.gatewayCode`, and `transaction.type`.
- The connector transaction ID is in `transaction.id`.

### 4.3 Amount Handling
- **Format**: Major units as a string with 2 decimal places.
- **Conversion**: Use `utils::to_currency_base_unit_as_string`

### 4.4 Payment Method Transformations
- Only `Card` is supported. The transformation is straightforward as shown in the request transformation pseudo-code.

## 5. Webhook Implementation
- Not supported by the connector.

## 6. Error Handling

### 6.1 API Error Response Format
```json
{
  "error": {
    "cause": "INVALID_REQUEST",
    "explanation": "The request was invalid."
  },
  "result": "ERROR"
}
```

### 6.2 Error Handling Strategy
- The `build_error_response` function will parse the nested `error` object.
- The `error.cause` will be mapped to the Hyperswitch error code.
- The `error.explanation` will be used as the error message.

## 7. Testing Strategy

### 7.1 Test Credentials
- Test credentials will be provided in `sample_auth.toml`.

### 7.2 Test Scenarios
1. **Successful Payment Flow**
   - Authorize -> Capture
   - Direct Pay (auto-capture)
2. **Failed Payment Scenarios**
   - Insufficient funds
   - Invalid card
3. **Refund Scenarios**
   - Full refund
   - Partial refund
4. **Void Scenario**
   - Void an authorized payment

### 7.3 Integration Test Structure
- The standard 20 sanity tests generated by the script will be implemented.

## 8. Connector-Specific Considerations

### 8.1 Quirks and Limitations
- Each API call requires a unique transaction ID in the URL.
- The `apiOperation` field in the request body determines the payment flow.
- The authentication key has a special format: `merchant.{merchant_id}:{password}`.
- The status of a transaction depends on three different fields in the response.

## 9. Implementation Checklist

### 9.1 Core Implementation
- [ ] Complete connector auth implementation
- [ ] Implement all trait methods (Payment, Refund, etc.)
- [ ] Complete request transformers
- [ ] Complete response transformers
- [ ] Implement error handling
- [ ] Add amount conversion logic

### 9.2 Additional Features
- [ ] 3DS handling

### 9.3 Testing & Documentation
- [ ] Integration tests for all flows
- [ ] Error scenario tests

## 10. References

### 10.1 External Documentation
- [Official API Documentation](https://ap-gateway.mastercard.com/api/documentation/apiDocumentation.html)

### 10.2 Internal References
- `guides/patterns/patterns.md`
- `guides/learnings/learnings.md`
