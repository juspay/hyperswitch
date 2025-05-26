# Monex Integration Documentation

This document provides technical details for integrating **Monex** (assumed to be MONEXgroup, a Canadian payment processing provider) into **Hyperswitch**, an open-source payment orchestrator, with a focus on card payments. The goal is to outline all API flows, request/response structures, URLs, and authentication details required for full integration. Since Monex’s official API documentation was not publicly accessible, this guide uses standard payment gateway API conventions and general information from MONEXgroup’s services. Please verify all details with Monex’s official API documentation.

## Connector URLs

| URL Type            | Description                                      | URL (Placeholder)                              |
|---------------------|--------------------------------------------------|------------------------------------------------|
| **baseUrl**         | Production base URL for Monex API                | https://api.monexgroup.com/v1/                 |
| **sandboxUrl**      | Sandbox/testing URL for Monex API                | https://sandbox.api.monexgroup.com/v1/         |
| **connect_url**     | Initiates account connection (if applicable)     | Not available; likely OAuth-based              |
| **token_url**       | Exchanges authorization code for access token    | Not available; likely OAuth-based              |
| **documentation_url** | Official Monex API documentation               | Contact Monex for access                      |
| **status_url**      | Health check or service status endpoint         | https://api.monexgroup.com/v1/status           |

**Note**: The URLs above are placeholders based on standard API conventions. Obtain actual URLs from Monex’s developer portal.

## Authentication

- **Authentication Type**: OAuth2 (assumed based on common payment gateway practices).
- **Steps to Configure**:
  1. Register on Monex’s developer portal to obtain a client ID and client secret.
  2. Use these credentials to request an access token via the OAuth2 client credentials flow.
  3. Include the access token in the `Authorization` header for all API requests.
- **Example Authentication**:
  - **Header**: `Authorization: Bearer <access_token>`
  - **Curl Command for Access Token**:
    ```bash
    curl -X POST \
      https://api.monexgroup.com/oauth/token \
      -H 'Content-Type: application/x-www-form-urlencoded' \
      -d 'grant_type=client_credentials&client_id=your_client_id&client_secret=your_client_secret'
    ```

## Supported Flows with Request/Response Structures

### 1. Authorize / Payment Intent Creation
- **Endpoint URL**: `/payments/authorize`
- **HTTP Method**: POST
- **Required Headers**:
  - `Authorization: Bearer <access_token>`
  - `Content-Type: application/json`
- **Request Payload**:
  ```json
  {
    "amount": 100,
    "currency": "USD",
    "card": {
      "number": "4111111111111111",
      "exp_month": 12,
      "exp_year": 2025,
      "cvc": "123"
    }
  }
  ```
- **Response Payload**:
  ```json
  {
    "payment_id": "12345",
    "status": "authorized"
  }
  ```
- **Curl**:
  ```bash
  curl -X POST \
    -H "Authorization: Bearer <access_token>" \
    -H "Content-Type: application/json" \
    -d '{"amount": 100, "currency": "USD", "card": {"number": "4111111111111111", "exp_month": 12, "exp_year": 2025, "cvc": "123"}}' \
    https://api.monexgroup.com/v1/payments/authorize
  ```

### 2. Capture
- **Endpoint URL**: `/payments/capture/{payment_id}`
- **HTTP Method**: POST
- **Required Headers**:
  - `Authorization: Bearer <access_token>`
  - `Content-Type: application/json`
- **Request Payload**:
  ```json
  {
    "amount": 100
  }
  ```
- **Response Payload**:
  ```json
  {
    "payment_id": "12345",
    "status": "captured"
  }
  ```
- **Curl**:
  ```bash
  curl -X POST \
    -H "Authorization: Bearer <access_token>" \
    -H "Content-Type: application/json" \
    -d '{"amount": 100}' \
    https://api.monexgroup.com/v1/payments/capture/12345
  ```

### 3. Refund
- **Endpoint URL**: `/payments/refund/{payment_id}`
- **HTTP Method**: POST
- **Required Headers**:
  - `Authorization: Bearer <access_token>`
  - `Content-Type: application/json`
- **Request Payload**:
  ```json
  {
    "amount": 50
  }
  ```
- **Response Payload**:
  ```json
  {
    "payment_id": "12345",
    "status": "refunded"
  }
  ```
- **Curl**:
  ```bash
  curl -X POST \
    -H "Authorization: Bearer <access_token>" \
    -H "Content-Type: application/json" \
    -d '{"amount": 50}' \
    https://api.monexgroup.com/v1/payments/refund/12345
  ```

### 4. Sync / Psync
- **Endpoint URL**: `/payments/{payment_id}`
- **HTTP Method**: GET
- **Required Headers**:
  - `Authorization: Bearer <access_token>`
- **Request Payload**: None
- **Response Payload**:
  ```json
  {
    "payment_id": "12345",
    "status": "captured",
    "amount": 100,
    "currency": "USD"
  }
  ```
- **Curl**:
  ```bash
  curl -X GET \
    -H "Authorization: Bearer <access_token>" \
    https://api.monexgroup.com/v1/payments/12345
  ```

### 5. Dispute Handling
- **Endpoint URL**: `/disputes/{dispute_id}`
- **HTTP Method**: GET
- **Required Headers**:
  - `Authorization: Bearer <access_token>`
- **Request Payload**: None
- **Response Payload**:
  ```json
  {
    "dispute_id": "67890",
    "status": "open",
    "amount": 100,
    "currency": "USD"
  }
  ```
- **Curl**:
  ```bash
  curl -X GET \
    -H "Authorization: Bearer <access_token>" \
    https://api.monexgroup.com/v1/disputes/67890
  ```

### 6. Tokenization / Vaulting
- **API for Tokenizing Payment Methods**: `/tokens`
- **Request and Response Structure**:
  - **Request**:
    ```json
    {
      "card": {
        "number": "4111111111111111",
        "exp_month": 12,
        "exp_year": 2025,
        "cvc": "123"
      }
    }
    ```
  - **Response**:
    ```json
    {
      "token": "tok_12345"
    }
    ```
- **Supported Tokens or Vault Options**: Card tokens for secure storage of payment details.
- **Curl**:
  ```bash
  curl -X POST \
    -H "Authorization: Bearer <access_token>" \
    -H "Content-Type: application/json" \
    -d '{"card": {"number": "4111111111111111", "exp_month": 12, "exp_year": 2025, "cvc": "123"}}' \
    https://api.monexgroup.com/v1/tokens
  ```

## Configuration & Setup

- **Required Configuration Parameters**:
  - **API Key** or **Client ID/Client Secret** for OAuth2 authentication.
  - **Merchant ID**: Unique identifier for the merchant account (if applicable).
- **Environment-Specific Variables**:
  - **Sandbox Environment**: Use `https://sandbox.api.monexgroup.com/v1/` for testing.
  - **Production Environment**: Use `https://api.monexgroup.com/v1/` for live transactions.
- **Supported Currencies, Countries, Card Networks, and Payment Methods**:
  - **Currencies**: Likely includes USD, CAD, EUR, and others (verify with Monex).
  - **Countries**: Primarily Canada and the US, possibly others (verify with Monex).
  - **Card Networks**: Visa, Mastercard, American Express, Discover (based on [MONEXgroup Homepage](https://monexgroup.com/)).
  - **Payment Methods**: Credit/debit card payments, potentially recurring payments via virtual terminals.

## Additional Information

- **PCI-Enabled Integration**:
  - **Tokenization**: Use the `/tokens` endpoint to securely store card details, avoiding direct handling of sensitive data.
  - **Secure Transmission**: Ensure all API requests are made over HTTPS to comply with PCI DSS standards.
  - **Compliance**: Work with Monex to complete a Self-Assessment Questionnaire (SAQ) for PCI compliance, ensuring secure card data processing.
  - **Best Practices**: Avoid storing card numbers locally; rely on Monex’s tokenization for recurring payments.

**Important Note**: The endpoints, payloads, and authentication details provided are based on standard payment gateway conventions due to the unavailability of Monex’s public API documentation. Contact MONEXgroup directly to obtain accurate API specifications, including exact URLs, supported flows, and authentication requirements. Testing in the sandbox environment is recommended before production deployment.