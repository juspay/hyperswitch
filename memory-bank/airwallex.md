Airwallex Payment Method Integration Guide: Cards, Digital Wallets, and Local Bank Transfers
I. Introduction to Airwallex Payment Integration
Purpose and Scope of This Guide
This guide provides a comprehensive technical overview for integrating various payment methods through the Airwallex platform. It focuses specifically on Cards, Google Pay, Skrill, iDEAL, Indonesian Bank Transfer, PayPal, Klarna, Trustly, BLIK, and Atome. The content is meticulously crafted for software developers, technical leads, and solutions architects, aiming to deliver actionable information on leveraging Airwallex's APIs, SDKs, e-commerce plugins, and hosted page integration options. The report aims to clarify technical prerequisites, configuration steps, and critical compliance considerations essential for successful implementation.

The information presented is systematically derived from key sections of the Airwallex documentation. This includes the "Platform APIs" section, with particular emphasis on "Payments" and "Payouts" for programmatic transaction management. Insights are also drawn from "Embedded Finance Use Cases," which covers solutions like "Payments for Platforms," and the "Developer Tools" section, encompassing "Developer Essentials," "API," "SDKs," "Webhooks," and the "Sandbox environment". These foundational documentation areas are crucial for understanding Airwallex's approach to payment acceptance and integration.

Overview of Airwallex Payment Solutions and Integration Options
Airwallex functions as a unified API platform, offering a predictable and flexible integration experience for developers seeking to embed comprehensive financial flows into their applications. Its suite of products spans global business accounts, payment acceptance, and spend management, all accessible through a single interface.

Merchants leveraging Airwallex can accept payments from major card schemes and a diverse array of other payment forms from customers worldwide. All transaction proceeds are seamlessly settled into a multi-currency Airwallex Wallet. This centralized settlement mechanism allows businesses to efficiently utilize funds for international transfers, card issuance, or withdrawal to local bank accounts.

Airwallex offers several distinct payment products tailored to various business models:

Online Payments: This product is specifically designed for businesses operating websites or mobile applications. It facilitates the acceptance of payments via cards, e-wallets, and numerous local payment methods. Online Payments integration can range from minimal code implementations for rapid deployment to highly customized user experiences, providing flexibility to meet diverse business needs.

Payment Links: For merchants without a traditional digital storefront, Payment Links offer a straightforward way to accept payments online. These links can be generated with a few clicks and shared via URLs or QR codes on invoices, social media, emails, or websites. This feature supports over 160 payment methods, including cards, and can be created manually via the web application or automated through APIs for large-scale operations.

Invoice Integrations: This solution streamlines payment collection for invoices generated through popular accounting and invoicing software, such as Xero and NetSuite. It supports payments via cards and over 30 local payment methods, enhancing global payment capabilities for businesses.

In-person Payments (Digital Devices): Airwallex also supports in-person payment collection through digital devices like phones or tablets. Customers can input their payment information or scan QR codes to complete transactions. It is important to note that traditional Point-of-Sale (POS) terminals are not currently supported by Airwallex.

General Integration Principles (API, SDKs, Webhooks, Sandbox)
Airwallex's integration framework is built upon robust principles that ensure flexibility and security across various implementation scenarios. The core of its offerings revolves around a REST-based API, complemented by a suite of developer tools and pre-built components.

Native API Integration: This approach offers developers the highest degree of control over the payment flow. It involves direct interaction with Airwallex's API endpoints, allowing for highly customized user experiences and backend logic. This method is typically chosen by organizations with significant development resources and specific requirements for their checkout process.

Mobile SDKs: Airwallex provides Software Development Kits (SDKs) for both iOS and Android platforms. These SDKs enable seamless integration of Airwallex's payment modules directly into mobile applications. They offer native UI pages that streamline the payment process, guiding users from payment method selection to information submission and external app redirections. A significant advantage of these SDKs is their automatic handling of 3D Secure authentication, which adapts to card issuer requirements for either frictionless or challenge flows.

Hosted Payment Page (HPP): The Hosted Payment Page is an integration option where shoppers are redirected to a secure, pre-built payment page managed and hosted by Airwallex. This approach simplifies payment acceptance by supporting multiple payment methods through a single integration. A key benefit of HPP is its substantial reduction in the merchant's PCI-DSS compliance burden, as Airwallex handles the collection and storage of sensitive payment details, often requiring only a PCI-DSS SAQ A questionnaire from the merchant.

Embedded Elements: For businesses seeking a balance between customization and ease of integration, Airwallex offers pre-built UI components known as "Elements." The "Drop-in Element" provides a comprehensive, full-featured checkout module that simplifies the acceptance of various payment methods. For more granular control, individual elements like the Card Element, Split Card Element, Apple Pay Button Element, and Google Pay Button Element can be embedded, allowing for a more tailored user experience within the merchant's own interface.

E-commerce Plugins: To facilitate quick setup and activation of payment methods for businesses using popular e-commerce platforms, Airwallex offers ready-to-use plugins. Examples include integrations for Shopify and WooCommerce, which streamline the process of offering a wide range of payment options to shoppers.

The diverse range of payment solutions and integration methods offered by Airwallex illustrates a strategic approach to addressing the varied technical capabilities and operational models of its merchant base. By providing options from full programmatic control via Native APIs to simplified, no-code solutions like Payment Links and e-commerce plugins, Airwallex ensures that businesses of all sizes can efficiently integrate its payment capabilities. This modularity allows Airwallex to serve a broader market by reducing the technical barriers to entry for smaller businesses while still offering the customization and flexibility required by larger enterprises.

Furthermore, the varying PCI DSS compliance requirements associated with different integration methods represent a significant strategic consideration for merchants. For instance, Native API integration necessitates PCI-DSS Level 1 certification, implying a substantial investment in security infrastructure and ongoing audits. Conversely, solutions like the Hosted Payment Page and Mobile SDKs significantly reduce this compliance burden, often requiring only a PCI-DSS SAQ A questionnaire. This differentiation means that the choice of integration method is not merely a technical preference but a critical business decision that directly impacts a merchant's security posture, operational costs, and overall risk management. Airwallex strategically leverages these compliance distinctions to guide merchants toward solutions that align with their existing security capabilities and their appetite for managing compliance overhead.

II. Core Airwallex Integration Concepts
Airwallex API Structure and Authentication (API Keys, OAuth Scopes)
Airwallex's API is fundamentally built on REST principles, offering a structured and adaptable integration experience for developers. This design promotes predictable interactions and simplifies the process of embedding financial functionalities into diverse applications.

API Endpoints:
Airwallex maintains distinct API endpoints for different operational environments:

Sandbox Environment: The endpoint https://api-demo.airwallex.com/api/v1/ is designated for testing and development purposes. This environment allows developers to thoroughly test their integrations without affecting live data or incurring actual transaction costs, providing a safe space for development and debugging.

Production Environment: The live endpoint for processing real transactions is https://api.airwallex.com/api/v1/. Integrations are migrated to this environment once testing in the sandbox is complete and validated.

Authentication:
To interact with any Airwallex API endpoint, authentication is required. This process involves obtaining an access token, which serves as a credential for subsequent API calls. The access token is acquired by authenticating with a unique Client ID and API key, both of which are accessible within the Airwallex Web Application.

API Key Management:
Airwallex offers two primary types of API keys, each with distinct access levels and recommended use cases, emphasizing security:

Admin API Key: This key provides comprehensive access to all Airwallex APIs. It is generated through the Airwallex web app (accessible via Account > Developer > API keys). Upon generation, the API key must be copied and stored securely immediately, as it cannot be viewed again from the web application. Admin API keys can be regenerated if their security is compromised or if a periodic refresh is required.

Restricted API Key: Designed for enhanced security, restricted API keys grant limited access to specific Airwallex APIs. Developers can define the scope of permissions, such as 'Edit' or 'View' access, for each key. These keys are particularly recommended for microservices architectures, where limiting the blast radius of a compromised key is critical. This approach minimizes risk by ensuring that a key only has the necessary permissions for its specific function.

Security Best Practices for API Keys:
Airwallex provides clear guidelines for securing API keys to prevent unauthorized access and potential breaches:

Principle of Least Privilege: When creating restricted API keys, it is strongly advised to enable only the minimal set of access permissions required for that key's specific use case. This reduces the potential impact if a key is compromised.

Secure Storage: API keys should be stored securely using dedicated password managers or privileged access management (PAM) systems. They should never be transmitted over insecure or general-purpose communication channels like email, SMS, or instant messaging applications.

Regeneration: If an API key is suspected of being inappropriately handled, viewed, or compromised, it must be regenerated immediately. Developers should be mindful of the operational impact of regenerating an API key that is actively in use within their systems.

Code Integration: API keys should not be hardcoded directly into source code files or committed to version control systems. Instead, they should be retrieved dynamically at runtime using environment variables, user input, or secure APIs provided by password and secret management systems.

OAuth Scopes:
Beyond API keys, certain integration types and functionalities within Airwallex necessitate specific OAuth scopes. These scopes represent permissions granted to an application, allowing it to access particular resources or perform designated actions on behalf of a user or merchant. Examples include:

Bill Payments: Requires scopes such as r:awx_action:settings.account_details_view, r:awx_action:balances_view, w:awx_action:contact_management_edit, and w:awx_action:transfers_edit to manage account details, view balances, and create payouts.

Payment Link Generation: Requires r:awx_action:payment_links_view and w:awx_action:payment_links_edit to create and manage payment links.

Online Payment Acceptance: Requires w:awx_action:pa_edit and r:awx_action:pa_view for facilitating online payment transactions.

Bank Feed Integration: Primarily requires r:awx_action:balances_view to access balance history for accounting purposes.

Sandbox Environment and Testing Framework
Airwallex provides a robust sandbox environment, accessible via api-demo.airwallex.com/api/v1/, which is crucial for developers to conduct thorough testing of their integrations without impacting live production data or incurring real costs. This dedicated testing ground ensures that all functionalities, from basic transactions to complex webhook interactions, can be validated before deployment to the production environment.

Sandbox Setup:
To initiate testing, developers must first set up their sandbox account. This involves logging into the sandbox Airwallex web application to generate test API keys and obtain a unique client ID necessary for making API calls. For multi-region deployments, it is important to note that multiple sandbox API keys may be required to cover all operational areas. Additionally, developers should configure risk settings and webhook configurations within the sandbox environment to accurately simulate the behavior expected in a live production setting. This preparatory step is vital for ensuring that the test environment mirrors production as closely as possible, allowing for realistic simulations of payment flows and error handling.

Test Card Numbers:
Airwallex offers a comprehensive suite of test card numbers specifically designed to simulate a wide array of payment scenarios. These include successful transactions, various types of decline reasons, and different outcomes for 3D Secure (3DS) authentication flows. The flexibility of these test cards is notable; they function effectively with any CVC (Card Verification Value) and any future expiry date, simplifying the testing process. Specific test card numbers are provided to simulate frictionless and challenge 3DS flows, as well as to trigger declines due to reasons such as suspected fraud, insufficient funds, or invalid card numbers. This granular control over test scenarios allows developers to validate their integration's resilience and error handling capabilities comprehensively.

Google Pay Testing:
Testing Google Pay integration within the Airwallex sandbox environment is facilitated using a real card in conjunction with demo API keys. It is important to understand that payments made with real cards in the demo environment will not result in actual charges. Airwallex maps the real card used during testing to specific test card numbers based on its brand (e.g., Visa, Mastercard, JCB, American Express) to simulate various outcomes. This mapping ensures that the test environment accurately reflects how different card brands behave in a live setting.

Developers can test various Google Pay scenarios, including successful transactions, issuer declines (e.g., due to an amount of $80.51), incorrect payment token encryption, and authentication declines in both frictionless and challenge modes. The sandbox also allows for testing scenarios where transactions are blocked by Airwallex's risk system (e.g., for AUD currency with an amount of $20.41). It is crucial to note that Apple Pay's sandbox environment is not directly supported by Airwallex; instead, testing relies on real cards with demo API keys. Furthermore, Apple Pay on the web is exclusively functional on Safari web browsers, both desktop and mobile.

Webhooks for Asynchronous Event Handling
Webhooks are a critical component of the Airwallex integration framework, enabling the platform to send instant, real-time push notifications to a merchant's application whenever specific events occur within their Airwallex account. This mechanism is particularly valuable for handling asynchronous events, where the outcome of an action is not immediately available after an API call, such as payment confirmations or chargebacks.

Configuration and Subscriptions:
To receive webhook notifications, merchants must subscribe to desired events by registering a notification URL. When a subscribed event is triggered in the merchant's account (or any account they are authorized to access), Airwallex dispatches a notification to the configured URL. These notifications are delivered as a JSON payload via HTTP POST requests.

The configuration process is managed through the Airwallex web application:

Log into the Airwallex web app.

Navigate to the Developer > Webhooks > Summary page.

Click Add Webhook to specify the notification URL and select the events for which notifications are desired.

Developers can preview the JSON payload structure for any webhook event by clicking on it.

A "Test event" button in the demo environment allows for testing webhook events against the specified notification URL, facilitating development and debugging.

Delivery Headers:
HTTP POST payloads sent to the webhook's configured URL endpoint include specific headers that provide important metadata for verification:

x-timestamp: This header contains a Long type timestamp, such as 1357872222592.

x-signature: This header is present if the webhook is configured with a secret. It contains the HMAC hex digest of the response body, which is generated using the SHA-256 hash function and the configured secret as the HMAC key.

Responding to Webhook Events:
It is imperative for the merchant's endpoint to acknowledge receipt of webhook notifications. This is done by returning a 200 HTTP status code. If Airwallex does not receive a 

200 response or receives a different status code, it will retry sending the notification multiple times over a period of three days. To prevent timeouts, it is recommended to acknowledge events immediately by returning the 

200 status code before executing any complex business logic associated with the event.

Checking Webhook Signatures:
Airwallex signs all webhook events it sends by including a signature in each request's header. This mechanism allows merchants to verify that the events genuinely originated from Airwallex and have not been tampered with.

The signature verification process involves several steps:

Retrieve the unique endpoint secret from the Airwallex web app. Each endpoint has its own secret.

Extract the x-timestamp and x-signature values from the request header.

Construct a value_to_digest string by concatenating the x-timestamp (as a string) and the raw JSON payload (the request's body, as a string).

Compute an HMAC using the SHA-256 hash function, with the endpoint's signing secret as the key and the value_to_digest string as the message.

Compare the computed signature with the x-signature received in the header. If they match, it confirms authenticity. Additionally, calculate the difference between the current timestamp and the received timestamp to ensure it falls within an acceptable tolerance, guarding against replay attacks. A common cause for signature mismatches is using a formatted JSON payload instead of the raw payload for signature computation.

Tips for Using Webhooks (Best Practices):

HTTPS Server: Always use an HTTPS URL for webhook endpoints for security, ensuring the server is correctly configured for HTTPS.

Retry Logic: Airwallex implements retry logic for failed deliveries, resending notifications multiple times over three days until a successful response is received.

Idempotency: Webhook endpoints may occasionally receive duplicate events. Implement idempotent processing using the "id" field of the event to deduplicate and ensure that processing the same event multiple times does not lead to unintended side effects.

Event Order: Airwallex does not guarantee the delivery order of events. Applications should not rely on a specific sequence and should handle events accordingly. The created_at field can be used for ordering if necessary.

IP Whitelisting: To ensure successful receipt of webhook calls, merchants must whitelist Airwallex's outgoing IP addresses. For the production environment, these include 35.240.218.67, 35.185.179.53, 34.87.64.173, 35.220.213.251, 34.92.128.176, 34.91.47.254, 34.91.75.229, 35.230.185.215, 34.86.42.60. For the demo environment, the IPs are 35.240.211.132, 35.187.239.216, 34.87.139.23, 34.92.48.104, 34.92.144.250, 34.92.15.70.

PCI DSS Compliance and Data Handling
PCI DSS (Payment Card Industry Data Security Standard) compliance is a critical consideration for any business processing card payments. Airwallex's integration options are designed with varying levels of PCI DSS responsibility for the merchant, allowing businesses to choose an integration path that aligns with their compliance capabilities and risk appetite.

Compliance Requirements by Integration Type:

Native API Integration: This method offers maximum control over the checkout experience but places the highest PCI DSS burden on the merchant. Partners opting for Native API integration must be PCI-DSS Level 1 certified and are required to provide a Report on Compliance (ROC) to Airwallex. This indicates that the merchant is directly handling sensitive cardholder data and must maintain a robust security environment to meet the stringent requirements of PCI DSS Level 1.

Hosted Payment Page (HPP) and Mobile SDKs: These integration options significantly reduce the merchant's PCI DSS compliance scope. When using HPP, Airwallex fully handles the collection and storage of shopper payment details, thereby minimizing the merchant's responsibility. This often means that a PCI-DSS SAQ A questionnaire is sufficient for compliance. Similarly, for card payments processed through Airwallex Mobile SDKs (both iOS and Android), merchants are typically required to complete a PCI-DSS SAQ A questionnaire and renew it regularly. This approach allows merchants to accept card payments without the extensive infrastructure and audit requirements of full PCI DSS certification.

Importance of Comprehensive Payment Data for Fraud Protection:
Beyond compliance, providing comprehensive payment data is crucial for enhancing fraud protection and optimizing authorization rates. Airwallex strongly advises passing detailed information to maximize success rates and mitigate risks. This includes:

Customer Information: Email address and customer name.

Product Information: Details about the goods or services being purchased.

Shipping Information: Delivery address details.

Billing Information: Billing address details.


Collecting billing address information, for instance, enables the leverage of Address Verification Service (AVS), a tool for fraud prevention. Providing this granular data allows Airwallex's risk management systems to perform more accurate assessments, thereby reducing the likelihood of fraudulent transactions and improving the overall authorization success rate for legitimate payments.

III. Payment Method Specific Integration Details
This section details the integration specifics for each requested payment method, including their properties, supported integration methods, and unique considerations.

Cards
Airwallex enables merchants to accept payments from major international card schemes, including Visa, Mastercard, American Express, JCB, UnionPay, Discover, and Diners Club. Beyond basic payment acceptance, Airwallex offers a full suite of merchant services, encompassing authentication, risk and fraud prevention, acceptance rate optimization, and a centralized platform for other Airwallex products.

Key Properties of Card Payments:

Payment Type: Card.

Available for Businesses Registered In: AU, CH, EU, HK, JP (Beta), MY (Beta), SG, NZ, UK, US (varies slightly by card brand).

Activation Time for Onboarding: Instant.

Shopper Regions: Global.

Minimum/Maximum Transaction Amount: Not specified, implying flexibility.

Recurring Payments: Supported (✅) across all major card brands, with UnionPay supporting credit cards only.

Refunds/Partial Refunds: Supported (✅).

Disputes: Supported (✅).

Placing a Hold (Delayed/Manual Capture): Supported (✅).

Integration Checklist and Considerations:
The integration process for cards involves distinct phases for sandbox testing and production deployment.

Sandbox Environment Testing:

Setup: Configure a sandbox account and utilize Airwallex's provided test card numbers to simulate various success and error scenarios. These test cards are flexible, working with any CVC and future expiry date.

API Keys: Generate test API keys and obtain the unique client ID from the sandbox Airwallex web app. Multiple keys may be needed for multi-region setups.

Account Configuration (Mandatory - via Account Manager): Add relevant Merchant Category Codes (MCCs), configure desired payment methods, set 3DS preferences (FORCE_3DS, EXTERNAL_3DS, SKIP_3DS), and define principal/fallback settlement currencies.

PCI DSS Certification: For Native API integration, PCI-DSS Level 1 certification is required, with proof submitted to Airwallex. For other payment acceptance options (excluding Pay by Link), a PCI-DSS SAQ A questionnaire must be completed if not Level 1 certified.

Web App Configuration: Configure risk settings and webhooks within the sandbox web app. Webhooks are essential for receiving real-time notifications on transaction outcomes (e.g., success, authorization failure, 3DS authentication failure, refunds).

Transaction Tests: Use sandbox API endpoints (api-demo.airwallex.com/api/v1/) to create specific transaction types and validate responses. This includes successful 3DS frictionless/challenge flows, failed 3DS authentication, and authorization failures.

Reporting: Download transaction and settlement reports from the sandbox environment to understand data availability for reporting, or use sandbox API endpoints for programmatic report download.

Production Environment Deployment:

API Keys: Generate production API keys from the live Airwallex web app, noting the need for multiple keys for multi-region deployments.

Account Configuration: Work with an Airwallex Account Manager to replicate sandbox settings in the production account.

Web App Configuration: Configure risk settings and webhooks in the production Airwallex web app, ensuring subscribed webhook events are received and managed.

Integration: Update API endpoints to production URLs (https://api.airwallex.com/api/v1/). Replicate sandbox transaction tests using nominal amounts and real cards to ensure live integration functionality.

Reporting: Update programmatic report downloads to production URLs if applicable.

Required Fields for API Integration:

Create Payment Intent:

request_id (string): A unique ID for the request. 

amount (number): The transaction amount. 

currency (string): The three-letter ISO currency code. 

merchant_order_id (string): Your unique order ID. 

return_url (string): URL to redirect the shopper after payment. 

Optional for enhanced fraud protection and authorization rates:

customer (object): Contains email and customer_name. 

order.products (array of objects): Details about items being purchased. 

shipping.address (object): Delivery address details. 

billing.address (object): Billing address details. 

payment_method_options.card.three_ds_action (string): Override global 3DS settings (FORCE_3DS, EXTERNAL_3DS, SKIP_3DS, null). 

Confirm Payment Intent:

intent_id (string): The ID of the Payment Intent. 

client_secret (string): The client secret of the Payment Intent. 

For saved payment methods/recurring payments:

customer_id (string): The ID of the customer. 

payment_consent_id (string): The ID of the payment consent. 

Google Pay
Google Pay allows shoppers to make payments using cards saved on their Google account or Android device wallet. Airwallex supports Google Pay integration via various methods, including Hosted Payment Page, Drop-in Element, Embedded Elements, Mobile SDK, and Native API.

Key Properties of Google Pay:

Payment Type: Digital Wallet / Tokenized Card.

Available for Businesses Registered In: Not explicitly detailed in snippets, but generally global where Google Pay is supported.

Activation Time for Onboarding: Not explicitly detailed in snippets, but activated if selected during onboarding.

Shopper Regions: Global where Google Pay is supported.

Processing/Settlement Currencies: Not explicitly detailed in snippets, but depends on underlying card support.

Recurring Payments: Supported via Payment Consent with Native API.

Refunds/Partial Refunds/Disputes/Placing a Hold: Dependent on underlying card support.

Integration Steps and Considerations:

Airwallex Activation: Google Pay is activated as a payment method if selected during the merchant onboarding process.

Google Integration Criteria:

(Optional) Complete Google's integration checklist for Google Pay APIs.

(Optional) Register on the Google Business console. This step is only required for Native API integration.

Refer to Google's web integration developer documentation and brand guidelines.

Airwallex Gateway Details: When setting up Google registration, merchants must provide Airwallex's gateway information for card tokenization: gateway: airwallex and gatewayMerchantId: Your merchant account open ID as provided by Airwallex.

Enabling via Airwallex Web App: Google Pay can be enabled under Payments > Settings in the Airwallex web app. A prerequisite is that the merchant account must be configured for online payments. Merchants must also accept Google Pay's terms and conditions.

Android SDK Integration for Google Pay:
For Android applications, the integration involves several technical steps :

Install Google Pay Component: Add the necessary Gradle dependency (io.github.airwallex:payment-googlepay) to the app-level build.gradle file.

Configure the SDK: Initialize the Airwallex SDK, enabling logging for debugging and setting the environment (e.g., Environment.DEMO or Environment.PRODUCTION). Include GooglePayComponent.PROVIDER in the list of supported component providers.

Setup Google Pay on SDK: Merchants must complete registration with Google and request production access. Crucially, merchants should not fill information under the direct integration section on the Google Pay API business console, as Airwallex manages this on their behalf. Configure the SDK with required 

GooglePayOptions, such as allowedCardAuthMethods (e.g., CRYPTOGRAM_3DS) and billingAddressParameters.

Create a PaymentIntent: The client app requires a PaymentIntent to form a payment session. This PaymentIntent should be created from the merchant's server using the Airwallex API and then passed to the client app.

Present Payment Sheet: Use the presentPaymentFlow method with an AirwallexSession object to display the Google Pay payment sheet to the shopper.

Native API Integration for Google Pay:
Native API integration provides full front-end control but requires handling interactions with Google directly. This includes identifying the payment processor in the TokenizationSpecification message to Google (gateway: airwallex, gatewayMerchantId: AWX_ACCT_OPEN_ID). For mobile-based transactions via Native API, merchants typically need their own Google Pay Gateway ID and certificates, whereas Airwallex certificates can be used for web-based Google Pay payments. The encrypted payment token received from Google Pay must be included in the 

Confirm PaymentIntent API call.

Required Fields for API Integration:

Create Payment Intent:

request_id (string): A unique ID for the request. 

amount (number): The transaction amount. 

currency (string): The three-letter ISO currency code. 

merchant_order_id (string): Your unique order ID. 

return_url (string): URL to redirect the shopper after payment. 

Confirm Payment Intent:

payment_method.type (string): Must be "googlepay". 

googlepay.payment_data_type (string): Type of payment data, e.g., "encrypted_payment_token". 

googlepay.encrypted_payment_token (string): The encrypted payment token received from Google Pay. 

For Native API integration with Google's LoadPaymentData:

tokenizationSpecification.type: "PAYMENT_GATEWAY" 

tokenizationSpecification.parameters.gateway: "airwallex" 

tokenizationSpecification.parameters.gatewayMerchantId: Your Airwallex account open ID (e.g., acct_xxxx). 

Create Payment Consent (for recurring payments):

request_id (string): A unique ID for the request. 

customer_id (string): The ID of the customer. 

payment_method.type (string): Must be "googlepay". 

payment_method.id (string): The ID of the payment method. 

next_triggered_by (string): E.g., "merchant". 

merchant_trigger_reason (string): E.g., "scheduled". 

metadata.schedule (string): E.g., "1st of month". 

Google Pay Button Element (createElement options):

type: "googlePayButton" 

intent_id (string): The ID of the Payment Intent. 

client_secret (string): The client secret. 

amount.value (string): The amount value. 

amount.currency (string): The currency. 

countryCode (string): Two-letter ISO-3166 country code. 

Optional: allowCreditCards, allowPrepaidCards, allowedAuthMethods (e.g., PAN_ONLY, CRYPTOGRAM_3DS), allowedCardNetworks, appearance, assuranceDetailsRequired, authFormContainer. 

Skrill
Skrill is a widely used e-wallet payment method supported by Airwallex, enabling online payments and fund transfers globally.

Key Properties of Skrill:

Payment Type: E-Wallet.

Available for Businesses Registered In: HK, AU, SG, UK, EU, US.

Activation Time for Onboarding: 3 business days.

Shopper Regions: Global (except blacklisted countries).

Processing Currencies: EUR, GBP, USD.

Settlement Currencies: Like-for-like or default settlement currency.

Settlement Schedule: 3 business days after payment capture.

Minimum Transaction Amount: Quick Checkout: 0.50 EUR; 1-Tap: 0.001 EUR.

Maximum Transaction Amount: Quick Checkout: 10,000.00 EUR (adjustable); 1-Tap: No limitation.

Checkout Session Timeout: 1 hour.

Recurring Payments: Not supported (⛔).

Refunds/Partial Refunds: Supported (✅).

Disputes (Chargebacks): Supported (✅).

Placing a Hold (Delayed/Manual Capture): Not supported (⛔).

Payments for Platforms Support: Supported (✅).

Supported Integration Methods for Skrill:
Airwallex offers various integration methods for Skrill, allowing businesses to choose based on their technical capabilities and desired user experience :

Online payments via your own website/app: Hosted Payment Page (✅), Drop-in Element (✅), Embedded Elements (✅), Mobile SDK (✅), Native API (✅). Subscription APIs are not supported (⛔).

Online payments via e-commerce plugins: Shopify (✅), WooCommerce (✅), Shopline (✅). ShopLazza (⛔) and Magento (⛔) are not supported.

Payment links & Invoice Integrations: Payment Links (✅). Xero Invoice is not supported (⛔).

Required Fields for API Integration:

Create Payment Intent:

request_id (string): A unique ID for the request. 

amount (number): The transaction amount. 

currency (string): The three-letter ISO currency code. 

merchant_order_id (string): Your unique order ID. 

return_url (string): URL to redirect the shopper after payment. 

Optional for enhanced fraud protection and authorization rates:

order.products (array of objects): Details about items being purchased. 

shipping.first_name (string), shipping.last_name (string), shipping.address (object): Shipping address details. 

Confirm Payment Intent:

payment_method.type (string): Must be "skrill". (Inferred from general API patterns for payment methods )

iDEAL
iDEAL is a prominent online banking payment method in the Netherlands, providing a payment guarantee to merchants and enabling consumers to pay directly through their mobile banking app or online bank account. It is based on the SEPA credit transfer system and is the most widely used payment method in the Netherlands.

Key Properties of iDEAL:

Payment Type: Online Banking.

Available for Businesses Registered In: HK*, AU, SG, UK, EU, US.

Activation Time for Onboarding: Instant.

Shopper Regions: Netherlands (NL).

Processing Currencies: EUR.

Settlement Currencies: Default settlement currency.

Settlement Schedule: T+3 business days, subject to reserve plans. Settlements below 100.00 EUR may be delayed.

Minimum Transaction Amount: 0.01 EUR.

Maximum Transaction Amount: Subject to the owner's bank account.

Checkout Session Timeout: 1 hour at bank selection page, 15 minutes at selected bank's page.

Recurring Payments: Not supported (⛔).

Refunds/Partial Refunds: Supported (✅) within 365 days.

Disputes (Chargebacks): Not supported (⛔).

Placing a Hold (Delayed/Manual Capture): Not supported (⛔).

Supported Integration Methods for iDEAL:
Airwallex provides various client-side integration methods for iDEAL, allowing merchants to manage their UI and minimize implementation effort :

Online payments via your own website/app: Hosted Payment Page (✅), Drop-in Element (✅), Embedded Elements (✅), Mobile SDK (✅).

Online payments via e-commerce plugins: Shopify (✅), WooCommerce (✅), Shopline (✅), Magento (✅).

Payment links & Invoice Integrations: Payment Links (✅).

API Integration Note: If integrating iDEAL through API, merchants must comply with the branding and logo use guidance provided by iDEAL.

Required Fields for API Integration:

Create Payment Intent:

request_id (string): A unique ID for the request. 

amount (number): The transaction amount. 

currency (string): The three-letter ISO currency code (e.g., "EUR"). 

merchant_order_id (string): Your unique order ID. 

return_url (string): URL to redirect the shopper after payment. 

Optional for enhanced fraud protection and authorization rates:

order.products (array of objects): Details about items being purchased. 

shipping.first_name (string), shipping.last_name (string), shipping.phone_number (string), shipping.address (object): Shipping address details. 

Confirm Payment Intent:

payment_method.type (string): Must be "ideal". (Inferred from general API patterns for payment methods )

Create Customer (for saving bank details, if applicable for other direct debit methods):

request_id (string): A unique ID for the request. 

merchant_customer_id (string): Your internal customer ID. 

email (string): Customer's email address. 

phone_number (string): Customer's phone number. 

first_name (string): Customer's first name. 

last_name (string): Customer's last name. 

Create Payment Consent (for saving bank details, if applicable for other direct debit methods):

request_id (string): A unique ID for the request. 

customer_id (string): The ID of the customer. 

next_triggered_by (string): E.g., "merchant". 

merchant_trigger_reason (string): E.g., "unscheduled". 

currency (string): Optional, the three-letter ISO currency code. 

Verify Payment Consent (for saving bank details, if applicable for other direct debit methods):

payment_method.bacs_direct_debit.verification_method (string): E.g., "truelayer". 

owner_name (string): Owner's name. 

owner_email (string): Owner's email. 

payment_method.bacs_direct_debit.account_number (string): Bank account number. 

payment_method.bacs_direct_debit.sort_code (string): Bank sort code. 

payment_method.bacs_direct_debit.bank_name (string): Bank name. 

payment_method.bacs_direct_debit.address (object): Address details. 

Indonesian Bank Transfer
Indonesian Bank Transfer Payments provide access to Indonesian shoppers, allowing them to receive a payment code on the merchant page and complete payments via mobile banking apps or ATMs.

Key Properties of Indonesian Bank Transfer:

Supported Banks: Bank Mandiri, Bank Danamon, CIMB Niaga, Permata, MayBank, Bank Rakyat Indonesia (BRI), BNI.

Payment Type: Bank Transfer.

Available for Businesses Registered In: HK, AU, SG, UK, US.

Activation Time for Onboarding: 7-14 business days.

Shopper Regions: Indonesia (ID).

Processing Currencies: Indonesian Rupiah (IDR).

Settlement Currencies: USD or default settlement currency if USD is not supported.

Settlement Schedule: T+8 business days, subject to reserve plans.

Minimum Transaction Amount: 10,000.00 IDR.

Maximum Transaction Amount: 6,600,000.00 IDR.

Checkout Session Timeout: 79 hours.

Recurring Payments: Not supported (⛔).

Refunds/Partial Refunds: Not supported (⛔).

Disputes (Chargebacks): Not supported (⛔).

Placing a Hold (Delayed/Manual Capture): Not supported (⛔).

Descriptor: Customer's statement will display "MOLPay", "MOLP", "MOLPay EC", or "NetBuilder EC".

Payments for Platforms Support: Supported (✅).

Supported Integration Methods for Indonesian Bank Transfer:
Airwallex offers various client-side integration methods for Indonesian Bank Transfer :

Online payments via your own website/app: Hosted Payment Page (✅), Drop-in Element (✅), Embedded Elements (✅), Mobile SDK (✅), Native API (✅). Subscription APIs are not supported (⛔).

Online payments via e-commerce plugins: Shopify (✅), WooCommerce (✅), Shopline (✅), ShopLazza (✅). Magento (⛔) is not supported.

Payment links & Invoice Integrations: Payment Links (✅). Xero Invoice (⛔) is not supported.

Required Fields for API Integration:

Create Payment Intent:

request_id (string): A unique ID for the request. 

amount (number): The transaction amount. 

currency (string): The three-letter ISO currency code (e.g., "IDR"). 

order.products (array of objects): Details about items being purchased. 

shipping.first_name (string), shipping.last_name (string), shipping.address (object): Shipping address details. 

merchant_order_id (string): Your unique order ID. 

return_url (string): URL to redirect the shopper after payment. 

Confirm Payment Intent:

payment_method.type (string): Must be "indonesian_bank_transfer". (Inferred from general API patterns for payment methods )

PayPal
PayPal is a widely recognized e-wallet that provides a secure checkout experience and access to a vast global customer base. Airwallex acts as an acquiring partner, facilitating PayPal integration into online stores. PayPal's functionalities vary based on the merchant's registration region.

Key Properties of PayPal (Managed Path for US Merchants):

Payment Type: E-Wallet.

Available for Businesses Registered In: US (Managed Path). HK, AU*, SG, UK, EU, NZ (Connected Path - requires external PayPal account).

Activation Time for Onboarding: Instant - 7 days.

Shopper Regions: Global.

Processing Currencies: AUD, BRL*, CAD, CZK, DKK, EUR, HKD, HUF*, JPY*, MYR*, MXN, NOK, NZD, PHP, PLN, GBP, RUB, SGD, SEK, CHF, TWD*, THB, USD.

Settlement Currencies (Managed Path): Dependent on processing currency; like-for-like for major currencies, others settled in USD or default settlement currency.

Settlement Schedule (Managed Path): 1 business day after payment capture.

Transaction Limits (Wallet/Cards): No limit. Buy Now Pay Later: 30.00-1,500.00 USD (Pay in 4, US), 45.00-2,000.00 GBP (Pay in 3, UK), 30.00-2,000.00 EUR (Pay in 4, EU).

Checkout Session Timeout: 6 hours.

Recurring Payments: Supported (✅) with Native API only.

Refunds/Partial Refunds: Supported (✅).

Disputes (Chargebacks): Supported (✅).

Placing a Hold (Delayed/Manual Capture): Supported (✅).

Descriptor: Your registered business name.

Payments for Platforms Support: Supported (✅).

Integration Methods for PayPal:

Online payments via your own website/app: Hosted Payment Page (✅), Drop-in Element (✅), Embedded Elements (✅), Mobile SDK (✅), Native API (✅).

Online payments via e-commerce plugins: WooCommerce (✅), Shopline (✅). Shopify (⛔), ShopLazza (⛔), Magento (⛔) are not supported.

Payment links & Invoice Integrations: Payment Links (✅), Xero Invoice (✅).

Key Considerations:

US Merchants (Managed Path): Airwallex offers a fully managed model where PayPal transactions settle directly into the Airwallex wallet, and disputes are managed via the Airwallex web app or API. No separate PayPal account is required.

Other Regions (Connected Path): For merchants registered in HK, AU, SG, UK, EU, NZ, an external PayPal account must be set up and connected to Airwallex. Pricing and settlement terms are negotiated directly with PayPal, and transactions settle to the PayPal account, not the Airwallex wallet.

Recurring Payments: For recurring payments with Klarna, a PaymentIntent must be created. If it's a free trial, the amount can be set to zero. For fixed frequency payments, subscription type products should be added to the order with expected amounts. For other scenarios, dummy product information can be passed. Providing additional_info.customer_activity_data.purchase_summaries is highly recommended to improve success rates, as Klarna uses this for credit and risk evaluations.

Required Fields for API Integration:

Create Payment Intent:

request_id (string): A unique ID for the request. 

amount (number): The transaction amount. 

email (string): Shopper's email. 

phone (string): Shopper's phone number. 

currency (string): The three-letter ISO currency code. 

merchant_order_id (string): Your unique order ID. 

order.type (string): E.g., "v_goods". 

order.products (array of objects): Details about items being purchased. 

order.shipping (object): Shipping details. 

Confirm Payment Intent:

request_id (string): A unique ID for the request. 

payment_method.type (string): Must be "paypal". 

paypal.country_code (string): Shopper's country code. 

paypal.shopper_name (string): Shopper's name. 

payment_method_options.paypal.auto_capture (boolean): Set to true for auto-capture. 

For recurring payments: payment_consent_id (string): The ID of the payment consent. 

Create Payment Consent (for recurring payments):

request_id (string): A unique ID for the request. 

customer_id (string): The ID of the customer. 

next_triggered_by (string): E.g., "merchant". 

merchant_trigger_reason (string): E.g., "unscheduled". 

requires_cvc (boolean): Indicates if CVC is required. 

Verify Payment Consent (for recurring payments):

request_id (string): A unique ID for the request. 

payment_method.type (string): Must be "paypal". 

return_url (string): URL to redirect after verification. 

Klarna
Klarna is a popular "Buy Now Pay Later" (BNPL) payment method that allows shoppers to pay instantly, later (e.g., in 30 days), or in installments. Airwallex supports Klarna integration, providing merchants with access to a flexible payment option.

Key Properties of Klarna:

Payment Type: Buy Now Pay Later.

Available for Businesses Registered In: Not explicitly detailed in snippets, but widely available.

Activation Time for Onboarding: Not explicitly detailed in snippets.

Shopper Regions: Global, with specific eligible countries and currencies for different scenarios.

Processing Currencies: Not explicitly detailed in snippets.

Settlement Currencies: Not explicitly detailed in snippets.

Settlement Schedule: Not explicitly detailed in snippets.

Minimum/Maximum Transaction Amount: Not explicitly detailed in snippets.

Checkout Session Timeout: Not explicitly detailed in snippets.

Recurring Payments: Supported (✅).

Refunds/Partial Refunds: Supported (✅).

Disputes (Chargebacks): Not explicitly detailed in snippets.

Placing a Hold (Delayed/Manual Capture): Supported (✅).

Integration Steps and Considerations:

Shopify Plugin: Airwallex offers a dedicated Klarna Payments app for Shopify stores, enabling Klarna Slice It, Pay Now, and Pay Later options.

Prerequisites: A valid and activated Airwallex account with Klarna enabled as a payment method.

Installation: Install the "Airwallex Klarna Payments app" from Shopify Admin. Connect to your Airwallex account, ensuring Klarna is activated for the linked account.

Configuration: Set "Customer contact method" to "Email only" in Shopify settings, as Klarna requires an email for all transactions. Enable the Klarna option in Airwallex Klarna Payments app settings to display the logo on the checkout page.

Currency Switching: Enable currency switching in Airwallex settings (Payments > Settings) to improve success rates for shoppers using different currencies.

First Name Requirement: Klarna encourages passing the first name for all transactions; configure Shopify to "Require first name and last name" during checkout.

Testing: Enable test mode in the app settings to mock successful and failed transactions before going live.

Manual Capture: Klarna payments can be manually captured after authorization, which is recommended if goods or services are not immediately fulfilled. This feature is available with Native API or plugins. Manual capture must be performed within 13 days of authorization, either via the web app or API.

On-Site Messaging (OSM): Integrating Klarna OSM into websites or mobile apps can promote flexible payment options to shoppers at checkout. This requires contacting an Airwallex account manager or support.

Required Fields for API Integration:

Create Payment Intent:

request_id (string): A unique ID for the request. 

amount (number): The transaction amount. Set to 0 for free trials. 

currency (string): The three-letter ISO currency code. 

order.products (array of objects): Details about items being purchased. 

For fixed frequency payments, type should be "subscription" with unit_price. 

Otherwise, dummy product info can be passed. 

order.shipping (object): Shipping details. 

merchant_order_id (string): Your unique order ID. 

return_url (string): URL to redirect the shopper after payment. 

Strongly advised: additional_info.customer_activity_data.purchase_summaries (object): For shopper credit and risk evaluations. 

Confirm Payment Intent (using Laybuy by Klarna example):

request_id (string): A unique ID for the request. 

payment_method.type (string): Must be "laybuy" (or "klarna" for direct Klarna). 

laybuy.country_code (string): Shopper's country code. 

laybuy.language (string): Language preference. 

laybuy.billing (object): Billing details. 

date_of_birth (string): Shopper's date of birth. 

email (string): Shopper's email. 

first_name (string): Shopper's first name. 

last_name (string): Shopper's last name. 

phone_number (string): Shopper's phone number. 

address (object): Billing address. 

payment_method_options.laybuy.auto_capture (boolean): Set to false for manual capture (default). 

Trustly
Trustly is an online banking payment service provider that facilitates direct payments from consumers' bank accounts, emphasizing security and eliminating the risk of stolen details or fraud. Founded in Sweden, Trustly operates in 29 countries and integrates with over 3300 banks across Europe.

Key Properties of Trustly:

Payment Type: Online Banking.

Available for Businesses Registered In: HK, AU, SG, UK, EU, US.

Activation Time for Onboarding: Not explicitly detailed in snippets.

Shopper Regions: DE, DK, EE, ES, FI, GB, LV, LT, NL, PL, PT, SE, SK.

Processing Currencies: DKK, EUR, GBP, NOK, PLN, SEK.

Settlement Currencies: DKK, EUR, GBP, NOK, PLN, SEK.

Minimum Transaction Amount: 0.01 EUR (also payer bank dependent).

Maximum Transaction Amount: Dependent on payer bank.

Session Timeout: 7 days.

Recurring Payments: Not supported (⛔).

Refunds/Partial Refunds: Supported (✅) within 365 days.

Disputes/Chargebacks: Not supported (⛔).

Settlement Threshold: 100.00 EUR.

Settlement Frequency: Daily.

Supported Integration Methods for Trustly:
Airwallex offers various client-side integration methods for Trustly :

Online payments via your own website/app: Hosted Payment Page (✅), Embedded fields (✅), Drop-in elements (✅), API (✅), Mobile SDK (✅). Pay by Link (⛔) is not supported.

Online payments via e-commerce plugins: Shopify (✅), WooCommerce (✅), Magento (✅). Shopline (⛔) and ShopLazza (⛔) are not supported.

Payment links & Invoice Integrations: Payment Links (✅). Xero Invoice (⛔) is not supported.

Desktop/Mobile Website Browser Integration:
To accept Trustly payments on a website, the shopper is redirected to the Trustly payment page. The process involves:

Initialize a Payment Intent: Create a PaymentIntent object from the backend server with request_id, amount, currency (e.g., EUR, DKK, GBP, NOK, PLN, SEK), and merchant_order_id.

Redirect to Trustly: Call the POST /api/v1/pa/payment_intents/{id}/confirm API endpoint to obtain a URL. Redirect the shopper to this URL to complete the payment on the Trustly payment page. The request includes payment_method type as trustly and shopper_name and country_code (e.g., NL, DE, DK, EE, ES, FI, GB, LV, LT, NO, PL, PT, SE, SK).

Wait for Notification: Airwallex notifies the merchant of the payment result asynchronously via webhook API. Subscribing to payment_intent.succeeded is recommended.

Query PaymentIntent Status: The status of a payment can be queried anytime via the GET /payment_intents/{id} API.

Required Fields for API Integration:

Create Payment Intent:

request_id (string): A unique ID for the request. 

amount (number): The transaction amount. 

currency (string): The three-letter ISO currency code (DKK, EUR, GBP, NOK, PLN, SEK). 

merchant_order_id (string): Your unique order ID. 

return_url (string): URL to redirect the shopper after payment. 

Confirm Payment Intent:

request_id (string): A unique ID for the request. 

payment_method.type (string): Must be "trustly". 

trustly.shopper_name (string): Shopper's full name (e.g., "first_name last_name"). 

trustly.country_code (string): Shopper's country code (DE, DK, EE, ES, FI, GB, LV, LT, NL, NO, PL, PT, SE, SK). 

BLIK
BLIK is a mobile payment system widely used in Poland, allowing users to make payments directly from their mobile banking app using a unique 6-digit one-time code. This code is valid for 2 minutes and is used for payment authentication.

Key Properties of BLIK:

Payment Type: Online Banking.

Available for Businesses Registered In: HK, AU, SG, UK, EU, US.

Activation Time for Onboarding: Up to 3 business days.

Shopper Regions: Poland (PL).

Processing Currencies: PLN.

Settlement Currencies: Default settlement currency.

Settlement Schedule: T+8 business days, subject to reserve plans. Settlements below 100.00 EUR may be delayed.

Minimum Transaction Amount: 0.01 PLN.

Maximum Transaction Amount: 10,000.00 PLN.

Checkout Session Timeout: 55 seconds.

Recurring Payments: Not supported (⛔).

Refunds/Partial Refunds: Supported (✅) within 13 months.

Disputes (Chargebacks): Supported (✅).

Placing a Hold (Delayed/Manual Capture): Not supported (⛔).

Payments for Platforms Support: Supported (✅).

Supported Integration Methods for BLIK:
Airwallex offers various client-side integration methods for BLIK :

Online payments via your own website/app: Hosted Payment Page (✅), Drop-in Element (✅), Embedded Elements (✅), Mobile SDK (✅), Native API (✅). Subscription APIs are not supported (⛔).

Online payments via e-commerce plugins: Shopify (✅), WooCommerce (✅), Shopline (✅). ShopLazza (⛔) and Magento (⛔) are not supported.

Payment links & Invoice Integrations: Payment Links (✅). Xero Invoice (⛔) is not supported.

Required Fields for API Integration:

Create Payment Intent:

request_id (string): A unique ID for the request. 

amount (number): The transaction amount. 

currency (string): The three-letter ISO currency code (e.g., "PLN"). 

merchant_order_id (string): Your unique order ID. 

return_url (string): URL to redirect the shopper after payment. 

Confirm Payment Intent:

payment_method.type (string): Must be "blik". 

payment_method_options.blik.code (string): Your customer's 6-digit BLIK code. 

Optional: payment_method.billing_details (object): Billing details associated with the transaction. 

Atome
Atome is a "Buy Now Pay Later" (BNPL) payment method that allows shoppers to make purchases and pay in installments. It is a popular option in Southeast Asia.

Key Properties of Atome:

Payment Method Type: Buy Now Pay Later.

Available for Businesses Registered In: HK, AU, UK, SG, EU.

Activation Time for Onboarding: 2-3 business days.

Shopper Regions: SG, MY.

Processing Currencies: SGD, MYR.

Settlement Currencies: SGD, USD.

Settlement Schedule: T+4 business days, subject to reserve plans.

Minimum Transaction Amount: 1.5 SGD, 10 MYR.

Maximum Transaction Amount: Up to credit limit. For SG shoppers, 1000 SGD (debit card) and 3000 SGD (credit card). Higher limits can be requested.

Checkout Session Timeout: 12 hours.

Recurring Payments: Not supported (⛔).

Refunds/Partial Refunds: Supported (✅) within 60 days.

Disputes (Chargebacks): Not supported (⛔).

Placing a Hold (Delayed/Manual Capture): Not supported (⛔).

Payments for Platforms Support: Supported (✅).

Supported Integration Methods for Atome:
Airwallex offers various client-side integration methods for Atome :

Online payments via your own website/app: Hosted Payment Page (✅), Drop-in Element (✅), Embedded Elements (✅), Mobile SDK (✅), Native API (✅). Subscription APIs are not supported (⛔).

Online payments via e-commerce plugins: Shopify (✅), WooCommerce (✅). Shopline (⛔), ShopLazza (⛔), and Magento (⛔) are not supported.

Payment links & Invoice Integrations: Payment Links (✅). Xero Invoice (⛔) is not supported.

Required Fields for API Integration:

Create Payment Intent:

request_id (string): A unique ID for the request. 

amount (number): The transaction amount. 

currency (string): The three-letter ISO currency code (e.g., "SGD"). 

order.products (array of objects): Details about items being purchased. Required for risk scanning. 

order.shipping.address (object): Shipping address details. Required for risk scanning. 

merchant_order_id (string): Your unique order ID. 

return_url (string): URL to redirect the shopper after payment. 

Confirm Payment Intent:

request_id (string): A unique ID for the request. 

payment_method.type (string): Must be "atome". 

atome.shopper_phone (string): Shopper's phone number in E.164 format (e.g., +6580001500). 

IV. Conclusion
The analysis of Airwallex's documentation reveals a highly flexible and comprehensive payment integration ecosystem designed to cater to a wide spectrum of business needs. The platform's modular approach, offering various integration methods such as Native API, Mobile SDKs, Hosted Payment Pages, Embedded Elements, and e-commerce plugins, demonstrates a clear understanding of diverse technical capabilities and operational requirements across different merchant segments. This strategic breadth allows businesses to select an integration path that best aligns with their internal resources, desired level of customization, and specific market demands.

A significant factor influencing the choice of integration method is the varying PCI DSS compliance burden. Airwallex intelligently structures its offerings to either offload or simplify compliance responsibilities for merchants, depending on the chosen integration. For instance, the stringent PCI-DSS Level 1 certification required for Native API integration contrasts sharply with the reduced scope (often SAQ A) for Hosted Payment Pages and Mobile SDKs. This differentiation is not merely a technical detail but a critical business consideration, directly impacting a merchant's security infrastructure investment, ongoing operational costs, and overall risk management strategy. Businesses can therefore make informed decisions that balance technical control with compliance overhead.

Furthermore, Airwallex provides a robust developer experience, characterized by a dedicated sandbox environment for thorough testing, comprehensive API documentation, and a sophisticated webhook system for asynchronous event handling. The availability of specific test card numbers for various scenarios, including 3D Secure authentication outcomes, underscores Airwallex's commitment to enabling rigorous pre-production validation. The emphasis on secure API key management and the detailed guidance for webhook implementation, including signature verification and best practices for idempotency and event ordering, collectively contribute to a secure and reliable integration framework.

In summary, Airwallex's payment integration capabilities are characterized by their adaptability, security, and developer-centric design. The platform's ability to support a broad range of payment methods across diverse regions, coupled with its flexible integration options and robust developer tools, positions it as a powerful solution for businesses seeking to optimize their global payment acceptance strategies while effectively managing technical complexity and regulatory compliance.