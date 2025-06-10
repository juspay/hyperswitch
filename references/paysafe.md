PaySafe Integration Documentation
PaySafe is a comprehensive payments platform offering secure card processing capabilities through multiple API endpoints and authentication methods. Based on the search results, PaySafe provides a robust REST-based API architecture supporting various payment operations including authorization, capture, refund, and settlement processes. The platform utilizes Basic Authentication with JSON-formatted requests and responses, making it suitable for integration with payment orchestrators like Hyperswitch.

Connector URLs
baseUrl: https://api.paysafe.com
sandboxUrl: https://api.test.paysafe.com

Other Important URLs:

card_payments_url: https://api.paysafe.com/cardpayments/v1/accounts/{account_id}

payment_hub_url: https://api.paysafe.com/paymenthub/v1

documentation_url: https://developer.paysafe.com/en/api-docs/

merchant_portal_url: https://login.paysafe.com (for account management)

Authentication
PaySafe uses HTTP Basic Authentication for API requests. The authentication process involves combining the API key username and password, encoding them in Base64, and including them in the Authorization header.

Authentication Type: Basic Auth
Configuration Steps:

Obtain API key username and password from Paysafe Merchant Back Office under Settings > API Key

Combine username and password with a colon separator

Encode the string using Base64

Include in Authorization header as "Basic [encoded_string]"

Example Authentication Header:

text
Authorization: Basic TWVyY2hhbnRYWVo6Qi10c3QxLTAtNTFlZDM5ZTQtMzEyZDAyMzQ1ZDNmMTIzMTIwODgxZGZmOWJiNDAyMGE4OWU4YWM0NGNkZmRjZWNkNzAyMTUxMTgyZmRjOTUyMjcyNjYxZDI5MGFiMmU1ODQ5ZTMxYmIwM2RlZWRlN2U=
Required Headers for All Requests:

Content-Type: application/json

Authorization: Basic [encoded_credentials]

Supported Flows with Request/Response Structures
1. Authorize / Payment Intent Creation
PaySafe requires a two-step process: first creating a Payment Handle, then processing the payment.

Step 1: Create Payment Handle
URL: POST /paymenthub/v1/paymenthandles

Request:

json
{
  "merchantRefNum": "merchant-ref-123",
  "transactionType": "PAYMENT",
  "paymentType": "CARD",
  "amount": 1000,
  "currencyCode": "USD",
  "card": {
    "cardNum": "4111111111111111",
    "cardExpiry": {
      "month": 12,
      "year": 2025
    },
    "cvv": "123",
    "holderName": "John Doe"
  },
  "billingDetails": {
    "street": "123 Main St",
    "city": "New York",
    "state": "NY",
    "country": "US",
    "zip": "10001"
  }
}
Response:

json
{
  "id": "ph_12345678-1234-1234-1234-123456789012",
  "paymentHandleToken": "PHT_abc123def456",
  "status": "PAYABLE",
  "action": "NONE",
  "merchantRefNum": "merchant-ref-123",
  "paymentType": "CARD"
}
Step 2: Process Payment
URL: POST /paymenthub/v1/payments

Request:

json
{
  "merchantRefNum": "10f64573-65d8-4d14-8337-66264094662f",
  "amount": 1407,
  "currencyCode": "USD",
  "dupCheck": false,
  "settleWithAuth": false,
  "paymentHandleToken": "PHT_abc123def456",
  "description": "Payment Authorization"
}
Response:

json
{
  "id": "3f7992f8-f550-42a7-bc10-79728a43d3c6",
  "paymentType": "CARD",
  "paymentHandleToken": "PHT_abc123def456",
  "merchantRefNum": "10f64573-65d8-4d14-8337-66264094662f",
  "currencyCode": "USD",
  "txnTime": "2024-01-15T10:30:00Z",
  "status": "COMPLETED",
  "amount": 1407,
  "availableToSettle": 1407
}
Curl:

bash
curl -X POST https://api.test.paysafe.com/paymenthub/v1/payments \
-u devcentre322:B-qa2-0-53625f86-302c021476f52bdc9deab7aea876bb28762e62f92fc6712d0214736abf501e9675e55940e83ef77f5c304edc7968 \
-H 'Content-Type: application/json' \
-d '{
  "merchantRefNum": "10f64573-65d8-4d14-8337-66264094662f",
  "amount": 1407,
  "currencyCode": "USD",
  "dupCheck": false,
  "settleWithAuth": true,
  "paymentHandleToken": "SC7nzOK9blfEKX1r",
  "description": "Demo Paysafe Js"
}'
2. Capture
URL: POST /paymenthub/v1/payments/{payment_id}/settlements

Request:

json
{
  "merchantRefNum": "settlement-ref-123",
  "amount": 1407,
  "dupCheck": false
}
Response:

json
{
  "id": "settlement_12345678-1234-1234-1234-123456789012",
  "paymentId": "3f7992f8-f550-42a7-bc10-79728a43d3c6",
  "merchantRefNum": "settlement-ref-123",
  "amount": 1407,
  "currencyCode": "USD",
  "status": "PENDING",
  "txnTime": "2024-01-15T10:35:00Z"
}
3. Refund
URL: POST /paymenthub/v1/settlements/{settlement_id}/refunds

Request:

json
{
  "merchantRefNum": "refund-ref-123",
  "amount": 500,
  "dupCheck": false
}
Response:

json
{
  "id": "refund_12345678-1234-1234-1234-123456789012",
  "settlementId": "settlement_12345678-1234-1234-1234-123456789012",
  "merchantRefNum": "refund-ref-123",
  "amount": 500,
  "currencyCode": "USD",
  "status": "PENDING",
  "txnTime": "2024-01-15T10:40:00Z"
}
4. Sync / Psync
URL: GET /v1/payments/{paymentId}

Request: No body required for GET request

Response:

json
{
  "id": "3f7992f8-f550-42a7-bc10-79728a43d3c6",
  "paymentType": "CARD",
  "merchantRefNum": "10f64573-65d8-4d14-8337-66264094662f",
  "currencyCode": "USD",
  "txnTime": "2024-01-15T10:30:00Z",
  "status": "COMPLETED",
  "amount": 1407,
  "availableToSettle": 1407,
  "riskReasonCode": []
}
5. Dispute Handling
PaySafe handles disputes through their Merchant Back Office system rather than direct API calls. Merchants can retrieve transaction details for chargeback responses using the Card Payments API.

URL: GET /cardpayments/v1/accounts/{account_id}/auths/{transaction_id}

Request: No body required

Response:

json
{
  "id": "transaction_id",
  "merchantRefNum": "merchant-ref-123",
  "amount": 1407,
  "currencyCode": "USD",
  "status": "COMPLETED",
  "authCode": "123456",
  "txnTime": "2024-01-15T10:30:00Z",
  "card": {
    "lastDigits": "1111",
    "cardExpiry": {
      "month": 12,
      "year": 2025
    }
  }
}
6. Tokenization / Vaulting
PaySafe supports payment tokenization through their Customer Vault system. Tokens can be created during the Payment Handle creation process by including customer profile information.

URL: POST /paymenthub/v1/paymenthandles with customer profile

Request:

json
{
  "merchantRefNum": "token-ref-123",
  "transactionType": "PAYMENT",
  "paymentType": "CARD",
  "profile": {
    "firstName": "John",
    "lastName": "Doe",
    "email": "john.doe@example.com"
  },
  "card": {
    "cardNum": "4111111111111111",
    "cardExpiry": {
      "month": 12,
      "year": 2025
    },
    "cvv": "123",
    "holderName": "John Doe"
  }
}
Response:

json
{
  "id": "ph_12345678-1234-1234-1234-123456789012",
  "paymentHandleToken": "PHT_abc123def456",
  "status": "PAYABLE",
  "profile": {
    "id": "profile_12345678",
    "firstName": "John",
    "lastName": "Doe",
    "email": "john.doe@example.com"
  },
  "card": {
    "paymentToken": "token_abc123def456",
    "lastDigits": "1111",
    "cardExpiry": {
      "month": 12,
      "year": 2025
    }
  }
}
Configuration & Setup
Required Configuration Parameters:

API Key Username: Obtained from Paysafe Merchant Back Office

API Key Password: Obtained from Paysafe Merchant Back Office

Account ID: Specific account identifier for card payments

Merchant ID: Merchant identifier for transaction processing

Environment-Specific Variables:

Sandbox: https://api.test.paysafe.com

Production: https://api.paysafe.com

Supported Features:

Card Networks: Visa, Visa Debit, Visa Electron, Visa Prepaid, American Express, Mastercard, Mastercard Debit (Maestro), Mastercard Prepaid, Discover

Currencies: USD, EUR, GBP, CAD, and others based on account setup

Countries: Multiple countries supported based on merchant account configuration

Payment Methods: Credit cards, Debit cards with 3D Secure 2 authentication support

Additional Information
PCI Compliance Integration Steps:

Use Payment Handles: Always use Payment Handles instead of directly processing card data to reduce PCI scope

Secure Tokenization: Implement card tokenization through Customer Vault to avoid storing sensitive card data

HTTPS Communication: All API communications must use HTTPS endpoints

Data Encryption: Card data should be encrypted before transmission to PaySafe endpoints

Compliance Level: Using Payment Handles and tokenization reduces PCI compliance requirements to SAQ-A level

Important Notes:

Payment Handle tokens have a 15-minute lifespan and must be used immediately after creation

For 3D Secure transactions, customers may be redirected to authentication pages

Test environment uses special card numbers for simulation - never use real card data in testing

Settlement can be automatic (settleWithAuth: true) or manual (settleWithAuth: false)