# Airwallex Connector Documentation

This document outlines key information for integrating with the Airwallex API, based on the existing Hyperswitch connector implementation.

## Base URL

The base URL for Airwallex API calls is configured dynamically within the Hyperswitch system via `connectors.airwallex.base_url`. Specific API endpoints are appended to this base URL.

## Authentication

Airwallex uses a Bearer token authentication mechanism.

1.  **Obtain Access Token:**
    *   **Endpoint:** `POST /api/v1/authentication/login`
    *   **Request Headers:**
        *   `x-api-key`: Your Airwallex API key.
        *   `x-client-id`: Your Airwallex client ID.
        *   `Content-Length: 0`
    *   **Response Body (`AirwallexAuthUpdateResponse`):**
        ```json
        {
          "expires_at": "2023-10-27T10:30:00Z", // Example ISO 8601 timestamp
          "token": "your_access_token_here"
        }
        ```

2.  **Using Access Token:**
    For subsequent API calls, include the obtained token in the `Authorization` header:
    *   `Authorization: Bearer <access_token>`

## Common Headers

*   `Content-Type: application/json` (for requests with a JSON body)
*   `Authorization: Bearer <access_token>` (for authenticated endpoints)

## API Endpoints and Payloads

### 1. Create Payment Intent (PreProcessing)

*   **Endpoint:** `POST /api/v1/pa/payment_intents/create`
*   **Request Body (`AirwallexIntentRequest`):**
    ```json
    {
      "request_id": "unique_request_id_string",
      "amount": "1000", // Amount in minor units (e.g., cents)
      "currency": "USD",
      "merchant_order_id": "your_merchant_order_id",
      "referrer_data": {
        "type": "hyperswitch",
        "version": "1.0.0"
      }
    }
    ```
*   **Response Body (`AirwallexPaymentsResponse`):**
    ```json
    {
      "status": "REQUIRES_PAYMENT_METHOD", // Or other statuses like SUCCEEDED, FAILED, PENDING, REQUIRES_CUSTOMER_ACTION, REQUIRES_CAPTURE, CANCELLED
      "id": "payment_intent_id_from_airwallex",
      "amount": 10.00, // Amount in major units
      "payment_consent_id": "consent_id_if_applicable",
      "next_action": { // Optional, present if further action is needed
        "url": "https://redirect.url.com/...",
        "method": "GET", // Or POST
        "data": {
          "JWT": "jwt_token_if_any",
          "threeDSMethodData": "3ds_method_data_if_any",
          "token": "token_if_any",
          "provider": "provider_name_if_any",
          "version": "version_if_any"
        },
        "stage": "WAITING_USER_INFO_INPUT" // Or WAITING_DEVICE_DATA_COLLECTION
      }
    }
    ```

### 2. Confirm Payment Intent (Authorize)

*   **Endpoint:** `POST /api/v1/pa/payment_intents/{payment_intent_id}/confirm`
*   **Request Body (`AirwallexPaymentsRequest`):**
    ```json
    {
      "request_id": "unique_request_id_string",
      "payment_method": {
        // Card example
        "card": {
          "expiry_month": "12",
          "expiry_year": "2025",
          "number": "card_number_here",
          "cvc": "cvc_here"
        },
        "type": "card"
        // Wallet example (Google Pay)
        // "googlepay": {
        //   "encrypted_payment_token": "encrypted_gpay_token",
        //   "payment_data_type": "encrypted_payment_token"
        // },
        // "type": "googlepay"
      },
      "payment_method_options": { // Optional
        "card": {
          "auto_capture": true // Or false
        }
      },
      "return_url": "https://your.return.url/after-payment", // Optional
      "device_data": {
        "accept_header": "application/json, text/plain, */*",
        "browser": {
          "java_enabled": false,
          "javascript_enabled": true,
          "user_agent": "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) ..."
        },
        "ip_address": "192.168.1.1",
        "language": "en-US",
        "mobile": { // Optional
          "device_model": "iPhone13,4",
          "os_type": "iOS",
          "os_version": "15.0"
        },
        "screen_color_depth": 24,
        "screen_height": 1080,
        "screen_width": 1920,
        "timezone": "-07:00" // Or other timezone offset
      }
    }
    ```
*   **Response Body:** Same as `AirwallexPaymentsResponse` (see Create Payment Intent).

### 3. Retrieve Payment Intent (PSync)

*   **Endpoint:** `GET /api/v1/pa/payment_intents/{payment_intent_id}`
*   **Response Body (`AirwallexPaymentsSyncResponse`):** Similar structure to `AirwallexPaymentsResponse`.
    ```json
    {
      "status": "SUCCEEDED",
      "id": "payment_intent_id_from_airwallex",
      "amount": 10.00,
      "payment_consent_id": "consent_id_if_applicable",
      "next_action": null // Or details if action is pending
    }
    ```

### 4. Confirm Continue (CompleteAuthorize for 3DS)

*   **Endpoint:** `POST /api/v1/pa/payment_intents/{payment_intent_id}/confirm_continue`
*   **Request Body (`AirwallexCompleteRequest`):**
    ```json
    {
      "request_id": "unique_request_id_string",
      "three_ds": {
        "acs_response": "acs_response_payload_as_string" // Optional
      },
      "type": "3ds_continue"
    }
    ```
*   **Response Body:** Same as `AirwallexPaymentsResponse`.

### 5. Capture Payment Intent

*   **Endpoint:** `POST /api/v1/pa/payment_intents/{payment_intent_id}/capture`
*   **Request Body (`AirwallexPaymentsCaptureRequest`):**
    ```json
    {
      "request_id": "unique_request_id_string",
      "amount": "1000" // Optional, amount in minor units to capture
    }
    ```
*   **Response Body:** Same as `AirwallexPaymentsResponse`.

### 6. Cancel Payment Intent (Void)

*   **Endpoint:** `POST /api/v1/pa/payment_intents/{payment_intent_id}/cancel`
*   **Request Body (`AirwallexPaymentsCancelRequest`):**
    ```json
    {
      "request_id": "unique_request_id_string",
      "cancellation_reason": "User requested cancellation" // Optional
    }
    ```
*   **Response Body:** Same as `AirwallexPaymentsResponse`.

### 7. Create Refund

*   **Endpoint:** `POST /api/v1/pa/refunds/create`
*   **Request Body (`AirwallexRefundRequest`):**
    ```json
    {
      "request_id": "unique_request_id_string",
      "amount": "500", // Optional, amount in minor units to refund
      "reason": "Product returned", // Optional
      "payment_intent_id": "original_payment_intent_id"
    }
    ```
*   **Response Body (`RefundResponse`):**
    ```json
    {
      "acquirer_reference_number": "arn_if_available",
      "amount": 5.00, // Amount in major units
      "id": "refund_id_from_airwallex",
      "status": "SUCCEEDED" // Or FAILED, RECEIVED, ACCEPTED
    }
    ```

### 8. Retrieve Refund (RSync)

*   **Endpoint:** `GET /api/v1/pa/refunds/{refund_id}`
*   **Response Body:** Same as `RefundResponse` (see Create Refund).

## Error Handling

*   **Error Response Body (`AirwallexErrorResponse`):**
    ```json
    {
      "code": "ERROR_CODE_FROM_AIRWALLEX",
      "message": "Detailed error message.",
      "source": "Field or component causing the error, if applicable"
    }
    ```

## Webhooks

*   **Signature Verification:**
    *   **Algorithm:** HMAC SHA256
    *   **Signature Header:** `x-signature` (hex-encoded signature)
    *   **Timestamp Header:** `x-timestamp`
    *   **Message to Sign:** Concatenation of the `x-timestamp` header value and the raw request body.
*   **Event Types (Partial List - refer to `AirwallexWebhookEventType` enum in `transformers.rs` for a more complete list):**
    *   `payment_intent.created`
    *   `payment_intent.requires_payment_method`
    *   `payment_intent.cancelled`
    *   `payment_intent.succeeded`
    *   `payment_intent.requires_capture`
    *   `payment_intent.requires_customer_action`
    *   `payment_attempt.authorized`
    *   `payment_attempt.authorization_failed`
    *   `refund.received`
    *   `refund.accepted`
    *   `refund.succeeded`
    *   `refund.failed`
    *   `dispute.accepted`
    *   `dispute.won`
    *   `dispute.lost`
*   **Webhook Payload Structure (General):**
    ```json
    {
      "source_id": "relevant_id_like_payment_intent_id_or_refund_id",
      "name": "payment_intent.succeeded", // Event type
      "data": {
        "object": {
          // ... actual event data object ...
        }
      }
    }
    ```
    For dispute events, the `data.object` will contain `AirwallexDisputeObject` structure.

This document provides a high-level overview. For precise field definitions, enums, and conditional logic, refer to the Rust struct definitions in `crates/hyperswitch_connectors/src/connectors/airwallex/transformers.rs`.
