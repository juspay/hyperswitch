```markdown
# Authipay Integration Documentation

We are integrating **Authipay**, a Fiserv payment processing solution, into **Hyperswitch**, an open-source payment orchestrator. This document provides comprehensive technical documentation for the Authipay connector, focusing exclusively on card payment processing. The goal is to detail all API flows, request/response structures, URLs, and authentication mechanisms required for a complete integration.

## Connector URLs

Authipay provides distinct URLs for sandbox and production environments to facilitate testing and live transaction processing. Below are the key URLs for the integration:

- **baseUrl**: https://prod.emea.api.fiservapps.com/sandbox/ipp/payments-gateway/v2/
  - **Description**: Production Base URL for live transactions.
- **sandboxUrl**: https://prod.emea.api.fiservapps.com/sandbox/ipp/payments-gateway/v2
  - **Description**: Sandbox URL for testing and development.

### Other Important URLs
- **connect_url**: Not applicable (no specific endpoint for account connection in the provided documentation).
- **token_url**: Not applicable (no specific endpoint for token exchange in the provided documentation).
- **documentation_url**: https://docs.fiserv.dev/public/docs/payments-getting-started
  - **Description**: Official Authipay API documentation.
- **status_url**: Not specified in the provided documentation.
  - **Description**: No health check or service status endpoint is explicitly defined.

**Note**: The provided OpenAPI specification does not include endpoints for initiating account connections, exchanging authorization codes, or checking service status. All payment-related interactions occur through the base URLs, with authentication handled via API keys and additional security headers. For any additional URLs, consult Authipay’s support or developer portal.

## Authentication

Authipay employs **API Key-based authentication** augmented with additional security headers to ensure secure and verifiable API requests. The authentication process requires specific headers, including `Api-Key`, `Client-Request-Id`, `Timestamp`, and `Message-Signature`, to prevent tampering and validate request integrity.

### Authentication Details
- **Authentication Type**: API Key-Based Authentication
- **Required Headers**:
  - `Content-Type: application/json`
  - Specifies the request body format.
  - `Api-Key: YOUR_API_KEY`
    - The unique API key issued by Authipay.
  - `Client-Request-Id: UNIQUE_REQUEST_ID`
    - A unique identifier (e.g., UUID) for each request, enabling traceability.
  - `Timestamp: CURRENT_TIMESTAMP`
    - Current time in milliseconds since Unix epoch, ensuring request freshness.
  - `Message-Signature: GENERATED_SIGNATURE`
    - A Base64-encoded HMAC SHA256 signature for request validation, computed using the request payload and other headers.

### Steps to Configure Authentication
1. **Obtain API Key and Secret Key**:
   - Register with Authipay through the Fiserv Developer Portal (https://developer.fiserv.com/) to obtain your API Key and Secret Key.
   - Ensure both keys are associated with the same application for consistent authentication.
2. **Generate Client-Request-Id**:
   - Create a unique identifier for each request, typically using a UUID library (e.g., `uuidv4()` in JavaScript).
   - Example: `550e8400-e29b-41d4-a1b2-3f4544d0000`.
3. **Set Timestamp**:
   - Capture the current timestamp in milliseconds (e.g., `new Date().getTime()` in JavaScript).
   - Example: `1695792000000` (representing a specific point in time).
4. **Stringify Request Body**:
   - Convert the JSON request body to a string for inclusion in the signature calculation.
   - For GET requests, use an empty string (`""`) as the request body.
5. **Generate Message-Signature**:
   - Concatenate the following: `Api-Key + Client-Request-Id + Timestamp + Request Body`.
   - Compute the HMAC SHA256 hash of the concatenated string using the Secret Key.
   - Encode the hash in Base64 to produce the `Message-Signature`.
   - **Note**: The exact signature generation algorithm is not fully detailed in the provided documentation. Refer to Authipay’s official resources (e.g., https://docs.fiserv.dev/public/docs/message-signature) for precise instructions.
6. **Include Headers in Requests**:
   - Add all required headers to each API call, ensuring consistency across requests.

### Example Authentication Headers
```bash
Content-Type: application/json
Api-Key: YOUR_API_KEY
Client-Request-Id: 550e8400-e29b-41d4-a1b2-3f4544d0000
Timestamp: 1695792000000
Message-Signature: GENERATED_BASE64_SIGNATURE
```

### Example Curl Command with Authentication
```bash
curl -X POST https://prod.emea.api.fiservapps.com/sandbox/ipp/payments-gateway/v2//payments \
  -H "Content-Type: application/json" \
  -H "Api-Key: YOUR_API_KEY" \
  -H "Client-Request-Id: 550e8400-e29b-41d4-a1b2-3f4544d0000" \
  -H "Timestamp: 1695792000000" \
  -H "Message-Signature: GENERATED_SIGNATURE" \
  -d '{}'
```

**Note**: The `Message-Signature` requires careful implementation. Developers should validate the signature generation process with Authipay’s support or documentation to ensure compliance with their security standards.

## Supported Flows with Request/Response Structures

The following sections detail the API flows for card payments, including authorize/payment intent creation, capture, refund, sync/psync, dispute handling, and tokenization/vaulting. Each flow includes the endpoint URL, HTTP method, required headers, request and response payloads, and a curl example where applicable. The flows are derived from the provided OpenAPI specification, focusing on card-based transactions (`PaymentCard` payment method).

### 1. Authorize / Payment Intent Creation

This flow initiates a card pre-authorization transaction, reserving funds on the cardholder’s account for later capture.

- **Endpoint URL**: `/payments`
- **HTTP Method**: POST
- **Required Headers**:
  - `Content-Type: application/json`
  - `Api-Key: YOUR_API_KEY`
  - `Client-Request-Id: UNIQUE_REQUEST_ID`
  - `Timestamp: CURRENT_TIMESTAMP`
  - `Message-Signature: GENERATED_SIGNATURE`

#### Request Payload
```json
{
  "requestType": "PaymentCardPreAuthTransaction",
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
  "splitShipment": {
    "totalCount": 1,
    "finalShipment": true
  },
  "incrementalFlag": false
}
```

**Notes**:
- `requestType`: Specifies the transaction type (`PaymentCardPreAuthTransaction` for pre-authorization).
- `transactionAmount`: Includes the amount and currency (ISO 4217 code, e.g., `EUR`).
- `paymentMethod`: Contains card details, including card number, security code, and expiry date.
- `splitShipment` and `incrementalFlag`: Optional fields for handling partial shipments or incremental authorizations.

#### Response Payload
```json
{
  "clientRequestId": "550e8400-e29b-41d4-a1b2-3f4544d0000",
  "apiTraceId": "rrt-1234567890-abcdef",
  "ipgTransactionId": "838123456789",
  "orderId": "ORDER12345",
  "transactionType": "PREAUTH",
  "transactionState": "AUTHORIZED",
  "paymentMethodDetails": {
    "paymentCard": {
      "expiryDate": {
        "month": "12",
        "year": "24"
      },
      "bin": "542418",
      "last4": "1732",
      "brand": "MASTERCARD"
    }
  },
  "transactionAmount": {
    "total": 12.04,
    "currency": "EUR"
  },
  "transactionTime": 1695792000,
  "approvedAmount": {
    "total": 12.04,
    "currency": "EUR"
  },
  "transactionStatus": "APPROVED",
  "approvalCode": "123456",
  "processor": {
    "responseCode": "00",
    "responseMessage": "Success"
  }
}
```

**Notes**:
- `ipgTransactionId`: Unique transaction identifier for subsequent operations (e.g., capture, refund).
- `transactionState`: Indicates the current state (`AUTHORIZED` for successful pre-authorization).
- `transactionStatus`: `APPROVED` indicates success; other values (e.g., `DECLINED`) indicate failure.
- `processor.responseCode`: `00` signifies approval; other codes indicate specific decline reasons.

#### Curl Example
```bash
curl -X POST https://prod.emea.api.fiservapps.com/sandbox/ipp/payments-gateway/v2//payments \
  -H "Content-Type: application/json" \
  -H "Api-Key: YOUR_API_KEY" \
  -H "Client-Request-Id: 550e8400-e29b-41d4-a1b2-3f4544d0000" \
  -H "Timestamp: 1695792000000" \
  -H "Message-Signature: GENERATED_SIGNATURE" \
  -d '{
    "requestType": "PaymentCardPreAuthTransaction",
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
    "splitShipment": {
      "totalCount": 1,
      "finalShipment": true
    },
    "incrementalFlag": false
  }'
```

### 2. Capture

This flow captures a previously authorized amount, finalizing the transaction and transferring funds.

- **Endpoint URL**: `/payments/{transaction-id}`
- **HTTP Method**: POST
- **Required Headers**:
  - `Content-Type: application/json`
  - `Api-Key: YOUR_API_KEY`
  - `Client-Request-Id: UNIQUE_REQUEST_ID`
  - `Timestamp: CURRENT_TIMESTAMP`
  - `Message-Signature: GENERATED_SIGNATURE`

#### Request Payload
```json
{
  "requestType": "PaymentCardPostAuthTransaction",
  "transactionAmount": {
    "total": 12.04,
    "currency": "EUR"
  }
}
```

**Notes**:
- Replace `{transaction-id}` with the `ipgTransactionId` from the authorize response.
- `requestType`: `PaymentCardPostAuthTransaction` for capturing authorized funds.
- `transactionAmount`: Must match or be less than the authorized amount.

#### Response Payload
```json
{
  "clientRequestId": "550e8400-e29b-41d4-a1b2-3f4544d0000",
  "apiTraceId": "rrt-1234567890-abcdef",
  "ipgTransactionId": "838123456789",
  "orderId": "ORDER12345",
  "transactionType": "POSTAUTH",
  "transactionState": "CAPTURED",
  "transactionAmount": {
    "total": 12.04,
    "currency": "EUR"
  },
  "transactionTime": 1695792000,
  "approvedAmount": {
    "total": 12.04,
    "currency": "EUR"
  },
  "transactionStatus": "APPROVED",
  "approvalCode": "123456",
  "processor": {
    "responseCode": "00",
    "responseMessage": "Success"
  }
}
```

**Notes**:
- `transactionState`: `CAPTURED` indicates successful capture.
- `transactionStatus`: `APPROVED` confirms the capture was successful.

#### Curl Example
```bash
curl -X POST https://prod.emea.api.fiservapps.com/sandbox/ipp/payments-gateway/v2//payments/838123456789 \
  -H "Content-Type: application/json" \
  -H "Api-Key: YOUR_API_KEY" \
  -H "Client-Request-Id: 550e8400-e29b-41d4-a1b2-3f4544d0000" \
  -H "Timestamp: 1695792000000" \
  -H "Message-Signature: GENERATED_SIGNATURE" \
  -d '{
    "requestType": "PaymentCardPostAuthTransaction",
    "transactionAmount": {
      "total": 12.04,
      "currency": "EUR"
    }
  }'
```

### 3. Refund

This flow processes a refund for a previously captured transaction, returning funds to the cardholder.

- **Endpoint URL**: `/payments/{transaction-id}`
- **HTTP Method**: POST
- **Required Headers**:
  - `Content-Type: application/json`
  - `Api-Key: YOUR_API_KEY`
  - `Client-Request-Id: UNIQUE_REQUEST_ID`
  - `Timestamp: CURRENT_TIMESTAMP`
  - `Message-Signature: GENERATED_SIGNATURE`

#### Request Payload
```json
{
  "requestType": "PaymentCardReturnTransaction",
  "transactionAmount": {
    "total": 12.04,
    "currency": "EUR"
  }
}
```

**Notes**:
- Replace `{transaction-id}` with the `ipgTransactionId` from the original transaction.
- `requestType`: `PaymentCardReturnTransaction` for processing refunds.
- `transactionAmount`: Specifies the refund amount, which can be partial or full.

#### Response Payload
```json
{
  "clientRequestId": "550e8400-e29b-41d4-a1b2-3f4544d0000",
  "apiTraceId": "rrt-1234567890-abcdef",
  "ipgTransactionId": "838123456789",
  "orderId": "ORDER12345",
  "transactionType": "RETURN",
  "transactionState": "RETURNED",
  "transactionAmount": {
    "total": 12.04,
    "currency": "EUR"
  },
  "transactionTime": 1695792000,
  "approvedAmount": {
    "total": 12.04,
    "currency": "EUR"
  },
  "transactionStatus": "APPROVED",
  "approvalCode": "123456",
  "processor": {
    "responseCode": "00",
    "responseMessage": "Success"
  }
}
```

**Notes**:
- `transactionState`: `RETURNED` indicates a successful refund.
- `transactionStatus`: `APPROVED` confirms the refund was processed.

#### Curl Example
```bash
curl -X POST https://prod.emea.api.fiservapps.com/sandbox/ipp/payments-gateway/v2//payments/838123456789 \
  -H "Content-Type: application/json" \
  -H "Api-Key: YOUR_API_KEY" \
  -H "Client-Request-Id: 550e8400-e29b-41d4-a1b2-3f4544d0000" \
  -H "Timestamp: 1695792000000" \
  -H "Message-Signature: GENERATED_SIGNATURE" \
  -d '{
    "requestType": "PaymentCardReturnTransaction",
    "transactionAmount": {
      "total": 12.04,
      "currency": "EUR"
    }
  }'
```

### 4. Sync / Psync

This flow retrieves the current state of a transaction, useful for synchronization or status checks.

- **Endpoint URL**: `/payments/{transaction-id}`
- **HTTP Method**: GET
- **Required Headers**:
  - `Api-Key: YOUR_API_KEY`
  - `Client-Request-Id: UNIQUE_REQUEST_ID`
  - `Timestamp: CURRENT_TIMESTAMP`
  - `Message-Signature: GENERATED_SIGNATURE`

#### Request Payload
No request body is required for GET requests.

#### Response Payload
```json
{
  "clientRequestId": "550e8400-e29b-41d4-a1b2-3f4544d0000",
  "apiTraceId": "rrt-1234567890-abcdef",
  "ipgTransactionId": "838123456789",
  "orderId": "ORDER12345",
  "transactionType": "PREAUTH",
  "transactionState": "AUTHORIZED",
  "paymentMethodDetails": {
    "paymentCard": {
      "expiryDate": {
        "month": "12",
        "year": "24"
      },
      "bin": "542418",
      "last4": "1732",
      "brand": "MASTERCARD"
    }
  },
  "transactionAmount": {
    "total": 12.04,
    "currency": "EUR"
  },
  "transactionTime": 1695792000,
  "approvedAmount": {
    "total": 12.04,
    "currency": "EUR"
  },
  "transactionStatus": "APPROVED",
  "approvalCode": "123456",
  "processor": {
    "responseCode": "00",
    "responseMessage": "Success"
  }
}
```

**Notes**:
- Replace `{transaction-id}` with the `ipgTransactionId`.
- The response mirrors the structure of other transaction responses, providing the current `transactionState` (e.g., `AUTHORIZED`, `CAPTURED`, `RETURNED`).

#### Curl Example
```bash
curl -X GET https://prod.emea.api.fiservapps.com/sandbox/ipp/payments-gateway/v2//payments/838123456789 \
  -H "Api-Key: YOUR_API_KEY" \
  -H "Client-Request-Id: 550e8400-e29b-41d4-a1b2-3f4544d0000" \
  -H "Timestamp: 1695792000000" \
  -H "Message-Signature: GENERATED_SIGNATURE"
```

### 5. Dispute Handling

The provided OpenAPI specification does not include specific endpoints or processes for dispute handling.

- **Endpoint URL**: Not specified.
- **HTTP Method**: N/A
- **Required Headers**: N/A
- **Request Payload**: N/A
- **Response Payload**: N/A

**Notes**:
- Dispute handling is likely managed outside the API, possibly through Authipay’s merchant portal, manual processes, or direct communication with their support team.
- For detailed dispute resolution procedures, contact Authipay support at `ipg-online@fiserv.com` or refer to the official documentation at https://docs.fiserv.dev/public/docs/payments-getting-started.

### 6. Tokenization / Vaulting

This flow tokenizes card details, enabling secure storage for future transactions without retaining sensitive card data.

- **Endpoint URL**: `/payment-tokens`
- **HTTP Method**: POST
- **Required Headers**:
  - `Content-Type: application/json`
  - `Api-Key: YOUR_API_KEY`
  - `Client-Request-Id: UNIQUE_REQUEST_ID`
  - `Timestamp: CURRENT_TIMESTAMP`
  - `Message-Signature: GENERATED_SIGNATURE`

#### Request Payload
```json
{
  "requestType": "PaymentCardPaymentTokenizationRequest",
  "paymentCard": {
    "number": "4035874000424977",
    "expiryDate": {
      "month": "12",
      "year": "25"
    },
    "securityCode": "977"
  },
  "createToken": {
    "reusable": true,
    "declineDuplicates": false
  }
}
```

**Notes**:
- `requestType`: `PaymentCardPaymentTokenizationRequest` for card tokenization.
- `createToken.reusable`: Set to `true` for tokens usable in multiple transactions.
- `createToken.declineDuplicates`: Set to `false` to allow duplicate tokens (optional).

#### Response Payload
```json
{
  "clientRequestId": "550e8400-e29b-41d4-a1b2-3f4544d0000",
  "apiTraceId": "rrt-1234567890-abcdef",
  "requestStatus": "SUCCESS",
  "paymentToken": {
    "value": "1235325235236",
    "reusable": true,
    "last4": "4977",
    "brand": "VISA",
    "type": "PAYMENT_CARD"
  },
  "paymentCard": {
    "number": "4035874000424977",
    "expiryDate": {
      "month": "12",
      "year": "25"
    }
  }
}
```

**Notes**:
- `paymentToken.value`: The generated token for future transactions.
- `requestStatus`: `SUCCESS` indicates successful tokenization.
- **Supported Tokens**: Reusable tokens for recurring payments, with optional duplicate decline.

#### Curl Example
```bash
curl -X POST https://prod.emea.api.fiservapps.com/sandbox/ipp/payments-gateway/v2//payment-tokens \
  -H "Content-Type: application/json" \
  -H "Api-Key: YOUR_API_KEY" \
  -H "Client-Request-Id: 550e8400-e29b-41d4-a1b2-3f4544d0000" \
  -H "Timestamp: 1695792000000" \
  -H "Message-Signature: GENERATED_SIGNATURE" \
  -d '{
    "requestType": "PaymentCardPaymentTokenizationRequest",
    "paymentCard": {
      "number": "4035874000424977",
      "expiryDate": {
        "month": "12",
        "year": "25"
      },
      "securityCode": "977"
    },
    "createToken": {
      "reusable": true,
      "declineDuplicates": false
    }
  }'
```

## Configuration & Setup

### Required Configuration Parameters
- **API Key**:
  - DescriptionDescription: Unique key provided by Authipay for API authentication.
  - Source: Obtained via the Fiserv Developer Portal.
- **Secret Key**:
  - DescriptionDescription: Used for generating the `Message-Signature`.
  - Source: Provided alongside the API Key.
- **Store ID** (Optional):
  - DescriptionDescription: An outlet ID for merchants with multiple stores, used in specific endpoints (e.g., `StoreIdParam`).
  - Format: String, max length 20 (e.g., `12345500000`).

### Environment-Specific Variables
- **Sandbox Environment**:
  - URL: https://prod.emea.api.fiservapps.com/sandbox/ipp/payments-gateway/v2
  - Usage: For testing and development without affecting live transactions.
- **Production Environment**:
  - URL: https://prod.emea.api.fiservapps.com/sandbox/ipp/payments-gateway/v2/
  - Usage: For live transaction processing.

### Supported Currencies, Countries, Card Networks, and Payment Methods
- **Currencies**:
  - Supports ISO 4217 currency codes (e.g., `EUR`, `USD`).
  - Specific supported currencies are not listed in the documentation documentation but are implied to cover major global currencies.
- **Countries**:
  - Not explicitly specified; depends on merchant configuration and Authipay’s operational regions.
  - Likely includes major markets (e.g., EU, US) based on the API’s EMEA focus.
- **Card Networks**:
  - Supported brands (inferred from schema examples):
    - VISA
    - MASTERCARD
    - AMEX
    - MAESTRO
    - RUPAY
- **Payment Methods**:
  - Primary Focus: Credit and debit card payments (`PaymentCard`).
  - Other Methods: The API supports additional methods (e.g., SEPA, wallets like Apple Pay/Google Pay), but this documentation focuses exclusively on card payments as per the requirements.

## Additional Information

### PCI-Enabled Integration

To comply with PCI DSS requirements, Authipay supports encrypting sensitive card data using the `PaymentCardProtected` object, reducing the merchant’s PCI compliance scope by avoiding plain-text card data transmission.

#### Steps for PCI-Enabled Integration
1. **Obtain Encryption Key**:
   - Request an encryption key and key ID from Authipay via their developer portal or support team.
   - Ensure the key supports the required algorithms (e.g., `DUKPT2009`, `AES128CBC`).
2. **Encrypt Card Data**:
   - Convert card details (e.g., card number, expiry date, security code) into a UTF-8 JSON block.
   - Apply padding (e.g., Padded80 standard).
   - Encrypt the padded JSON block using the provided key and algorithm.
   - Encode the encrypted data in Base64.
3. **Include Encrypted Data in Requests**:
   - Replace plain `paymentCard` objects with `paymentCardProtected` in API requests.
   - Include the encryption key details (e.g., `index`, `name`, `version`, `derivationAlgo`, `encryptionAlgo`).
4. **Validate Integration**:
   - Test the encrypted requests in the sandbox environment to ensure correct decryption by Authipay.
   - Confirm compliance with Authipay’s support team.

#### Example Request with Encrypted Data
```json
{
  "requestType": "PaymentCardPreAuthTransaction",
  "transactionAmount": {
    "total": 12.04,
    "currency": "EUR"
  },
  "paymentMethod": {
    "paymentCardProtected": {
      "encryptedData": "BASE64_ENCODED_ENCRYPTED_DATA",
      "key": {
        "index": "KSN_VALUE",
        "name": "KEY_NAME",
        "version": "1.0",
        "derivationAlgo": "DUKPT2009",
        "encryptionAlgo": "AES128CBC"
      }
    }
  }
}
```

**Notes**:
- The `encryptedData` field contains the Base64-encoded result of the encryption process.
- The `key` object specifies the encryption parameters, ensuring Authipay can decrypt the data.
- For detailed encryption guidelines, refer to Authipay’s official documentation or contact support at `ipg-online@fiserv.com`.

### Error Handling
Authipay’s API returns standard HTTP status codes and error responses for various scenarios. Below are common error codes and their meanings:

- **400 Bad Request**:
  - Indicates invalid request parameters or malformed JSON.
  - Response Example:
    ```json
    {
      "error": {
        "code": "INVALID_REQUEST",
        "message": "Invalid or missing parameters"
      }
    }
    ```
- **401 Unauthenticated**:
  - Invalid or missing API Key.
- **403 Unauthorized**:
  - API Key lacks permission for the requested operation.
- **404 Not Found**:
  - Invalid endpoint or transaction ID.
- **409 Transaction Gateway Declined**:
  - Transaction declined by the gateway (e.g., insufficient funds).
- **422 Transaction Endpoint Declined**:
  - Transaction declined by the processor.
- **500 Server Error**:
  - Internal server error on Authipay’s side.
- **502 Endpoint Communication Error**:
  - Issue communicating with downstream systems.

**Recommendation**: Implement robust error handling in your integration to parse these responses and provide meaningful feedback to users.

### Additional Notes
- **Message-Signature Generation**: The exact process for generating the `Message-Signature` is critical for secure requests. Since the provided documentation lacks specifics, developers must consult Authipay’s official resources or support for the HMAC SHA256 algorithm and concatenation rules.
- **Testing in Sandbox**: Use the sandbox URL for all testing to avoid impacting live transactions. Ensure test card numbers (e.g., `5424180279791732`) are used, as provided in the documentation examples.
- **Support Contact**: For any integration issues, contact Authipay support at `ipg-online@fiserv.com` or visit https://fiserv.dev/support.

## Conclusion

This documentation provides a complete guide for integrating Authipay with Hyperswitch, focusing on card payment processing. It covers all required API flows (authorize, capture, refund, sync, tokenization), authentication setup, configuration parameters, and PCI compliance steps. For further details or clarifications, refer to the official Authipay documentation at https://docs.fiserv.dev/public/docs/payments-getting-started or contact their support team.

**Last Updated**: June 2, 2025
```