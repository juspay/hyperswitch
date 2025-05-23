# Spreedly Integration Documentation

This document provides comprehensive technical details for integrating **Spreedly** with **Hyperswitch**, an open-source payment orchestrator, focusing on card payment processing. It includes API flows, request/response structures, URLs, authentication details, and configuration steps to ensure a seamless integration.

## Connector URLs

| **URL Type**         | **Details**                                                                 |
|----------------------|-----------------------------------------------------------------------------|
| **baseUrl**          | https://core.spreedly.com/v1/                                               |
| **sandboxUrl**       | Not separate; use `sandbox=true` when creating gateways for testing.         |
| **connect_url**      | Use Spreedly’s iFrame or Express solutions for payment collection. Specific URLs provided upon setup. |
| **token_url**        | Tokenization via payment method creation: https://core.spreedly.com/v1/payment_methods |
| **documentation_url**| https://docs.spreedly.com/reference/api/v1/                                  |
| **status_url**       | Not explicitly specified; health check endpoints may require further investigation. |

## Authentication

- **Authentication Type**: HTTP Basic Authentication
- **Steps to Configure**:
  1. Log in to the [Spreedly ID Dashboard](https://developer.spreedly.com/).
  2. Retrieve your `environment_key` and `access_secret`.
  3. Encode credentials as `base64(environment_key:access_secret)` for the `Authorization` header.
- **Example Authentication Header**:
  ```
  Authorization: Basic <base64 encoded environment_key:access_secret>
  ```
- **Example Curl**:
  ```bash
  curl -X GET https://core.spreedly.com/v1/transactions.json \
    -H 'Authorization: Basic <base64_credentials>'
  ```

## Supported Flows with Request/Response Structures

### 1. Authorize / Payment Intent Creation

- **Endpoint URL**: https://core.spreedly.com/v1/gateways/<gateway_token>/transactions.json
- **HTTP Method**: PUT
- **Required Headers**:
  - `Authorization: Basic <base64_credentials>`
  - `Content-Type: application/json`
- **Request Payload**:
  ```json
  {
    "transaction": {
      "type": "authorize",
      "amount": "10.00",
      "payment_method_token": "payment_method_token_here",
      "options": {
        "capture": false
      }
    }
  }
  ```
- **Response Payload**:
  ```json
  {
    "transaction": {
      "id": "transaction_id",
      "status": "success",
      "amount": "10.00",
      "type": "authorize",
      "payment_method": {
        "token": "payment_method_token_here"
      },
      "created_at": "2025-05-22T12:04:00Z",
      "updated_at": "2025-05-22T12:04:00Z"
    }
  }
  ```
- **Curl**:
  ```bash
  curl -X PUT \
    https://core.spreedly.com/v1/gateways/<gateway_token>/transactions.json \
    -H 'Content-Type: application/json' \
    -H 'Authorization: Basic <base64_credentials>' \
    -d '{"transaction": {"type": "authorize", "amount": "10.00", "payment_method_token": "payment_method_token_here", "options": {"capture": false}}}'
  ```

### 2. Capture

- **Endpoint URL**: https://core.spreedly.com/v1/gateways/<gateway_token>/transactions.json
- **HTTP Method**: PUT
- **Required Headers**: Same as above
- **Request Payload**:
  ```json
  {
    "transaction": {
      "type": "capture",
      "amount": "10.00",
      "reference_transaction_id": "original_authorize_transaction_id"
    }
  }
  ```
- **Response Payload**:
  ```json
  {
    "transaction": {
      "id": "capture_transaction_id",
      "status": "success",
      "amount": "10.00",
      "type": "capture",
      "created_at": "2025-05-22T12:04:00Z",
      "updated_at": "2025-05-22T12:04:00Z"
    }
  }
  ```
- **Curl**:
  ```bash
  curl -X PUT \
    https://core.spreedly.com/v1/gateways/<gateway_token>/transactions.json \
    -H 'Content-Type: application/json' \
    -H 'Authorization: Basic <base64_credentials>' \
    -d '{"transaction": {"type": "capture", "amount": "10.00", "reference_transaction_id": "original_authorize_transaction_id"}}'
  ```

### 3. Refund

- **Endpoint URL**: https://core.spreedly.com/v1/gateways/<gateway_token>/transactions.json
- **HTTP Method**: PUT
- **Required Headers**: Same as above
- **Request Payload**:
  ```json
  {
    "transaction": {
      "type": "refund",
      "amount": "10.00",
      "reference_transaction_id": "original_purchase_transaction_id"
    }
  }
  ```
- **Response Payload**:
  ```json
  {
    "transaction": {
      "id": "refund_transaction_id",
      "status": "success",
      "amount": "10.00",
      "type": "refund",
      "created_at": "2025-05-22T12:04:00Z",
      "updated_at": "2025-05-22T12:04:00Z"
    }
  }
  ```
- **Curl**:
  ```bash
  curl -X PUT \
    https://core.spreedly.com/v1/gateways/<gateway_token>/transactions.json \
    -H 'Content-Type: application/json' \
    -H 'Authorization: Basic <base64_credentials>' \
    -d '{"transaction": {"type": "refund", "amount": "10.00", "reference_transaction_id": "original_purchase_transaction_id"}}'
  ```

### 4. Sync / Psync

- **Description**: Synchronizes transaction data with Hyperswitch by retrieving transaction lists or details.
- **Endpoint URL**: https://core.spreedly.com/v1/transactions.json
- **HTTP Method**: GET
- **Required Headers**: `Authorization: Basic <base64_credentials>`
- **Request Payload**: None (GET request)
- **Response Payload**:
  ```json
  {
    "transactions": [
      {
        "id": "transaction_id_1",
        "status": "success",
        "amount": "10.00",
        "type": "authorize",
        "created_at": "2025-05-22T12:04:00Z"
      },
      {
        "id": "transaction_id_2",
        "status": "success",
        "amount": "15.00",
        "type": "capture",
        "created_at": "2025-05-22T12:04:00Z"
      }
    ]
  }
  ```
- **Curl**:
  ```bash
  curl -X GET \
    https://core.spreedly.com/v1/transactions.json \
    -H 'Authorization: Basic <base64_credentials>'
  ```
- **Specific Transaction Details**:
  - **Endpoint URL**: https://core.spreedly.com/v1/transactions/<transaction_token>.json
  - **HTTP Method**: GET
  - **Response Payload**:
    ```json
    {
      "transaction": {
        "id": "transaction_id",
        "status": "success",
        "amount": "10.00",
        "type": "authorize",
        "created_at": "2025-05-22T12:04:00Z",
        "updated_at": "2025-05-22T12:04:00Z"
      }
    }
    ```

### 5. Dispute Handling

- **Description**: Disputes are tracked within transaction data. Retrieve transaction details to check for dispute status.
- **Endpoint URL**: https://core.spreedly.com/v1/transactions/<transaction_token>.json
- **HTTP Method**: GET
- **Required Headers**: `Authorization: Basic <base64_credentials>`
- **Request Payload**: None (GET request)
- **Response Payload**:
  ```json
  {
    "transaction": {
      "id": "transaction_id",
      "status": "dispute",
      "dispute": {
        "status": "pending",
        "reason": "fraudulent",
        "created_at": "2025-05-22T12:04:00Z"
      },
      "amount": "10.00",
      "type": "purchase",
      "created_at": "2025-05-22T12:04:00Z"
    }
  }
  ```
- **Curl**:
  ```bash
  curl -X GET \
    https://core.spreedly.com/v1/transactions/<transaction_token>.json \
    -H 'Authorization: Basic <base64_credentials>'
  ```

### 6. Tokenization / Vaulting

- **Endpoint URL**: https://core.spreedly.com/v1/payment_methods.json
- **HTTP Method**: POST
- **Required Headers**:
  - `Authorization: Basic <base64_credentials>`
  - `Content-Type: application/json`
- **Request Payload**:
  ```json
  {
    "payment_method": {
      "type": "credit_card",
      "payment_method": {
        "number": "4111111111111111",
        "month": "12",
        "year": "2025",
        "first_name": "John",
        "last_name": "Doe"
      }
    }
  }
  ```
- **Response Payload**:
  ```json
  {
    "payment_method": {
      "token": "payment_method_token_here",
      "type": "credit_card",
      "last_four_digits": "1111",
      "card_type": "visa",
      "created_at": "2025-05-22T12:04:00Z",
      "updated_at": "2025-05-22T12:04:00Z"
    }
  }
  ```
- **Curl**:
  ```bash
  curl -X POST \
    https://core.spreedly.com/v1/payment_methods.json \
    -H 'Content-Type: application/json' \
    -H 'Authorization: Basic <base64_credentials>' \
    -d '{"payment_method": {"type": "credit_card", "payment_method": {"number": "4111111111111111", "month": "12", "year": "2025", "first_name": "John", "last_name": "Doe"}}}'
  ```
- **Supported Tokens**: Credit card tokens are reusable across gateways, supporting universal tokenization.
- **Vault Options**: Tokens are stored securely at Spreedly, reducing PCI compliance scope.

## Configuration & Setup

- **Required Configuration Parameters**:
  - `environment_key`: Unique identifier for the Spreedly environment.
  - `access_secret`: Secret key for authentication.
  - Gateway credentials (e.g., for Stripe: `login`, `password`).
- **Environment-Specific Variables**:
  - **Sandbox**: Create gateways with `sandbox=true` for testing. Use test card numbers (e.g., 4111111111111111 for Visa) from [Spreedly Test Data](https://core.spreedly.com/reference/test-data/#credit-cards).
  - **Production**: Create gateways without `sandbox` parameter for live transactions.
- **Supported Currencies, Countries, Card Networks, and Payment Methods**:
  - Depend on the gateway (e.g., Stripe supports Visa, MasterCard, American Express, and multiple currencies like USD, EUR).
  - Check gateway-specific documentation via [Spreedly Gateways](https://docs.spreedly.com/reference/api/v1/gateways/).

## Additional Information

- **PCI Enabled Integration**:
  - Spreedly minimizes PCI compliance requirements by handling card data through its iFrame or Express solutions, ensuring card details do not touch merchant servers.
  - Steps:
    1. Implement Spreedly’s iFrame or Express for payment collection.
    2. Tokenize payment methods using the payment methods endpoint.
    3. Use tokens for transactions, avoiding direct card data handling.
  - This approach aligns with PCI DSS standards, reducing compliance scope.