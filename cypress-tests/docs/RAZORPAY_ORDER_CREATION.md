# Razorpay Order Creation Flow Documentation

## Overview

This document clarifies the distinction between **Razorpay's optional order creation flow** and **BNPL providers' mandatory pre-authorization requirements**.

## Razorpay Order Creation (Optional)

### What It Is
- An **optional** step performed via the `/orders` API before initiating payment authorization
- Specifically applicable to **UPI payments** (`requires_order_creation_before_payment` returns `true` for `PaymentMethod::Upi`)
- Creates an order and generates an `order_id` that can be used in subsequent payment requests

### Key Characteristics

| Aspect | Details |
|--------|---------|
| **Mandatory** | No - This is an optional optimization, not a requirement |
| **Applicable To** | Primarily UPI payments |
| **API Endpoint** | `POST /v1/orders` |
| **Response Field** | `order_id` |
| **Internal Order Creation** | No - Hyperswitch does NOT create internal orders within the `/payments` call |
| **Flow Type** | Optional pre-authorization step |

### API Flow

```
1. (Optional) Create Order
   POST /v1/orders
   {
     "amount": 10000,
     "currency": "INR",
     "receipt": "receipt#1"
   }
   
   Response:
   {
     "id": "order_xxxxxxx",
     "amount": 10000,
     "currency": "INR",
     "status": "created"
   }

2. Initiate Payment
   POST /payments
   {
     "amount": 10000,
     "currency": "INR",
     // order_id can be included if step 1 was performed
   }
```

## BNPL Provider Pre-Authorization (Mandatory)

### What It Is
- A **mandatory** connector-specific pre-authorization requirement
- Must be completed before the main payment authorization can proceed
- Often involves creating an order/session at the BNPL provider

### Key Characteristics

| Aspect | Details |
|--------|---------|
| **Mandatory** | Yes - Cannot proceed without it |
| **Applicable To** | All BNPL transactions |
| **API Endpoint** | Connector-specific (e.g., Klarna, Afterpay) |
| **Response Field** | Session token or authorization reference |
| **Internal Order Creation** | Yes - Often creates internal order/session state |
| **Flow Type** | Mandatory pre-authorization requirement |

## Comparison Table

| Feature | Razorpay Order Creation | BNPL Pre-Authorization |
|---------|------------------------|----------------------|
| **Mandatory** | ❌ No | ✅ Yes |
| **Purpose** | Optimization/Organization | Authorization prerequisite |
| **UPI Only** | ✅ Yes | ❌ No (all BNPL) |
| **Internal State** | ❌ No | ✅ Yes |
| **Skip Allowed** | ✅ Yes | ❌ No |
| **Fail Payment If Missing** | ❌ No | ✅ Yes |

## Implementation Notes

### In Hyperswitch

The `requires_order_creation_before_payment` method in the `Connector` enum determines if order creation is needed:

```rust
impl Connector {
    pub fn requires_order_creation_before_payment(&self, pm: &PaymentMethod) -> bool {
        match self {
            Connector::Razorpay => matches!(pm, PaymentMethod::Upi),
            // ... other connectors
        }
    }
}
```

### Configuration

Razorpay connector configuration is located at:
- **Config File**: `cypress/e2e/configs/Payment/Razorpay.js`
- **Utils Integration**: `cypress/e2e/configs/Payment/Utils.js`

## Testing

### Prerequisites

1. Add Razorpay credentials to `creds.json`:
```json
{
  "razorpay": {
    "connector_account_details": {
      "auth_type": "HeaderKey",
      "api_key": "your_api_key"
    }
  }
}
```

2. Set environment variables:
```bash
export CYPRESS_CONNECTOR="razorpay"
export CYPRESS_BASEURL="http://localhost:8080"
export CYPRESS_CONNECTOR_AUTH_FILE_PATH="/path/to/creds.json"
```

### Supported Payment Methods

- Card payments (Visa, Mastercard, etc.)
- UPI payments (with optional order creation flow)

## References

- [Razorpay Orders API Documentation](https://razorpay.com/docs/api/orders/)
- [Hyperswitch Connector Specifications](../Payment/Razorpay.js)
- [DeepWiki: Razorpay Order Creation Flow](https://deepwiki.com/search/for-razorpay-connector-what-is)
