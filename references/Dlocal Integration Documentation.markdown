# Dlocal Integration Documentation

This document outlines the technical integration of **Dlocal** into **Hyperswitch**, an open-source payment orchestrator, focusing exclusively on card payment flows. It covers API endpoints, authentication, request/response structures, webhooks, configuration, and additional considerations to ensure a robust integration.

## Connector URLs

**Urls**

- **baseUrl**: `https://api.dlocal.com`
- **sandboxUrl**: `https://sandbox.dlocal.com`
- **Other Important URLs**:
  - **connect_url**: Not applicable (Dlocal does not use OAuth-style account connection).
  - **token_url**: Not applicable (Dlocal uses API keys, not token exchange).
  - **webhook_url**: Merchant-defined, e.g., `http://merchant.com/notifications` (configured in Dlocal dashboard).
  - **documentation_url**: `https://docs.dlocal.com/`
  - **status_url**: Not explicitly provided (contact Dlocal support for health check endpoints).

## Authentication

- **Authentication Type**: HMAC-SHA256
- **Steps to Configure**:
  1. Register a Dlocal account via the official website ([Dlocal Docs](https://docs.dlocal.com/)).
  2. Access the Dlocal Merchant Dashboard and navigate to **Settings > Integration** to obtain:
     - `X-Login`: API login credential.
     - `X-Trans-Key`: Transaction key.
     - `Secret Key`: Used for generating HMAC-SHA256 signatures.
  3. Generate the signature using the `Secret Key`, combining request data (e.g., `X-Date`, request body) with the HMAC-SHA256 algorithm.
- **Example Authentication Headers**:
  ```bash
  X-Login: sak223k2wdksdl2
  X-Trans-Key: fm12O7G9
  X-Date: 2018-02-20T15:44:42.310Z
  Authorization: V2-HMAC-SHA256, Signature: 1bd227f9d892a7f4581b998c21e353b1686a6bdad5940e7bb6aa596c96e0a6ec
  X-Version: 2.1
  User-Agent: MerchantTest/1.0
  Content-Type: application/json
  ```
- **Curl Example** (for reference):
  ```bash
  curl -X POST \
    -H 'X-Date: 2018-02-20T15:44:42.310Z' \
    -H 'X-Login: sak223k2wdksdl2' \
    -H 'X-Trans-Key: fm12O7G9' \
    -H 'Content-Type: application/json' \
    -H 'X-Version: 2.1' \
    -H 'User-Agent: MerchantTest/1.0' \
    -H 'Authorization: V2-HMAC-SHA256, Signature: 1bd227f9d892a7f4581b998c21e353b1686a6bdad5940e7bb6aa596c96e0a6ec' \
    -d '{...}' \
    https://api.dlocal.com/secure_payments
  ```

## Supported Flows with Request/Response Structures

Below are the supported card payment flows, including endpoint details, HTTP methods, headers, and example payloads. All flows use the authentication headers described above.

### 1. Authorize / Payment Intent Creation

This flow reserves funds on a customer’s card without capturing them, suitable for scenarios like hotel bookings.

- **Endpoint URL**: `https://api.dlocal.com/secure_payments`
- **HTTP Method**: `POST`
- **Required Headers**: As specified in the Authentication section.
- **Request Payload (Example)**:
  ```json
  {
    "amount": 100,
    "currency": "USD",
    "country": "BR",
    "payment_method_id": "CARD",
    "payment_method_flow": "DIRECT",
    "payer": {
      "name": "John Doe",
      "email": "john.doe@example.com",
      "document": "123456789"
    },
    "card": {
      "holder_name": "John Doe",
      "number": "4111111111111111",
      "cvv": "123",
      "expiration_month": 12,
      "expiration_year": 2025,
      "capture": false
    },
    "order_id": "123456",
    "notification_url": "http://merchant.com/notifications"
  }
  ```
- **Response Payload (Example)**:
  ```json
  {
    "id": "D-4-e2227981-8ec8-48fd-8e9a-19fedb08d73a",
    "amount": 100,
    "currency": "USD",
    "payment_method_id": "CARD",
    "payment_method_type": "CARD",
    "payment_method_flow": "DIRECT",
    "country": "BR",
    "card": {
      "holder_name": "John Doe",
      "expiration_month": 12,
      "expiration_year": 2025,
      "brand": "VI",
      "last4": "1111"
    },
    "created_date": "2019-02-06T21:04:43.000+0000",
    "approved_date": "2019-02-06T21:04:44.000+0000",
    "status": "AUTHORIZED",
    "status_detail": "The payment was authorized",
    "status_code": "600",
    "order_id": "123456",
    "notification_url": "http://merchant.com/notifications"
  }
  ```
- **Curl Example**:
  ```bash
  curl -X POST \
    -H 'X-Date: 2018-02-20T15:44:42.310Z' \
    -H 'X-Login: sak223k2wdksdl2' \
    -H 'X-Trans-Key: fm12O7G9' \
    -H 'Content-Type: application/json' \
    -H 'X-Version: 2.1' \
    -H 'User-Agent: MerchantTest/1.0' \
    -H 'Authorization: V2-HMAC-SHA256, Signature: 1bd227f9d892a7f4581b998c21e353b1686a6bdad5940e7bb6aa596c96e0a6ec' \
    -d '{
      "amount": 100,
      "currency": "USD",
      "country": "BR",
      "payment_method_id": "CARD",
      "payment_method_flow": "DIRECT",
      "payer": {
        "name": "John Doe",
        "email": "john.doe@example.com",
        "document": "123456789"
      },
      "card": {
        "holder_name": "John Doe",
        "number": "4111111111111111",
        "cvv": "123",
        "expiration_month": 12,
        "expiration_year": 2025,
        "capture": false
      },
      "order_id": "123456",
      "notification_url": "http://merchant.com/notifications"
    }' \
    https://api.dlocal.com/secure_payments
  ```

### 2. Capture

This flow captures the authorized funds to complete the transaction.

- **Endpoint URL**: `https://api.dlocal.com/payments`
- **HTTP Method**: `POST`
- **Required Headers**: As specified in the Authentication section.
- **Request Payload (Example)**:
  ```json
  {
    "authorization_id": "D-4-e2227981-8ec8-48fd-8e9a-19fedb08d73a",
    "amount": 100,
    "currency": "USD",
    "order_id": "123456-capture"
  }
  ```
- **Response Payload (Example)**:
  ```json
  {
    "id": "D-4-09f52dd0-5cfa-4b0e-a471-1608ea0dba24",
    "amount": 100,
    "currency": "USD",
    "country": "BR",
    "created_date": "2019-02-07T13:47:06.000+0000",
    "approved_date": "2019-02-07T13:47:06.000+0000",
    "status": "PAID",
    "status_detail": "The payment was paid",
    "status_code": "200",
    "order_id": "123456-capture",
    "authorization_id": "D-4-e2227981-8ec8-48fd-8e9a-19fedb08d73a"
  }
  ```
- **Curl Example**:
  ```bash
  curl -X POST \
    -H 'X-Date: 2018-02-20T15:44:42.310Z' \
    -H 'X-Login: sak223k2wdksdl2' \
    -H 'X-Trans-Key: fm12O7G9' \
    -H 'Content-Type: application/json' \
    -H 'X-Version: 2.1' \
    -H 'User-Agent: MerchantTest/1.0' \
    -H 'Authorization: V2-HMAC-SHA256, Signature: 1bd227f9d892a7f4581b998c21e353b1686a6bdad5940e7bb6aa596c96e0a6ec' \
    -d '{
      "authorization_id": "D-4-e2227981-8ec8-48fd-8e9a-19fedb08d73a",
      "amount": 100,
      "currency": "USD",
      "order_id": "123456-capture"
    }' \
    https://api.dlocal.com/payments
  ```

### 3. Refund

This flow processes refunds for completed payments.

- **Endpoint URL**: `https://api.dlocal.com/refunds`
- **HTTP Method**: `POST`
- **Required Headers**: As specified in the Authentication section.
- **Request Payload (Example)**:
  ```json
  {
    "payment_id": "PAY4334346343",
    "amount": 100.00,
    "currency": "USD",
    "notification_url": "http://merchant.com/notifications"
  }
  ```
- **Response Payload (Example)**:
  ```json
  {
    "id": "REF42342",
    "payment_id": "PAY245235",
    "amount": 100.00,
    "amount_refunded": 100.00,
    "currency": "USD",
    "status": "SUCCESS",
    "status_code": 200,
    "status_detail": "The refund was paid.",
    "created_date": "2018-02-15T15:14:52-00:00",
    "order_id": "SALE-124635123"
  }
  ```
- **Curl Example**:
  ```bash
  curl -X POST \
    -H 'X-Date: 2018-02-20T15:44:42.310Z' \
    -H 'X-Login: sak223k2wdksdl2' \
    -H 'X-Trans-Key: fm12O7G9' \
    -H 'Content-Type: application/json' \
    -H 'X-Version: 2.1' \
    -H 'User-Agent: MerchantTest/1.0' \
    -H 'Authorization: V2-HMAC-SHA256, Signature: 1bd227f9d892a7f4581b998c21e353b1686a6bdad5940e7bb6aa596c96e0a6ec' \
    -d '{
      "payment_id": "PAY4334346343",
      "amount": 100.00,
      "currency": "USD",
      "notification_url": "http://merchant.com/notifications"
    }' \
    https://api.dlocal.com/refunds
  ```

### 4. Sync / Psync

This flow retrieves the status of an existing payment.

- **Endpoint URL**: `https://api.dlocal.com/payments/{payment_id}/status`
- **HTTP Method**: `GET`
- **Required Headers**: As specified in the Authentication section (excluding `Content-Type`).
- **Request Payload**: None (payment_id included in URL).
- **Response Payload (Example)**:
  ```json
  {
    "id": "PAY4334346343",
    "amount": 100,
    "currency": "USD",
    "payment_method_id": "CARD",
    "payment_method_type": "CARD",
    "payment_method_flow": "DIRECT",
    "country": "BR",
    "status": "PAID",
    "status_detail": "The payment was paid",
    "status_code": "200",
    "created_date": "2019-02-07T13:47:06.000+0000"
  }
  ```
- **Curl Example**:
  ```bash
  curl -X GET \
    -H 'X-Date: 2018-02-20T15:44:42.310Z' \
    -H 'X-Login: sak223k2wdksdl2' \
    -H 'X-Trans-Key: fm12O7G9' \
    -H 'X-Version: 2.1' \
    -H 'User-Agent: MerchantTest/1.0' \
    -H 'Authorization: V2-HMAC-SHA256, Signature: 1bd227f9d892a7f4581b998c21e353b1686a6bdad5940e7bb6aa596c96e0a6ec' \
    https://api.dlocal.com/payments/PAY4334346343/status
  ```

### 5. Dispute Handling

- **Description**: Disputes (chargebacks) are managed via asynchronous webhook notifications sent to the merchant’s registered chargeback notification URL. Merchants can retrieve chargeback details using a specific endpoint.
- **Endpoint URL (Retrieval)**: `https://api.dlocal.com/chargebacks/{chargeback_id}`
- **HTTP Method**: `GET`
- **Required Headers**: As specified in the Authentication section (excluding `Content-Type`).
- **Request Payload**: None (chargeback_id included in URL).
- **Response Payload (Example)**:
  ```json
  {
    "id": "CHAR42342",
    "payment_id": "PAY245235",
    "amount": 100.00,
    "currency": "USD",
    "status": "COMPLETED",
    "status_code": 200,
    "status_detail": "The chargeback was executed.",
    "created_date": "2018-02-15T15:14:52-00:00",
    "notification_url": "http://merchant.com/notifications",
    "order_id": "merchant_num_123456"
  }
  ```
- **Curl Example (Retrieval)**:
  ```bash
  curl -X GET \
    -H 'X-Date: 2018-02-20T15:44:42.310Z' \
    -H 'X-Login: sak223k2wdksdl2' \
    -H 'X-Trans-Key: fm12O7G9' \
    -H 'X-Version: 2.1' \
    -H 'User-Agent: MerchantTest/1.0' \
    -H 'Authorization: V2-HMAC-SHA256, Signature: 1bd227f9d892a7f4581b998c21e353b1686a6bdad5940e7bb6aa596c96e0a6ec' \
    https://api.dlocal.com/chargebacks/CHAR42342
  ```

### 6. Tokenization / Vaulting

- **Description**: Dlocal supports saving card details for future use by setting `"save": true` in the card object during payment creation. A `card_id` is returned for subsequent tokenized payments.
- **API for Tokenizing Payment Methods**:
  - Use `https://api.dlocal.com/secure_payments` with `"save": true` to save a card.
  - Use `https://api.dlocal.com/payments` for payments with a saved `card_id`.
- **Request Payload (Example for Saving Card)**:
  ```json
  {
    "amount": 120.00,
    "currency": "USD",
    "country": "BR",
    "payment_method_id": "CARD",
    "payment_method_flow": "DIRECT",
    "payer": {
      "name": "John Doe",
      "email": "john.doe@example.com",
      "document": "123456789"
    },
    "card": {
      "holder_name": "John Doe",
      "number": "4111111111111111",
      "cvv": "123",
      "expiration_month": 12,
      "expiration_year": 2025,
      "save": true
    },
    "order_id": "123456",
    "notification_url": "http://merchant.com/notifications"
  }
  ```
- **Response Payload (Example)**:
  ```json
  {
    "id": "D-4-cf8eef6b-52d5-4320-b5ea-f5e0bbe4343f",
    "amount": 120,
    "currency": "USD",
    "payment_method_id": "CARD",
    "payment_method_type": "CARD",
    "payment_method_flow": "DIRECT",
    "country": "BR",
    "card": {
      "holder_name": "John Doe",
      "expiration_month": 12,
      "expiration_year": 2025,
      "brand": "VI",
      "last4": "1111",
      "card_id": "CID-124c18a5-874d-4982-89d7-b9c256e647b5"
    },
    "created_date": "2018-12-26T20:26:09.000+0000",
    "approved_date": "2018-12-26T20:26:09.000+0000",
    "status": "PAID",
    "status_detail": "The payment was paid",
    "status_code": "200",
    "order_id": "123456",
    "notification_url": "http://merchant.com/notifications"
  }
  ```
- **Supported Tokens or Vault Options**: Dlocal returns a `card_id` for saved cards, which can be used for future payments without re-entering card details.

## Webhooks

- **List of Event Types**:
  - `payment_succeeded`: Payment completed successfully.
  - `payment_failed`: Payment was rejected or failed.
  - `refund_processed`: Refund was successfully processed.
  - `refund_failed`: Refund processing failed.
  - `chargeback_applied`: A chargeback was initiated by the customer.
- **Example Webhook Payload (Chargeback Notification)**:
  ```json
  {
    "id": "CHAR42342",
    "payment_id": "PAY245235",
    "amount": 100.00,
    "currency": "USD",
    "status": "COMPLETED",
    "status_code": 200,
    "status_detail": "The chargeback was executed.",
    "created_date": "2018-02-15T15:14:52-00:00",
    "notification_url": "http://merchant.com/notifications",
    "order_id": "merchant_num_123456"
  }
  ```
- **Signature Verification**: Webhooks are signed using HMAC-SHA256. Merchants must verify the signature using the `Secret Key` to ensure authenticity. Refer to Dlocal’s security documentation ([Payins Security](https://docs.dlocal.com/reference/payins-security)) for details.

## Configuration & Setup

- **Required Configuration Parameters**:
  - `X-Login`: API login credential.
  - `X-Trans-Key`: Transaction key.
  - `Secret Key`: For generating and verifying signatures.
  - `notification_url`: URL for receiving webhook notifications.
  - `Merchant Country Code`: Required for country-specific configurations.
- **Environment-Specific Variables**:
  - **Sandbox**: Use `https://sandbox.dlocal.com` and test API keys.
  - **Production**: Use `https://api.dlocal.com` and live API keys.
- **Supported Currencies, Countries, Card Networks, and Payment Methods**:
  - **Currencies**: Supports dozens of currencies (e.g., USD, BRL, ARS). See Dlocal’s country reference ([Payment Methods](https://docs.dlocal.com/docs/payment-method)).
  - **Countries**: Over 40 countries, including Brazil, Argentina, Mexico, etc.
  - **Card Networks**: Visa, Mastercard, and local card schemes (e.g., Meeza in Egypt).
  - **Payment Methods**: Credit cards, debit cards (focus of this integration).

| Parameter | Description | Example |
|-----------|-------------|---------|
| X-Login | API login credential | sak223k2wdksdl2 |
| X-Trans-Key | Transaction key | fm12O7G9 |
| Secret Key | For HMAC-SHA256 signatures | (Provided by Dlocal) |
| notification_url | Webhook URL | http://merchant.com/notifications |
| Merchant Country Code | Country-specific code | BR (Brazil) |

## Additional Information

- **PCI Enabled Integration**:
  - Dlocal is PCI DSS Level 1 compliant, ensuring secure card data handling.
  - Merchants using the Full API (`https://api.dlocal.com/secure_payments`) must comply with PCI DSS standards ([Full API](https://docs.dlocal.com/docs/full-api)).
  - Steps:
    1. Implement secure card data handling per PCI DSS requirements.
    2. Use Dlocal’s Smart Fields or hosted solutions to minimize PCI scope.
    3. Regularly audit and validate compliance with Dlocal support.
- **Available SDKs or Libraries**:
  - Dlocal provides SDKs for Node.js, Python, Java, and others. Check the official documentation for availability ([Dlocal Docs](https://docs.dlocal.com/)).
- **Rate Limits or Throttling Details**:
  - Not explicitly documented. Contact Dlocal support for specific rate limit policies.
- **Idempotency Key Support**:
- **Known Connector-Specific Behavior or Quirks**:
  - Use `https://api.dlocal.com/secure_payments` for payments with full card details (PCI compliance required).
  - Use `https://api.dlocal.com/payments` for tokenized payments with `card_id`.
  - Webhooks are critical for asynchronous updates (e.g., chargebacks, refunds).
  - Installment plans require creating an installment plan first ([Installments](https://docs.dlocal.com/docs/installments)).
  - Sandbox testing allows simulating payment statuses using the `description` field ([Make a Test Payment](https://docs.dlocal.com/docs/make-a-test-payment)).