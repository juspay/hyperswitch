Spreedly Payment Gateway Integration for Hyperswitch
Overview
Spreedly: Spreedly, Inc.


Website: https://www.spreedly.com


API Documentation: https://developer.spreedly.com


Description: Spreedly is a global payments orchestration platform that enables businesses to connect with multiple payment gateways and services through a single API. It provides features like secure card vaulting, tokenization, and support for various payment methods, facilitating streamlined and secure payment processing.


Supported Features
Feature
Supported
Notes
Card Payments
Yes
Supports major card brands
3DS Authentication
Yes
Optional via SCA Providers
Tokenization
Optional
Automatic during transaction if raw card data is provided
Refunds
Yes
Full and partial refunds supported
Disputes/Chargebacks
No
Not directly handled
Webhooks
Yes
Supports event notifications

Authentication
Method: HTTP Basic Authentication over HTTPS


Credentials Required:


Environment Key: Acts as the username


Access Secret: Acts as the password


Setup Instructions:


Log in to the Spreedly dashboard.


Navigate to the desired environment or create a new one.


Obtain the Environment Key from the environment settings.


Generate an Access Secret for the environment.


Use these credentials for API authentication.


Example:


curl https://core.spreedly.com/v1/gateways.json \
  -u 'ENVIRONMENT_KEY:ACCESS_SECRET'
API Specifications
Base URL: https://core.spreedly.com/v1/


Authorize/Payment Intent Creation
URL: /v1/gateways/{gateway_token}/authorize.json


Method: POST


Request:


{
  "transaction": {
    "credit_card": {
      "number": "4111111111111111",
      "verification_value": "123",
      "month": "12",
      "year": "2025",
      "first_name": "John",
      "last_name": "Doe"
    },
    "amount": 1000,
    "currency_code": "USD"
  }
}




Response:


{
  "transaction": {
    "token": "transaction_token",
    "succeeded": true,
    "transaction_type": "Authorize",
    "amount": 1000,
    "currency_code": "USD",
    "payment_method": {
      "token": "payment_method_token"
    }
  }
}




Notes:


amount is in cents (minor units).


The credit_card object contains raw card data; Spreedly will tokenize it during the transaction.


Capture
URL: /v1/transactions/{transaction_token}/capture.json


Method: POST


Request:


{
  "transaction": {
    "amount": 1000
  }
}




Response:


{
  "transaction": {
    "token": "capture_transaction_token",
    "succeeded": true,
    "transaction_type": "Capture",
    "amount": 1000
  }
}




Notes:


transaction_token refers to the original authorization transaction.


Refund
URL: /v1/transactions/{transaction_token}/credit.json


Method: POST


Request:


{
  "transaction": {
    "amount": 500
  }
}




Response:


{
  "transaction": {
    "token": "refund_transaction_token",
    "succeeded": true,
    "transaction_type": "Credit",
    "amount": 500
  }
}




Notes:


Supports both full and partial refunds.


Sync/Payment Status
URL: /v1/transactions/{transaction_token}.json


Method: GET


Response:


{
  "transaction": {
    "token": "transaction_token",
    "succeeded": true,
    "transaction_type": "Authorize",
    "amount": 1000,
    "currency_code": "USD"
  }
}




Notes:


Retrieves the status and details of a specific transaction.


Rate Limits
General: 30 requests per minute per environment.


Error Codes
Common HTTP status codes:


200 OK: Successful request.


401 Unauthorized: Authentication failed.


422 Unprocessable Entity: Validation errors.


500 Internal Server Error: Server-side error.


Payment Flows
Supported Flows
Direct Authorization with raw card data


Authorization and Capture


Refunds


Flow Details
Authorization with Raw Card Data
Steps:


Submit the transaction with raw card details in the credit_card object.


Spreedly tokenizes the card and processes the authorization.


Example:


curl https://core.spreedly.com/v1/gateways/{gateway_token}/authorize.json \
  -u 'ENVIRONMENT_KEY:ACCESS_SECRET' \
  -H 'Content-Type: application/json' \
  -d '{
    "transaction": {
      "credit_card": {
        "number": "4111111111111111",
        "verification_value": "123",
        "month": "12",
        "year": "2025",
        "first_name": "John",
        "last_name": "Doe"
      },
      "amount": 1000,
      "currency_code": "USD"
    }
  }'
Capture
Steps:


Capture a previously authorized transaction using its token.


Example:


curl https://core.spreedly.com/v1/transactions/{transaction_token}/capture.json \
  -u 'ENVIRONMENT_KEY:ACCESS_SECRET' \
  -H 'Content-Type: application/json' \
  -d '{
    "transaction": {
      "amount": 1000
    }
  }'
Refund
Steps:


Issue a refund for a settled transaction using its token.


Example:


curl https://core.spreedly.com/v1/transactions/{transaction_token}/credit.json \
  -u 'ENVIRONMENT_KEY:ACCESS_SECRET' \
  -H 'Content-Type: application/json' \
  -d '{
    "transaction": {
      "amount": 500
    }
  }'
3DS Handling
Implementation:


Utilize the /v1/sca/providers/{sca_provider_key}/authenticate endpoint to perform 3DS authentication.


Requires integration with a supported SCA provider.


Currency Unit
Unit: Minor units (e.g., cents for USD).


Webhooks
Supported: Yes


Event Types:


transaction_succeeded


transaction_failed


payment_method_added


Payload Structure:


{
  "event": {
    "event_type": "transaction_succeeded",
    "transaction": {
      "token": "transaction_token",
      "amount": 1000,
      "currency_code": "USD"
    }
  }
}
Signature Verification:


Not explicitly documented; recommend verifying the source IP and using HTTPS.


Setup Instructions:


Log in to the Spreedly dashboard.


Navigate to the environment settings.



