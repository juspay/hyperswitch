Key Points
Maxpay Overview: Maxpay is a payment gateway that supports secure online card payments, offering features like 3D Secure, tokenization, and chargeback protection, making it suitable for integration with Hyperswitch.
Integration Feasibility: It seems likely that Maxpay can be integrated into Hyperswitch by implementing standard payment flows (authorization, capture, refund) and webhook handling, aligning with Hyperswitch’s connector architecture.
Technical Requirements: Developers will need Maxpay API keys or merchant credentials, and the integration must handle JSON-based APIs, webhook signatures, and 3DS flows.
Potential Challenges: Limited documentation on rate limits and error codes may require contacting Maxpay support for clarification.
Next Steps: Follow Hyperswitch’s connector guidelines and verify Maxpay’s specific requirements with their support team.
Integration Summary
Maxpay is an international payment gateway that supports card payments, 3D Secure authentication, tokenization, refunds, and webhooks. Integrating it into Hyperswitch, an open-source payments switch written in Rust, involves implementing its API endpoints and webhook handling to align with Hyperswitch’s ConnectorIntegration trait. The process requires obtaining API credentials from Maxpay’s dashboard, configuring payment flows, and ensuring secure webhook processing.
Steps to Integrate
Obtain Credentials: Sign up on Maxpay’s website and retrieve API keys or merchant credentials from the dashboard.
Implement API Calls: Use Maxpay’s API endpoints for authorization, capture, and refunds, ensuring proper signature generation.
Handle Webhooks: Configure a secure webhook URL in Maxpay’s dashboard and verify signatures in Hyperswitch.
Test and Deploy: Use Maxpay’s test environment with provided test cards, then switch to live mode after review.
Considerations
Documentation Gaps: Some details, like rate limits, are unclear, so developers should reach out to Maxpay support.
Hyperswitch Alignment: Ensure Maxpay’s flows match Hyperswitch’s expected payment processing structure.

Maxpay Payment Gateway Integration for Hyperswitch
Overview
Connector Name: Maxpay
Website: Maxpay
API Documentation: Maxpay Docs
Description: Maxpay is an international payment gateway service provider designed to facilitate secure online payments for businesses, particularly in high-risk industries. It offers robust API integration for processing credit and debit card payments, supports 3D Secure authentication, tokenization for recurring payments, and chargeback protection. Maxpay’s compliance with standards like PCI DSS and PSD2 ensures secure transactions, making it a viable candidate for integration into Hyperswitch, an open-source payments switch written in Rust. This guide provides a detailed roadmap for developers to integrate Maxpay as a new connector, focusing on card payment processing.
Supported Features
Feature
Supported
Notes
Card Payments
Yes
Supports major card networks (Visa, Mastercard, American Express)
3DS Authentication
Yes
Supports AUTH3D and SALE3D flows for enhanced security
Tokenization
Yes
Enables one-click payments using billToken for recurring transactions
Refunds
Yes
Supports full refunds via /api/refund endpoint
Disputes/Chargebacks
Yes
Provides chargeback tracking and protection services
Webhooks
Yes
Supports transaction status updates via Callback 1.0 (form-urlencoded) and Callback 2.0 (JSON)

Authentication
Method:
Hosted Payment Pages (HPP): Uses Public Key and Private Key for authentication.
Host to Host API: Requires Merchant Account and Merchant Password.
Credentials Required:
HPP: Public Key and Private Key, available in the Maxpay dashboard under Payment pages -> General -> API keys.
Host to Host: Merchant Account (6-32 characters) and Merchant Password (6-32 characters), obtained from the Maxpay dashboard.
Setup Instructions:
Sign up at Maxpay and create a merchant application.
Navigate to the Maxpay dashboard to retrieve API keys (for HPP) or Merchant Account and Password (for Host to Host).
Configure these credentials in Hyperswitch’s configuration file, typically development.toml, under the connector settings.
For signature generation, use the Private Key to create a SHA256 hash of request parameters, as described in the Maxpay documentation.
Example:
 bash
Copy
curl -X POST https://gateway.maxpay.com/api/cc \
-H "Content-Type: application/json" \
-d '{"merchant_account":"your_account","merchant_password":"your_password","transactionType":"AUTH"}'


Maxpay Connector Configuration Example
toml
Show inline
API Specifications
Base URL:
Live: Maxpay Gateway
Test: Maxpay Sandbox
Supported Endpoints:
Authorize/Payment Intent Creation:
URL: /api/cc
Method: POST
Request:
 json
Copy
{
  "merchant_account": "your_account",
  "merchant_password": "your_password",
  "transactionType": "AUTH",
  "amount": 100,
  "currency": "USD",
  "card_number": "4111111111111111",
  "card_expiry": "12/2025",
  "card_cvv": "123"
}


Response:
 json
Copy
{
  "transactionId": "hppS1513841180.7902mId3126aId1335",
  "reference": "SLFF00000006CAAC0F1B",
  "status": "success",
  "code": 0
}


Notes: Required fields include merchant_account, merchant_password, amount, and currency. Card details should not be stored on the merchant server for security.
Capture:
URL: /api/cc
Method: POST
Request:
 json
Copy
{
  "merchant_account": "your_account",
  "merchant_password": "your_password",
  "transactionType": "SETTLE",
  "reference": "SLFF00000006CAAC0F1B"
}


Response: Similar to authorization response.
Notes: Use after successful AUTH to capture funds.
Refund:
URL: /api/refund
Method: POST
Request:
 json
Copy
{
  "merchant_account": "your_account",
  "merchant_password": "your_password",
  "reference": "SLFF00000006CAAC0F1B",
  "amount": 100
}


Response: Similar to authorization response.
Notes: Supports full refunds; partial refunds may depend on acquirer.
Sync/Payment Status:
URL: /api/cc
Method: POST
Request:
 json
Copy
{
  "merchant_account": "your_account",
  "merchant_password": "your_password",
  "transactionType": "CHECK",
  "reference": "SLFF00000006CAAC0F1B"
}


Response: Returns transaction status.
Tokenization:
URL: /api/cc
Method: POST
Request:
 json
Copy
{
  "merchant_account": "your_account",
  "merchant_password": "your_password",
  "transactionType": "TOKENIZE",
  "card_number": "4111111111111111",
  "card_expiry": "12/2025",
  "card_cvv": "123"
}


Response:
 json
Copy
{
  "billToken": "9293f4vc-47fq-45d4-9eh2-08d29te9d899",
  "status": "success"
}


Preprocessing Flow: Not explicitly supported; assume standard authorization flow.
Cancel Flow: Use /api/cancel for subscription cancellations.
Rate Limits: Not specified in documentation; contact Maxpay Support for details.
Error Codes: Limited details available; common errors include invalid credentials or declined transactions (e.g., code 3100 for test declines).
Payment Flows
Supported Flows:
Direct Authorization (AUTH)
Tokenization-Based Authorization (using billToken)
3D Secure flows (AUTH3D, SALE3D)
Flow Details:
Authorization:
Steps:
Send AUTH request to /api/cc with card details and amount.
Receive transaction ID and reference for further actions.
Example:
 bash
Copy
curl -X POST https://gateway.maxpay.com/api/cc \
-H "Content-Type: application/json" \
-d '{"merchant_account":"your_account","merchant_password":"your_password","transactionType":"AUTH","amount":100,"currency":"USD"}'


Capture:
Steps: Send SETTLE request with the reference from AUTH.
Example: As shown in Capture endpoint.
Refund:
Steps: Send refund request with transaction reference.
Example: As shown in Refund endpoint.
Dispute Handling: Maxpay provides chargeback tracking; specific API endpoints are not detailed.
Tokenization:
Steps: Use TOKENIZE request to generate billToken for recurring payments.
Example: As shown in Tokenization endpoint.
3DS Handling:
Supports AUTH3D and SALE3D flows, requiring callback_url and redirect_url (HTTPS).
Dynamic 3D can transform SALE3D to SALE based on Maxpay rules.
Currency Unit: Uses ISO 4217 alpha-3 codes (e.g., USD, EUR). Align with Hyperswitch’s get_currency_unit function, typically in minor units.
Webhooks
Supported: Yes
Event Types: Transaction status updates (e.g., payment succeeded, refund processed, decline).
Payload Structure:
Callback 1.0 (application/x-www-form-urlencoded):
 text
Copy
transactionId=hppS1513841180.7902mId3126aId1335&reference=SLFF00000006CAAC0F1B&status=success&code=0&checkSum=91935c8e71dcac1473d6613e62e0d84d039a075971efd1e11fb4901001938a365d2bee325793a


Callback 2.0 (application/json):
 json
Copy
{
  "uniqueTransactionId": "hpp180926125439m7059a4040uf62e5bb29b97bc",
  "reference": "SLFF0000000040598D81",
  "status": "success",
  "code": 0
}


Signature Verification:
Method: SHA256 hash of callback parameters and private key.
Example (Callback 1.0):
 php
Copy
hash('sha256', 'billToken=9293f4vc-47fq-45d4-9eh2-08d29te9d899|code=0|...|your_private_key')


Setup Instructions:
Configure callback URL in Maxpay dashboard (Account settings -> Firewall -> Send new request).
Ensure HTTPS protocol for callback URL; self-generated SSL is not valid.
In Hyperswitch, implement webhook handling to parse payloads and verify checkSum or X_SIGNATURE.
Return HTTP 200 with body “OK” to acknowledge receipt.
Configuration
Required Parameters:
HPP: Public Key, Private Key, Callback URL
Host to Host: Merchant Account, Merchant Password, Callback URL
Supported Currencies: All ISO 4217 alpha-3 currencies (e.g., USD, EUR, GBP).
Supported Countries: Operates in 50+ jurisdictions, including EU, UK, and USA.
Supported Card Networks: Visa, Mastercard, American Express, and other standard networks.
Additional Settings:
Idempotency Keys: Not explicitly supported; assume standard request handling.
Custom Metadata: Include in custom product parameters (e.g., product details in JSON format).
Test Mode: Use test cards (e.g., Visa 2D: 4111111111111111, CVV 123, Expiry MM/2020+).
Going Live: Submit integration for review via Maxpay dashboard and whitelist production callback URL.
Hyperswitch Compatibility
Connector Traits: Hyperswitch connectors typically implement the ConnectorIntegration trait, which includes methods like make_payment, sync_payment, refund, and capture. Maxpay’s API aligns as follows:
make_payment: Map to /api/cc with AUTH or SALE transaction types.
sync_payment: Map to /api/cc with CHECK transaction type.
refund: Map to /api/refund.
capture: Map to /api/cc with SETTLE.
Implementation Notes:
Ensure Maxpay’s 3DS flows (AUTH3D, SALE3D) are compatible with Hyperswitch’s 3DS handling.
Map Maxpay’s billToken to Hyperswitch’s tokenization framework for recurring payments.
Handle webhook signatures using Hyperswitch’s webhook processing logic.
Code Structure:
Create a new module in Hyperswitch’s crates/router/src/connector directory (e.g., maxpay.rs).
Implement ConnectorCommon and ConnectorIntegration traits, defining Maxpay-specific logic for each payment flow.
Use Rust’s request crate for HTTP requests and sha2 for signature generation.
Edge Cases
Currency Unit Handling: Maxpay uses ISO 4217 alpha-3 codes. Hyperswitch typically expects minor units (e.g., cents for USD). Ensure conversion logic aligns with get_currency_unit.
3DS Requirements: Maxpay’s Dynamic 3D feature may transform SALE3D to SALE based on rules configured by Maxpay. Coordinate with Maxpay support to define rules.
Acquirer Dependency: Some features (e.g., conditional parameters like address) depend on the acquirer. Clarify requirements before implementation.
Test Mode: Add a “+” symbol to user_phone in test mode to avoid declines
