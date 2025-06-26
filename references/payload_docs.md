# Payload Integration Documentation

Based on my research, Payload is a payment processing platform that offers comprehensive APIs for handling various payment operations. Below is the complete technical documentation for integrating Payload into Hyperswitch.

## Connector URLs

**Base URLs:**
- baseUrl: https://api.payload.com
- sandboxUrl: Test environment uses the same base URL but with test API keys (prefixed with test_)

**Other Important URLs:**
- documentation_url: https://docs.payload.com/apis/
- status_url: Not explicitly documented in available sources
- connect_url: Not applicable (direct API integration)
- token_url: Not applicable (uses API key authentication)

## Authentication

**Authentication Type:** HTTP Basic Authentication

**Configuration Steps:**
1. Obtain API secret key from Payload dashboard under Settings > API Keys
2. Use the secret key as the username in HTTP Basic Auth header
3. Production keys start with secret_key_, test keys start with test_secret_key_

**Authentication Headers:**
```
Authorization: Basic <base64(secret_key:)>
```

**Example Curl Command:**
```bash
curl "https://api.payload.com/transactions" \
-u secret_key_3bW9JMZtPVDOfFNzwRdfE:
```

## Supported Flows with Request/Response Structures

### 1. Authorize / Payment Intent Creation

**Endpoint:** POST https://api.payload.com/transactions  
**Method:** POST

**Headers:**
```
Authorization: Basic <base64(api_key:)>
Content-Type: application/x-www-form-urlencoded
```

**Request:**
```json
{
  "amount": 100,
  "type": "payment",
  "status": "authorized",
  "payment_method": {
    "type": "card",
    "card": {
      "card_number": "4242 4242 4242 4242",
      "expiry": "12/25",
      "card_code": "123"
    }
  }
}
```

**Response:**
```json
{
  "id": "txn_3bW9JN4BVk3wU0ZZQs2Ay",
  "type": "payment",
  "amount": 100,
  "status": "authorized",
  "currency": "USD",
  "created_at": "2023-01-01T12:00:00Z",
  "payment_method": {
    "id": "pm_3bW9JMoQUQiZaEV8TgPUO",
    "type": "card",
    "card": {
      "last_four": "4242",
      "brand": "visa"
    }
  }
}
```

**Curl:**
```bash
curl "https://api.payload.com/transactions" \
-u secret_key_3bW9JMZtPVDOfFNzwRdfE: \
-d "amount=100" \
-d "type=payment" \
-d "status=authorized" \
-d "payment_method[type]=card" \
-d "payment_method[card][card_number]=4242 4242 4242 4242" \
-d "payment_method[card][expiry]=12/25" \
-d "payment_method[card][card_code]=123"
```

### 2. Capture

**Endpoint:** PUT https://api.payload.com/transactions/<transaction_id>  
**Method:** PUT

**Headers:**
```
Authorization: Basic <base64(api_key:)>
Content-Type: application/x-www-form-urlencoded
```

**Request:**
```json
{
  "status": "processed"
}
```

**Response:**
```json
{
  "id": "txn_3bW9JN4BVk3wU0ZZQs2Ay",
  "type": "payment",
  "amount": 100,
  "status": "processed",
  "currency": "USD",
  "updated_at": "2023-01-01T12:05:00Z"
}
```

**Curl:**
```bash
curl -X PUT "https://api.payload.com/transactions/txn_3bW9JN4BVk3wU0ZZQs2Ay" \
-u secret_key_3bW9JMZtPVDOfFNzwRdfE: \
-d "status=processed"
```

### 3. Refund

**Endpoint:** POST https://api.payload.com/transactions  
**Method:** POST

**Headers:**
```
Authorization: Basic <base64(api_key:)>
Content-Type: application/x-www-form-urlencoded
```

**Request:**
```json
{
  "type": "refund",
  "amount": 100,
  "ledger": [
    {
      "assoc_transaction_id": "txn_3bW9JN4BVk3wU0ZZQs2Ay"
    }
  ]
}
```

**Response:**
```json
{
  "id": "txn_refund_3bW9JN4BVk3wU0ZZQs2Ay",
  "type": "refund",
  "amount": 100,
  "status": "processed",
  "currency": "USD",
  "created_at": "2023-01-01T12:10:00Z",
  "ledger": [
    {
      "assoc_transaction_id": "txn_3bW9JN4BVk3wU0ZZQs2Ay"
    }
  ]
}
```

**Curl:**
```bash
curl "https://api.payload.com/transactions" \
-u secret_key_3bW9JMZtPVDOfFNzwRdfE: \
-d "type=refund" \
-d "amount=100" \
-d "ledger[0][assoc_transaction_id]=txn_3bW9JN4BVk3wU0ZZQs2Ay"
```

### 4. Sync / Psync

**Endpoint:** GET https://api.payload.com/transactions/<transaction_id>  
**Method:** GET

**Headers:**
```
Authorization: Basic <base64(api_key:)>
```

**Request:**  
No request body required

**Response:**
```json
{
  "id": "txn_3bW9JN4BVk3wU0ZZQs2Ay",
  "type": "payment",
  "amount": 100,
  "status": "processed",
  "currency": "USD",
  "created_at": "2023-01-01T12:00:00Z",
  "updated_at": "2023-01-01T12:05:00Z",
  "payment_method": {
    "id": "pm_3bW9JMoQUQiZaEV8TgPUO",
    "type": "card",
    "card": {
      "last_four": "4242",
      "brand": "visa"
    }
  }
}
```

**Curl:**
```bash
curl "https://api.payload.com/transactions/txn_3bW9JN4BVk3wU0ZZQs2Ay" \
-u secret_key_3bW9JMZtPVDOfFNzwRdfE:
```

### 5. Dispute Handling

Based on the available documentation, Payload does not explicitly provide dispute handling APIs. Disputes would typically be handled through their dashboard or customer support channels. The payment status overview indicates various transaction states including "Rejected" status for declined payments, but specific dispute management endpoints are not documented.

**Note:** Dispute handling may require manual processing through Payload's dashboard or support channels.

### 6. Tokenization / Vaulting

**Endpoint:** POST https://api.payload.com/payment_methods  
**Method:** POST

**Headers:**
```
Authorization: Basic <base64(api_key:)>
Content-Type: application/x-www-form-urlencoded
```

**Request:**
```json
{
  "type": "card",
  "card": {
    "card_number": "4242 4242 4242 4242",
    "expiry": "12/25",
    "card_code": "123"
  },
  "customer_id": "acct_u7NDGPfjBc4uwChD",
  "default_payment_method": true
}
```

**Response:**
```json
{
  "id": "pm_3bW9JMoQUQiZaEV8TgPUO",
  "type": "card",
  "card": {
    "last_four": "4242",
    "brand": "visa",
    "expiry": "12/25"
  },
  "customer_id": "acct_u7NDGPfjBc4uwChD",
  "is_default": true,
  "created_at": "2023-01-01T12:00:00Z"
}
```

## Configuration & Setup

**Required Configuration Parameters:**
- API Key: Secret key from Payload dashboard
- Environment: Production or Test (determined by API key prefix)

**Environment-specific Variables:**
- Production: API keys start with secret_key_
- Sandbox: API keys start with test_secret_key_

**Supported Features:**
- Currencies: USD, CAD (primary focus on North American markets)
- Countries: United States and Canada
- Card Networks: Visa, Mastercard, American Express, Discover
- Payment Methods: Credit/Debit Cards, Bank Accounts (ACH), RTP, Check21

## Additional Information

### PCI Compliance

Payload is PCI-DSS Level 1 certified. To enable PCI-compliant integration:

- **Use Client-Side UI Elements:** Payload provides secure UI components that encrypt sensitive data on the client side, keeping your servers out of PCI scope
- **Tokenization:** Utilize Payload's secure vault for storing payment methods
- **Secure Data Handling:** All sensitive financial data is encrypted using cryptographic splitting and stored in Payload's zero-access vault

**Integration Steps for PCI Compliance:**
1. Implement Payload's client-side UI elements for card data collection
2. Route sensitive payment data directly to Payload's secure endpoints
3. Store only tokenized payment method references in your system
4. Use Payload's APIs for all payment operations without handling raw card data

### Webhook Support

Payload supports webhooks for payment events:
- Payment success/failure notifications
- Status change updates
- Configurable webhook URLs for different events

### Risk Management

Payload includes built-in fraud detection and risk management with machine learning-based behavioral analysis across 30+ classifiers.