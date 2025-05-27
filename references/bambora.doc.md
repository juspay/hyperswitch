Integrating Bambora Payment Gateway with Hyperswitch
Overview: Bambora offers a robust RESTful API for card payment processing, supporting features like authorization, capture, refunds, and tokenization, making it a suitable candidate for integration with Hyperswitch’s open-source payments switch.
Key Features: Supports major card networks (Visa, Mastercard, Amex), 3D Secure, tokenization, and callbacks for transaction updates, though detailed webhook documentation is limited.
Implementation: Developers can integrate Bambora by configuring API credentials, mapping payment flows to Hyperswitch’s traits, and handling callbacks, with some details requiring verification from Bambora’s support.
Uncertainty: Information on rate limits, full currency/country support, and detailed webhook payloads is not fully available in public documentation, so developers should consult Bambora’s developer portal or support team.
Getting Started
To integrate Bambora with Hyperswitch, you’ll need to obtain a Merchant ID and Payments API Passcode from the Bambora dashboard. These credentials allow you to make secure API calls to process card payments. The integration involves mapping Bambora’s API endpoints to Hyperswitch’s connector architecture, ensuring compatibility with Rust-based systems.

Key Steps
Configure Credentials: Set up your Merchant ID and API Passcode in Hyperswitch’s configuration file.
Implement Payment Flows: Use Bambora’s RESTful API to handle authorization, capture, refunds, and tokenization.
Handle Callbacks: Set up a callback URL to receive transaction updates and verify them using an MD5 hash.
Test Thoroughly: Use Bambora’s test card numbers to simulate transactions before going live.
Next Steps
Refer to Bambora’s developer portal for detailed API specifications and contact their support for specifics on currencies, countries, and rate limits. Cross-reference with Hyperswitch’s connector documentation to ensure alignment with its architecture.

Comprehensive Technical Integration Guide for Bambora in Hyperswitch
Overview
Connector Name: Bambora
Website: Bambora Developer Portal
API Documentation: Bambora Payment APIs
Description: Bambora is a payment gateway that provides a RESTful API for processing card payments, tokenization, and recurring billing. It supports a variety of payment flows, including authorization, capture, refunds, and dispute handling, making it a versatile choice for integration into Hyperswitch, an open-source payments switch written in Rust. Bambora’s API is designed to reduce PCI compliance scope through tokenization and supports 3D Secure for enhanced security. This guide outlines the technical details for a production-ready integration, focusing on card payment processing.
Supported Features

Feature	Supported	Notes
Card Payments	Yes	Supports Visa, Mastercard, Amex, Discover, and other major card networks.
3DS Authentication	Yes	Optional; requires setup with bank and Bambora support for Verified by Visa, Mastercard SecureCode, or Amex SafeKey.
Tokenization	Yes	Supports single-use and multi-use tokens for recurring payments.
Refunds	Yes	Full and partial refunds supported via API.
Disputes/Chargebacks	Yes	Dispute management available through dedicated API endpoints.
Webhooks	Yes	Implemented as callbacks to a merchant-specified URL with transaction details and MD5 hash verification.
Authentication
Method: API Key (Passcode)
Credentials Required:
Merchant ID: A unique identifier for your Bambora account.
Payments API Passcode: A secure key for API authentication.
Setup Instructions:
Log in to the Bambora dashboard.
Navigate to Administration > Account Settings > Order Settings to find the Merchant ID (displayed in the top right corner) and Payments API Passcode in the Payment Gateway section.
In Hyperswitch, update the configuration file (e.g., development.toml) with these credentials under the Bambora connector settings.
Example:
bash

curl -H "Authorization: Passcode Base64Encoded(300200578:4BaD82D9197b4cc4b70a221911eE9f70)" \
     -H "Content-Type: application/json" \
     https://api.na.bambora.com/v1/payments
The Authorization header uses a Base64-encoded string of merchant_id:passcode.
API Specifications
Base URL: https://api.na.bambora.com/v1
Supported Endpoints:
Authorize/Payment Intent Creation:
URL: /payments
Method: POST
Request: JSON payload with payment details, including payment_method, order_number, amount, and card details.
Response: JSON with transaction ID, status, and other details.
Example Request:
json

{
  "payment_method": "card",
  "order_number": "orderNum000112",
  "amount": 100.0,
  "card": {
    "name": "John Doe",
    "number": "5100000010001004",
    "expiry_month": "12",
    "expiry_year": "18",
    "cvd": "123"
  }
}
Notes: Requires full billing address (name, phone, street, city, state/province, postal code, country) for transactions.
Capture:
URL: /payments/{transId}/complete
Method: POST
Request: JSON with the amount to capture.
Response: JSON with updated transaction details.
Notes: Used for completing pre-authorized transactions; transId is obtained from the authorization response.
Refund:
URL: /payments/{transId}/returns
Method: POST
Request: JSON with the refund amount (full or partial).
Response: JSON with refund transaction details.
Notes: Supports both referenced (tied to a transaction) and unreferenced refunds.
Sync/Payment Status:
URL: /payments/{transId}
Method: GET
Response: JSON with current transaction status.
Notes: Used to check the status of a payment.
Dispute Handling:
URL: /disputes
Method: GET/POST
Request/Response: JSON with dispute details.
Notes: Specific dispute handling processes may require additional documentation from Bambora.
Tokenization:
URL: /tokens
Method: POST
Request: JSON with card details.
Response: JSON with a token code for single-use or multi-use payments.
Example Request:
json

{
  "number": "4030000010001234",
  "expiry_month": "02",
  "expiry_year": "20",
  "cvd": "123"
}
Rate Limits: Not specified in public documentation. Developers should contact Bambora support for details.
Error Codes: Bambora provides bank-related response codes (e.g., response.message and response.message_id). Refer to Bambora’s response codes for details.
Payment Flows
Supported Flows:
Direct Authorization: Process card payments directly.
Tokenization-Based Authorization: Use single-use or multi-use tokens for payments.
Flow Details:
Authorization:
Steps:
Send a POST request to /payments with card details, amount, and payment_method: "card".
Handle the response to obtain the transaction ID and status.
Example:
json

{
  "payment_method": "card",
  "order_number": "orderNum000112",
  "amount": 100.0,
  "card": {
    "name": "John Doe",
    "number": "5100000010001004",
    "expiry_month": "12",
    "expiry_year": "18",
    "cvd": "123"
  }
}
Capture:
Steps:
Use the transaction ID from the authorization response.
Send a POST request to /payments/{transId}/complete with the capture amount.
Refund:
Steps:
Send a POST request to /payments/{transId}/returns with the refund amount.
For unreferenced refunds, use /payments/0/returns.
Dispute Handling:
Steps: Use the /disputes endpoint to retrieve or manage dispute information.
Tokenization:
Steps:
Send a POST request to /tokens to generate a token.
Use the token in a /payments request with payment_method: "token".
3DS Handling:
Requires enabling 3D Secure through the merchant’s bank and Bambora support.
Send a POST request to /payments with a 3d_secure object containing browser details and enabled flag.
Currency Unit: Bambora uses minor units (e.g., cents for USD). Hyperswitch’s get_currency_unit function should convert amounts accordingly.
Webhooks
Supported: Yes, via callbacks.
Event Types: Transaction status updates (e.g., payment succeeded, refund processed).
Payload Structure: JSON with transaction details (e.g., transaction ID, status) and an MD5 hash for verification.
Signature Verification:
Concatenate callback URL parameter values (excluding the hash) and append the MD5 key.
Calculate the MD5 hash and compare it with the provided hash.
Setup Instructions:
In the Bambora dashboard, configure the callback URL under Administration > Account Settings.
In Hyperswitch, implement a webhook handler to process incoming callbacks and verify the MD5 hash.
If the callback fails, Bambora retries hourly for 24 hours.
Configuration
Required Parameters:
Merchant ID: Obtained from the Bambora dashboard.
Payments API Passcode: Found in the Payment Gateway section of the dashboard.
Supported Currencies: Supports approximately 150 currencies, including USD and CAD. The exact list is not publicly documented; verify with Bambora support.
Supported Countries: Primarily North America (Canada, USA); confirm additional countries with Bambora.
Supported Card Networks: Visa, Mastercard, Amex, Discover, and others; check Bambora’s documentation for the full list.
Additional Settings:
Idempotency Keys: Include in API requests to prevent duplicate transactions.
Custom Metadata: Optional fields like billing and shipping details can be included in payment requests.
Transaction Validation: Ensure “Restrict Internet Transaction Processing Types” is set to allow both purchase and pre-authorization in the Bambora dashboard.
Hyperswitch Compatibility
To integrate Bambora with Hyperswitch, developers must implement the ConnectorCommon and ConnectorIntegration traits:

ConnectorCommon: Define connector metadata (e.g., name, supported currencies) and authentication logic.
ConnectorIntegration: Implement methods for payment flows (e.g., authorize, capture, refund) by mapping to Bambora’s API endpoints.
Rust Implementation: Use Rust’s HTTP client (e.g., reqwest) to make API calls to Bambora’s endpoints, handling JSON serialization and Base64 encoding for authentication.
Bambora Connector for Hyperswitch
rust
Show inline
Notes on Missing Information
Supported Currencies and Countries: The full list is not publicly available. Developers should consult Bambora’s developer portal or contact support.
Rate Limits: Not documented publicly; verify with Bambora support.
Webhook Payload Example: Limited details available; check Bambora’s dashboard for sample payloads.
Dispute Handling Details: Specific processes may require additional documentation from Bambora.
Recommendations
Cross-Reference with Hyperswitch: Review Hyperswitch’s add_connector.md documentation on GitHub to ensure compliance with connector standards.
Test Environment: Use Bambora’s test card numbers (available at Bambora’s test cards) for sandbox testing.
Contact Support: For missing details (e.g., rate limits, webhook payloads), reach out to Bambora support or check community forums like Stack Overflow.
Key Citations
Bambora Payment APIs Overview
Bambora Developer Portal
Bambora Gateway Setup Guide – Advanced Billing
Bambora Payment API Response Codes
Bambora Test Card Numbers