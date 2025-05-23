# Structuring a New Hyperswitch Connector

This document outlines the standard structure and key components involved in integrating a new payment connector into the Hyperswitch platform. The information is derived from analyzing existing connectors like `stripebilling` and the comprehensive internal integration guides.

A new connector, let's call it `[ConnectorName]`, primarily consists of two Rust files within the `crates/hyperswitch_connectors/src/connectors/` directory:

1.  **Main Logic File (`[connector_name].rs`)**: Orchestrates the connector's interaction with Hyperswitch's core.
2.  **Transformers File (`[connector_name]/transformers.rs`)**: Handles connector-specific data structures and transformation logic.

## 1. Main Logic File (`[connector_name].rs`)

This file defines the connector's behavior and implements core Hyperswitch traits.

### Key Components:

*   **Module for Transformers:**
    ```rust
    pub mod transformers;
    // ... other use statements
    use transformers as [connector_name_module]; // e.g., use transformers as stripebilling;
    ```

*   **Connector Struct Definition:**
    A struct, often holding an `amount_converter`.
    ```rust
    #[derive(Clone)]
    pub struct [ConnectorNamePascalCase] {
        amount_converter: &'static (dyn AmountConvertor<Output = StringMinorUnit> + Sync),
    }

    impl [ConnectorNamePascalCase] {
        pub fn new() -> &'static Self {
            &Self {
                amount_converter: &StringMinorUnitForConnector, // Or another converter
            }
        }
    }
    ```

*   **Marker Trait Implementations:**
    Empty implementations indicating supported Hyperswitch flows (e.g., `api::Payment`, `api::PaymentAuthorize`, `api::RefundExecute`).

*   **`ConnectorCommonExt` Implementation:**
    Typically for building common request headers (`build_headers` method).

*   **`ConnectorCommon` Implementation:**
    Provides fundamental connector information:
    *   `id()`: Returns the snake_case name of the connector (e.g., `"connector_name"`).
    *   `get_currency_unit()`: Specifies `api::CurrencyUnit::Minor` or `api::CurrencyUnit::Base`.
    *   `common_get_content_type()`: Usually `"application/json"`.
    *   `base_url()`: Retrieves the base URL from configuration.
    *   `get_auth_header()`: Constructs authentication headers using the auth struct defined in `transformers.rs`.
    *   `build_error_response()`: Parses the connector's specific error response (defined in `transformers.rs`) and maps it to Hyperswitch's generic `ErrorResponse`.

*   **`ConnectorIntegration<Flow, RequestData, ResponseData>` Implementations:**
    This is where the logic for each specific payment/refund flow (Authorize, Capture, PSync, Refund Execute, Refund Sync, etc.) resides. For each flow:
    *   `get_headers()`: Defines request headers.
    *   `get_content_type()`: Defines request content type.
    *   `get_url()`: Constructs the specific API endpoint URL for the flow.
    *   `get_request_body()`: Transforms Hyperswitch's `RouterData` into the connector's specific request struct (from `transformers.rs`).
    *   `build_request()`: Assembles the `services::Request` object.
    *   `handle_response()`: Parses the connector's HTTP response into its specific response struct and transforms it back into Hyperswitch's `RouterData`.
    *   `get_error_response()`: Handles error responses, usually delegating to `build_error_response`.

*   **`ConnectorSpecifications` Implementation:**
    Provides metadata like supported payment methods, connector name for display, etc.

*   **`webhooks::IncomingWebhook` Implementation (If Applicable):**
    Handles incoming webhook notifications, including signature verification, event identification, and payload parsing.

## 2. Transformers File (`[connector_name]/transformers.rs`)

This file is crucial for data mapping and defining all connector-specific data types.

### Key Components:

*   **`[ConnectorNamePascalCase]RouterData<T>` Struct:**
    A helper struct to bundle Hyperswitch's `RouterData` with the amount converted to the connector's expected format/unit.

*   **Connector-Specific Request Structs:**
    *   Define Rust structs that mirror the JSON request bodies expected by `[ConnectorName]`'s API (e.g., `[ConnectorName]PaymentsRequest`, `[ConnectorName]CardDetails`).
    *   These structs derive `serde::Serialize`.
    *   Implement `TryFrom<&[ConnectorName]RouterData<&HyperswitchRequestData>>` for these structs to map data from Hyperswitch's generic types to the connector-specific format. This involves using Hyperswitch types like `pii::Email`, `cards::CardNumber`, and `masking::Secret` for sensitive data.

*   **Authentication Struct (`[ConnectorNamePascalCase]AuthType`):**
    *   Defines how authentication details (e.g., API keys, secrets) are stored and accessed.
    *   Implement `TryFrom<&ConnectorAuthType>` to parse Hyperswitch's generic auth type into the connector-specific structure.

*   **Connector-Specific Status Enums:**
    *   Define enums for payment statuses, refund statuses, etc., as used by `[ConnectorName]` (e.g., `[ConnectorName]PaymentStatus`).
    *   Implement `From<[ConnectorName]StatusEnum> for common_enums::AttemptStatus` (or `common_enums::RefundStatus`, etc.) to map these to Hyperswitch's standard statuses.

*   **Connector-Specific Response Structs:**
    *   Define Rust structs that mirror the JSON response bodies from `[ConnectorName]`'s API (e.g., `[ConnectorName]PaymentsResponse`).
    *   These structs derive `serde::Deserialize` (and often `Serialize`).
    *   Implement `TryFrom<ResponseRouterData<F, [ConnectorName]ResponseStruct, T, PaymentsResponseData>> for RouterData<F, T, PaymentsResponseData>` to map these connector-specific responses back to Hyperswitch's generic response types.

*   **Connector-Specific Error Response Struct:**
    *   Defines the structure of `[ConnectorName]`'s error responses (e.g., `[ConnectorName]ErrorResponse`), including fields like `code`, `message`, and `reason`.

*   **Webhook-Related Structs (If Applicable):**
    *   Structs to deserialize webhook payloads from `[ConnectorName]`, including event types and data objects.

## General Workflow for Integration:

1.  **API Familiarization:** Thoroughly understand the target connector's API documentation (endpoints, authentication, request/response formats, error codes, amount units).
2.  **Template Generation:** Use the `sh scripts/add_connector.sh [connector_name_lowercase] [base_url]` script to create boilerplate files.
3.  **Transformer Implementation (`transformers.rs`):**
    *   Define all necessary request, response, error, and authentication structs.
    *   Define status enums and their mappings to Hyperswitch enums.
    *   Implement all `TryFrom` traits for data conversion, adhering to Hyperswitch type conventions (e.g., using `pii::Email`, `cards::CardNumber`, `masking::Secret`, `serde` attributes like `rename_all`, `skip_serializing_if`).
4.  **Main Logic Implementation (`[connector_name].rs`):**
    *   Implement the `ConnectorCommon` trait.
    *   Implement the `ConnectorIntegration<Flow, RequestData, ResponseData>` trait for each required flow (Authorize, PSync, Capture, Void, Refund Execute, Refund Sync, etc.), using the types and `TryFrom` implementations from `transformers.rs`.
    *   Implement `ConnectorSpecifications`.
    *   If webhooks are supported, implement the `IncomingWebhook` trait.
5.  **Core Enum Updates:** Add the new connector to `Connector` and `RoutableConnectors` enums in `crates/common_enums/src/connector_enums.rs`.
6.  **Configuration:** Add connector settings to `crates/connector_configs/toml/development.toml` (and other relevant environment files).
7.  **Testing:**
    *   Adapt the generated test file (moved to `crates/router/tests/connectors/`).
    *   Add sandbox credentials to `crates/router/tests/connectors/sample_auth.toml`.
    *   Implement comprehensive tests for all supported flows and edge cases.

This structured approach, detailed in Hyperswitch's internal guides (`guide/connector_integration_guide.md` and `memory-bank/techContext.md`), ensures that connector-specific details are well-encapsulated. `transformers.rs` handles the "what" (data shapes and mapping), while `[connector_name].rs` handles the "how" (API call orchestration and flow logic).

## Learnings from Analyzing Existing Hyperswitch Connectors

This section details common patterns and variations observed across a dozen Hyperswitch connectors (`stripebilling`, `shift4`, `globalpay`, `worldpay`, `paypal`, `cybersource`, `klarna`, `nuvei`, `adyenplatform`, `airwallex`, `fiserv`, and `aci`). These insights complement the general structure outlined above and highlight practical implementation details.

### 1. Core Structure Adherence

All analyzed connectors strictly adhere to the two-file pattern:
*   **`[connector_name].rs`**: Contains the primary logic, trait implementations, and flow orchestration.
*   **`[connector_name]/transformers.rs`**: Houses all data structures specific to the connector (requests, responses, errors, authentication types, enums) and the `TryFrom` logic for converting between Hyperswitch's generic types and these connector-specific types.

This separation effectively isolates connector-specific data transformations from the main flow logic, promoting modularity and maintainability.

### 2. Key Trait Implementations in `[connector_name].rs`

*   **`ConnectorCommon`**:
    *   `id()`: Consistently returns the `snake_case` name of the connector.
    *   `get_currency_unit()`: This varies. For example, Stripe, Adyen, Klarna, Shift4, and Airwallex use `api::CurrencyUnit::Minor`. Paypal, Braintree, Fiserv, ACI, Cybersource, Globalpay, Worldpay, and Nuvei use `api::CurrencyUnit::Base`. This choice dictates how amounts are handled and converted in `transformers.rs`.
    *   `common_get_content_type()`: Predominantly `"application/json"`. ACI is an exception, using `"application/x-www-form-urlencoded"`.
    *   `base_url()`: Always retrieves the base URL from the `Connectors` configuration struct. Some connectors, like Klarna, might dynamically construct region-specific base URLs based on metadata.
    *   `get_auth_header()`: Implemented using the connector-specific authentication struct defined in `transformers.rs`. Common patterns include:
        *   Basic Authentication (Base64 encoded `username:password` or `client_id:client_secret`): Braintree, Klarna, Paypal (for token refresh).
        *   API Key in Header (e.g., `Authorization: Bearer <token>`, `x-api-key: <key>`): Adyen, Stripe, Paypal (for API calls), Airwallex, Adyenplatform.
        *   Custom Signature-Based Authentication: Fiserv, Cybersource, ACI, involving constructing a signature string from various request components (timestamp, payload, path) and signing it with a secret.
    *   `build_error_response()`: Parses the connector's specific error struct (defined in `transformers.rs`) and maps its fields (code, message, reason) to Hyperswitch's generic `ErrorResponse`. The structure of native error responses varies significantly across connectors.

*   **`ConnectorIntegration<Flow, RequestData, ResponseData>`**:
    *   Implemented for each payment or refund flow supported by the connector (e.g., `Authorize`, `Capture`, `PSync`, `RefundExecute`, `RefundSync`, `Void`).
    *   `get_headers()`: Often delegates to a common `build_headers()` (from `ConnectorCommonExt`) which might add shared headers (like `Content-Type`, `Authorization` from access token). Flow-specific headers or idempotency keys are added here or in `build_headers`.
    *   `get_url()`: Constructs the full API endpoint URL for the specific flow, often by appending a path to the `base_url()`. Path parameters (like transaction IDs or order IDs) are interpolated here.
    *   `get_request_body()`: This is a critical transformation step.
        *   It typically involves creating an instance of `[ConnectorName]RouterData` (a helper struct from `transformers.rs` that bundles `RouterData` with a converted amount).
        *   Then, it calls `TryFrom` on the connector-specific request struct (also from `transformers.rs`) passing the `[ConnectorName]RouterData`. This `TryFrom` implementation handles the detailed mapping from Hyperswitch's generic `RouterData` fields to the connector's expected request structure.
        *   Finally, it returns `RequestContent::Json(Box::new(connector_req))` for JSON bodies or `RequestContent::FormUrlEncoded(Box::new(connector_req))` for form-encoded bodies (like ACI).
    *   `handle_response()`:
        *   Parses the raw HTTP response (e.g., `res.response.parse_struct("[ConnectorName]ResponseStruct")`).
        *   The parsed connector-specific response struct is then converted back into Hyperswitch's generic `RouterData<Flow, RequestData, ResponseData>` by implementing `TryFrom<ResponseRouterData<F, [ConnectorName]ResponseStruct, T, PaymentsResponseData>> for RouterData<F, T, PaymentsResponseData>` in `transformers.rs`. This involves mapping status codes, extracting transaction IDs, handling redirection data, and potentially storing connector-specific metadata.
    *   `get_error_response()`: Usually delegates to `self.build_error_response()` defined in `ConnectorCommon`.

*   **`ConnectorSpecifications`**:
    *   `get_connector_about()`: Provides static `ConnectorInfo` (display name, description, connector type).
    *   `get_supported_payment_methods()`: Defines a `LazyLock<SupportedPaymentMethods>` static variable. This is a detailed structure specifying each supported payment method (e.g., Card, Wallet, BankRedirect), payment method type (e.g., Credit, Paypal, Klarna), and features like mandate support, refund capabilities, supported capture methods, 3DS support, and specific card networks. This is highly connector-specific.
    *   `get_supported_webhook_flows()`: Lists `common_enums::EventClass` (e.g., `Payments`, `Refunds`, `Disputes`) for which webhooks are supported.

*   **`webhooks::IncomingWebhook`**:
    *   Implemented by connectors supporting webhook notifications (e.g., Stripe, Adyen, Paypal, Airwallex, Braintree, Cybersource, Nuvei).
    *   `get_webhook_source_verification_algorithm()`: Specifies the cryptographic algorithm used for signature verification (e.g., `crypto::Sha256`, `crypto::HmacSha256`, `crypto::HmacSha1`).
    *   `get_webhook_source_verification_signature()`: Extracts the signature string from request headers (e.g., `Stripe-Signature`, `x-signature`, `bt_signature`).
    *   `get_webhook_source_verification_message()`: Constructs the exact message string that was signed by the connector. This often involves concatenating elements like timestamps, the raw request body, and potentially webhook secrets or endpoint URLs. The specifics are highly connector-dependent.
    *   `get_webhook_object_reference_id()`: Parses the webhook payload (usually JSON, but Braintree uses XML within a form-encoded payload) to extract a primary reference ID, like a payment ID or refund ID, mapping it to `api_models::webhooks::ObjectReferenceId`.
    *   `get_webhook_event_type()`: Maps the connector's native event type string or enum (from the webhook payload) to Hyperswitch's standard `IncomingWebhookEvent` enum.
    *   `get_webhook_resource_object()`: Parses and returns the main data object from the webhook payload, boxed as `dyn masking::ErasedMaskSerialize`.
    *   `get_dispute_details()`: If the webhook pertains to a dispute, this method parses dispute-specific information (amount, currency, reason, status, etc.) into `hyperswitch_interfaces::disputes::DisputePayload`.

### 3. Transformers (`transformers.rs`) Deep Dive

The `transformers.rs` file is central to isolating connector-specific data concerns.

*   **Role in Data Mapping**: Its primary role is to define all data structures that the connector API expects for requests and provides in responses, and to implement the conversion logic to and from Hyperswitch's generic data models.
*   **Key Struct Categories**:
    *   **Request Structs**: Mirror the connector's API request bodies. Derived `serde::Serialize`.
    *   **Response Structs**: Mirror the connector's API response bodies. Derived `serde::Deserialize`.
    *   **Authentication Structs (`[ConnectorName]AuthType`)**: Define how auth credentials from `ConnectorAuthType` are structured for the specific connector.
    *   **Error Structs (`[ConnectorName]ErrorResponse`)**: Define the structure of error messages from the connector.
    *   **Enums**: For statuses (payment, refund, dispute), payment brands, transaction types, etc., specific to the connector.
    *   **Metadata Structs (`[ConnectorName]Meta`)**: Often used to pass intermediate state or IDs (like `authorize_id`, `capture_id`, `session_token`) between different flow steps via `RouterData.connector_metadata`.
    *   **Helper Structs (`[ConnectorName]RouterData<T>`)**: Bundles `RouterData<T>` with the amount converted to the connector's expected unit/format, simplifying `TryFrom` implementations for request structs.

*   **`TryFrom` Trait Usage**:
    *   **Requests**: `impl TryFrom<&[ConnectorName]RouterData<&HyperswitchRequestData>> for [ConnectorName]RequestStruct`. These implementations are often complex, involving:
        *   Accessing various fields from `item.router_data.request` (e.g., amount, currency, payment method data, billing/shipping address, metadata).
        *   Conditional logic based on `PaymentMethodData` variants (Card, Wallet, BankRedirect, PayLater, MandatePayment) to construct different parts of the request payload. For example, card details are mapped differently than PayPal details.
        *   Mapping Hyperswitch types (e.g., `hyperswitch_domain_models::payment_method_data::Card`, `pii::Email`, `hyperswitch_domain_models::address::Address`) to the connector's field requirements.
        *   Handling amount conversions using the `amount_converter` (e.g., `utils::to_currency_base_unit` or `utils::to_currency_minor_unit`).
        *   Using `masking::Secret` for sensitive fields and `.peek()` or `.expose()` when the connector expects plain text.
        *   Employing `serde` attributes like `#[serde(rename_all = "camelCase")]`, `#[serde(skip_serializing_if = "Option::is_none")]`, `#[serde(flatten)]`, `#[serde(untagged)]` to match the connector's JSON/form structure.
    *   **Responses**: `impl TryFrom<ResponseRouterData<F, [ConnectorName]ResponseStruct, T, PaymentsResponseData>> for RouterData<F, T, PaymentsResponseData>`. These map the connector's response back to Hyperswitch's generic structure:
        *   Mapping the connector's status codes/strings to `common_enums::AttemptStatus` or `common_enums::RefundStatus`.
        *   Extracting the primary transaction ID into `resource_id: ResponseId::ConnectorTransactionId(...)`.
        *   Populating `mandate_reference` if a mandate/token was created.
        *   Constructing `redirection_data: Box<Option<RedirectForm>>` if the response indicates a redirect (e.g., for 3DS or off-site payment methods).
        *   Storing any necessary intermediate data or additional IDs in `connector_metadata` using a custom `Meta` struct serialized to JSON.
    *   **Authentication**: `impl TryFrom<&ConnectorAuthType> for [ConnectorName]AuthType` parses generic auth credentials into connector-specific fields.

### 4. Key Variations and Connector-Specific Implementations

*   **Authentication Mechanisms**:
    *   **Basic Auth**: Braintree (`public_key:private_key`), Klarna (`username:password`).
    *   **API Key in Header**: Adyen (`x-api-key`), Stripe (`Authorization: Bearer <secret_key>`), Paypal (`Authorization: Bearer <access_token>`), Adyenplatform (`Authorization: <api_key>`).
    *   **OAuth or Custom Token Flow for Bearer Token**: Connectors like Paypal and Airwallex first obtain an access token.
        *   Paypal uses a standard OAuth-like flow.
        *   Airwallex uses a custom login endpoint (`/authentication/login`) with `X-API-KEY` and `X-CLIENT-ID` headers to get a Bearer token, which is then used for subsequent API calls in an `Authorization: Bearer <token>` header.
    *   **Custom Signature**:
        *   Fiserv: HMAC-SHA256 of concatenated string.
        *   Cybersource: HMAC-SHA256 of a string composed of specific headers (host, date, request-target, v-c-merchant-id) and a payload digest. The signature is passed in a `Signature` header, along with `v-c-merchant-id`, `Date`, `Host`, and `Digest` (for POST/PATCH). `CybersourceAuthType` holds `api_key`, `merchant_account`, and `api_secret`.
        *   ACI: Uses `Authorization` header with Bearer token from config.
    *   **Nuvei**: Uses a `session_token` obtained via `getSessionToken.do`, then includes `merchant_id`, `merchant_site_id`, `timestamp`, and a `checksum` (SHA256 hash of concatenated fields + secret) in payment requests.
    *   **Stripebilling**: Uses a Bearer token (`Authorization: Bearer <api_key>`) along with a specific API version header (`stripe-version: 2022-11-15`). The `StripebillingAuthType` in `transformers.rs` holds the `api_key`.
    *   **Shift4**: Uses a Bearer token (`Authorization: <api_key>`). The `Shift4AuthType` in `transformers.rs` holds the `api_key`.
    *   **Globalpay**: Implements an access token flow. `GlobalpayAuthType` stores `app_id` and `key`. To get an access token, a request is made to `/accesstoken` with `app_id`, a `nonce`, and a `secret` (SHA512 hash of `nonce + key`). Subsequent API calls use `Authorization: Bearer <access_token>` and an `X-GP-Version` header.
    *   **Worldpay**: Uses Basic Authentication (`Authorization: Basic <base64_encoded_string>`). The `WorldpayAuthType` in `transformers.rs` constructs this from `key1` (username) and `api_key` (password) from `ConnectorAuthType::SignatureKey`. It also requires an `X-WP-API-Version` header. The `entity_id` (from `api_secret` in `ConnectorAuthType::SignatureKey`) is included in request bodies.
    *   **Paypal**: Implements an OAuth 2.0 client credentials flow to obtain a Bearer token. `PaypalAuthType` can be `StandardIntegration` (client_id, client_secret) or `PartnerIntegration` (client_id, client_secret, payer_id). The access token is then used in `Authorization: Bearer <token>` header. Additional headers like `PayPal-Partner-Attribution-Id`, `PayPal-Request-Id`, and `Prefer` are also used. For partner integrations, a `PayPal-Auth-Assertion` header is constructed.

*   **Amount Handling**:
    *   **Minor Units** (cents, etc.): Stripe, Adyen, Klarna, Shift4, Stripebilling, Worldpay. `StringMinorUnitForConnector` or `MinorUnitForConnector` are common. Stripebilling, Shift4 and Worldpay use `MinorUnitForConnector` and a `[ConnectorName]RouterData<T>` helper struct.
    *   **Base/Major Units** (dollars, euros, etc.): Paypal, Braintree, Fiserv, ACI, Cybersource, Globalpay, Nuvei, Airwallex.
        *   `StringMajorUnitForConnector` or `FloatMajorUnitForConnector` are used.
        *   Paypal uses `StringMajorUnitForConnector` and a `PaypalRouterData<T>` helper struct.
        *   Cybersource uses `StringMajorUnitForConnector` (via its `amount_converter`) and a `CybersourceRouterData<T>` helper struct.
        *   Airwallex and Globalpay use `StringMinorUnitForConnector` (which seems to be a misnomer if it's for base units, or implies internal conversion to minor units before sending if the connector expects minor units despite the name - Globalpay's `amount_converter` is `StringMinorUnitForConnector` suggesting it expects minor units). Globalpay uses a `GlobalPayRouterData<T>` helper.
    *   The `amount_converter` field in the connector struct and helper structs like `[ConnectorName]RouterData<T>` (e.g., `AdyenRouterData<T>`, `AirwallexRouterData<T>`, `PaypalRouterData<T>`, `CybersourceRouterData<T>`) facilitate correct amount formatting based on the connector's expected unit. The `PaypalRouterData` also includes fields for `shipping_cost`, `order_tax_amount`, and `order_amount` which are used in constructing the `purchase_units` in requests.

*   **Detailed Request/Response Transformation: PayPal Example**:
    *   **Request Transformation (`PaypalPaymentsRequest::try_from(&PaypalRouterData<&PaymentsAuthorizeRouterData>)`)**:
        *   **Intent**: Determined by `is_auto_capture()`: `PaypalPaymentIntent::Capture` or `PaypalPaymentIntent::Authorize`.
        *   **`purchase_units`**: A `Vec<PurchaseUnitRequest>`. Typically one unit.
            *   `reference_id`, `custom_id`, `invoice_id`: Mapped from `connector_request_reference_id` and `merchant_order_reference_id`.
            *   `amount`: An `OrderRequestAmount` struct.
                *   `currency_code`: From `RouterData.request.currency`.
                *   `value`: From `PaypalRouterData.amount` (already converted to major unit string).
                *   `breakdown`: An `AmountBreakdown` struct.
                    *   `item_total`: `OrderAmount` with `value` from `PaypalRouterData.amount`.
                    *   `shipping`: `OrderAmount` with `value` from `PaypalRouterData.shipping_cost`.
            *   `payee`: Optional `Payee` struct with `merchant_id` (Paypal Payer ID from auth credentials if partner integration).
            *   `shipping`: Optional `ShippingAddress` struct, mapped from `RouterData.shipping_address`.
            *   `items`: A `Vec<ItemDetails>`, typically one item with name, quantity 1, and `unit_amount` (from `PaypalRouterData.amount`).
        *   **`payment_source`**: An `Option<PaymentSourceItem>` enum. This is where different payment methods are handled:
            *   `Card`: `PaymentSourceItem::Card(CardRequest::CardRequestStruct(...))`
                *   `billing_address`: Mapped from `RouterData.billing_address`.
                *   `expiry`: Formatted as `YYYY-MM`.
                *   `name`: From billing full name.
                *   `number`: `CardNumber`.
                *   `security_code`: CVC.
                *   `attributes.vault`: If `setup_future_usage` is `OffSession`, includes `PaypalVault` with `store_in_vault: OnSuccess` and `usage_type: Merchant`.
                *   `attributes.verification`: If `auth_type` is `ThreeDs`, includes `ThreeDsMethod` with `method: ScaAlways`.
            *   `PaypalRedirect`: `PaymentSourceItem::Paypal(PaypalRedirectionRequest::PaypalRedirectionStruct(...))`
                *   `experience_context`: `ContextStruct` with `return_url`, `cancel_url` (from `complete_authorize_url`), `shipping_preference` (`SetProvidedAddress` or `GetFromFile`), and `user_action: PayNow`.
                *   `attributes.vault`: Similar to Card for `OffSession` mandate.
            *   `BankRedirect` (Eps, Giropay, Ideal, Sofort): `PaymentSourceItem::Eps(RedirectRequest(...))` etc.
                *   `name`: Billing full name.
                *   `country_code`: Billing country.
                *   `experience_context`: Similar to PaypalRedirect.
            *   `MandatePayment`:
                *   If PMD is Card: `PaymentSourceItem::Card(CardRequest::CardVaultStruct(VaultStruct { vault_id: connector_mandate_id }))`.
                *   If PMD is Paypal: `PaymentSourceItem::Paypal(PaypalRedirectionRequest::PaypalVaultStruct(VaultStruct { vault_id: connector_mandate_id }))`.
    *   **Response Transformation (`RouterData::try_from(ResponseRouterData<F, PaypalAuthResponse, T, PaymentsResponseData>)`)**:
        *   The `PaypalAuthResponse` is an enum that can be `PaypalOrdersResponse`, `PaypalRedirectResponse`, or `PaypalThreeDsResponse`.
        *   **`PaypalOrdersResponse`**:
            *   `status`: Mapped from `PaypalOrdersResponse.status` (e.g., `COMPLETED`, `PAYER_ACTION_REQUIRED`) and `intent` to Hyperswitch `AttemptStatus`.
            *   `resource_id`: `PaypalOrdersResponse.id` (Order ID).
            *   `connector_metadata`: A `PaypalMeta` struct is created.
                *   `authorize_id` or `capture_id`: Extracted from the first `purchase_units.payments.authorizations[0].id` or `captures[0].id`.
                *   `psync_flow`: Set to the `intent` from the response.
            *   `mandate_reference`: If `payment_source.paypal.attributes.vault.id` or `payment_source.card.attributes.vault.id` is present, it's used as `connector_mandate_id`.
            *   `redirection_data`: Typically `None` for direct order responses unless `PAYER_ACTION_REQUIRED`.
        *   **`PaypalRedirectResponse`**:
            *   `status`: Mapped from `PaypalRedirectResponse.status` and `intent`.
            *   `resource_id`: `PaypalRedirectResponse.id`.
            *   `redirection_data`: A `RedirectForm` is constructed using the `href` from `links` where `rel == "payer-action"`.
            *   `connector_metadata`: `PaypalMeta` with `psync_flow` set to the response `intent`. If `payment_experience` is `InvokeSdkClient`, `next_action` is set to `CompleteAuthorize`.
        *   **`PaypalThreeDsResponse`**:
            *   `status`: Mapped from `PaypalThreeDsResponse.status` (usually `PAYER_ACTION_REQUIRED`).
            *   `resource_id`: `PaypalThreeDsResponse.id`.
            *   `redirection_data`: A `RedirectForm` is constructed using the `href` from `links` where `rel == "payer-action"`. The `redirect_uri` (Hyperswitch's `complete_authorize_url`) is added as a form field.
            *   `connector_metadata`: `PaypalMeta` with `psync_flow` set to `Authenticate`.
    *   **Refund Transformation (`RouterData::try_from(ResponseRouterData<Execute, RefundResponse, RefundsData, RefundsResponseData>)`)**:
        *   `PaypalRefundRequest` contains `amount` (major unit).
        *   `RefundResponse` contains `id` (refund ID) and `status` (`RefundStatus` enum: `COMPLETED`, `PENDING`, `FAILED`).
        *   This maps to `RefundsResponseData` with `connector_refund_id` and `refund_status`.

*   **Request Structure - Intent/Confirm Pattern**:
    *   Some connectors like Airwallex (and Stripe) use a two-step process: first create a "Payment Intent" (or equivalent) which returns an intent ID and often a client secret. Then, a second call is made to "confirm" this intent with payment details.
    *   For Airwallex, PreProcessing step creates an intent (`AirwallexIntentRequest` to `/payment_intents/create`). The Authorize step then confirms this intent using the `intent_id` in the URL path (`AirwallexPaymentsRequest` to `/payment_intents/{intent_id}/confirm`).

*   **Error Handling and Structuring**:
    *   Error response structures are highly variable. Some provide a list of error objects with codes, messages, and sometimes field-specific details (e.g., Cybersource, Braintree, Stripe). Others might have a single error object or simpler code/message pairs (e.g., Fiserv, ACI).
    *   The `build_error_response` method in `[connector_name].rs` is responsible for parsing these diverse structures into Hyperswitch's standard `ErrorResponse`.

*   **Redirection Flows**:
    *   Common for 3DS authentication, off-site payment methods (Paypal, Klarna), and some bank redirects.
    *   Connectors return a redirect URL and sometimes parameters.
    *   `transformers.rs` maps this to `RedirectForm::Form { endpoint, method, form_fields }` or `RedirectForm::Html { html_data }` (e.g., Klarna checkout).
    *   The `handle_response` for authorize/session flows populates `RouterData.response.redirection_data`.

*   **Webhook Verification**:
    *   **Stripe**: Uses `Stripe-Signature` header containing `t=` (timestamp) and `v1=` (HMAC-SHA256 signature). Message is `timestamp.raw_body`.
    *   **Paypal**: Has a `v1/notifications/verify-webhook-signature` API endpoint. Requires sending various headers from the incoming webhook (`paypal-transmission-id`, `paypal-transmission-time`, `paypal-cert-url`, `paypal-transmission-sig`, `paypal-auth-algo`) along with the `webhook_id` (merchant secret) and the raw `webhook_event` body.
    *   **Braintree**: Uses `bt_signature` (public_key|signature pairs) and `bt_payload` (Base64 encoded XML). Message is the `bt_payload`. Signature is HMAC-SHA1 of the message, keyed by SHA1 of the private key.
    *   **Airwallex**: Uses `x-signature` (HMAC-SHA256 hex encoded) and `x-timestamp` header. The message to verify is `timestamp_value + raw_request_body`.
    *   **Cybersource**: Webhooks not deeply analyzed in this pass, but typically involve signature verification.
    *   **Nuvei**: Uses `advanceResponseChecksum` (SHA256 hex encoded). Message is a concatenation of `secret_key`, `totalAmount`, `currency`, `responseTimeStamp`, `pppTransactionID`, `Status` (uppercased), and `productID`.
    *   **Stripebilling**: Uses the `stripe-signature` header, which contains a timestamp (`t=timestamp_value`) and one or more signatures (`v1=signature_value`). The verification algorithm is HMAC-SHA256. The message to verify is constructed by concatenating the timestamp string, a `.` character, and the raw request body. Webhook event types like `StripebillingEventType::PaymentSucceeded` (maps to `invoice.paid`) are defined, primarily for revenue recovery scenarios.
    *   **Shift4**: Does not seem to have explicit webhook signature verification implemented in the connector code, relying on parsing the event type and data directly. Key webhook event types include `CHARGE_SUCCEEDED`, `CHARGE_FAILED`, `CHARGE_UPDATED`, `CHARGE_CAPTURED`, `CHARGE_REFUNDED`.
    *   **Globalpay**: Uses the `x-gp-signature` header. The verification algorithm is SHA512. The message to verify is constructed by concatenating the JSON payload string and the webhook secret. Event types like `CAPTURED` and `DECLINED` are mapped to Hyperswitch events.
    *   **Worldpay**: Uses the `Event-Signature` header. The verification algorithm is HMAC-SHA256, keyed with the hex-decoded webhook secret. The message to verify is the raw request body. Webhook events like `AUTHORIZED`, `SETTLED`, `REFUSED` are mapped.

*   **Request/Response Patterns**:
    *   Connectors often return a status and a `next_action` object, especially for redirect or 3DS flows.
    *   **Stripebilling**: Defines simple request structs like `StripebillingPaymentsRequest` (with `amount` and `StripebillingCard` details) and `StripebillingRefundRequest`. Response structs like `StripebillingPaymentsResponse` and `RefundResponse` include an `id` and a status enum (`StripebillingPaymentStatus`, `RefundStatus`) which maps to Hyperswitch's `AttemptStatus` or `RefundStatus`.
    *   **Shift4**:
        *   Payment requests (`Shift4PaymentsRequest`) are an enum (`Shift4PaymentMethod`) that can be `CardsNon3DSRequest`, `BankRedirectRequest`, or `Cards3DSRequest`.
        *   For 3DS, a `PreProcessing` step is made to the `/3d-secure` endpoint using `Cards3DSRequest` (containing card details and `return_url`). This returns a `Shift4ThreeDsResponse` with `enrolled` status, `redirectUrl`, and a `token`.
        *   The `CompleteAuthorize` step then uses this `token` (from `connector_metadata`) in a `CardsNon3DSRequest` (specifically `CardPayment::CardToken`) to the `/charges` endpoint.
        *   Non-3DS and completed 3DS payments receive a `Shift4NonThreeDsResponse` which includes `id`, `status` (`Shift4PaymentStatus`), `captured` flag, and an optional `flow` object with `next_action` (Redirect, Wait, None) and `redirect_url`.
        *   Refunds use `Shift4RefundRequest` (with `charge_id` and `amount`) and receive a `RefundResponse` with `id` and `status` (`Shift4RefundStatus`).
    *   **Globalpay**:
        *   `GlobalpayPaymentsRequest` is used for authorize, capture, and void. It includes `account_name` (from metadata), amount, currency, reference, country, `capture_mode`, and `payment_method` data.
        *   The `payment_method` field within the request is an enum `PaymentMethodData` which can be `Card`, `Apm` (for Paypal, Eps, Giropay, Ideal, Sofort - with `provider` field), or `DigitalWallet` (for GooglePay - with `provider` and `payment_token`).
        *   `GlobalpayPaymentsResponse` is used for payments and refunds. It contains `id`, `status` (enum `GlobalpayPaymentStatus`), `amount`, `currency`. The `payment_method` field in the response can contain an `apm.redirect_url` for redirection flows.
        *   Refunds use `GlobalpayRefundRequest` (with `amount`).
    *   **Worldpay**:
        *   `WorldpayPaymentsRequest` is the primary request struct, containing an `instruction` (with `settlement`, `method`, `payment_instrument`, `narrative`, `value`, `three_ds`, `token_creation`, `customer_agreement`) and `merchant` (with `entity_id`).
        *   The `payment_instrument` can be `Card`, `RawCardForNTI`, `CardToken`, `Googlepay`, or `Applepay`.
        *   For 3DS, if `AuthenticationType::ThreeDs`, the `instruction.three_ds` field is populated. The response (`WorldpayPaymentsResponse.other_fields`) can be `DDCResponse` (for device data collection, providing a URL and JWT) or `ThreeDsChallenged` (providing a challenge URL and JWT). The `CompleteAuthorize` flow then POSTs to specific `/3dsDeviceData` or `/3dsChallenges` endpoints.
        *   The main response `WorldpayPaymentsResponse` contains an `outcome` (enum `PaymentOutcome`) and `other_fields` (enum `WorldpayPaymentResponseFields` which can be `AuthorizedResponse`, `DDCResponse`, `ThreeDsChallenged`, `RefusedResponse`, or `FraudHighRisk`).
        *   Refunds use `WorldpayPartialRequest` (with `reference` and `value`).
    *   **Cybersource**:
        *   Uses a complex set of request structs like `CybersourcePaymentsRequest`, `CybersourceAuthSetupRequest`, and `CybersourcePreProcessingRequest` (which can be `AuthEnrollment` or `AuthValidate`).
        *   Key components in requests include `processing_information` (with `action_list`, `authorization_options`, `commerce_indicator`), `payment_information` (enum for Card, Wallets, Mandate), `order_information` (amount, billing), and `client_reference_information`.
        *   For 3DS, an initial `Authorize` call with card details might go to `/risk/v1/authentication-setups` returning an `access_token`, `device_data_collection_url`, and `reference_id` (stored in `RedirectForm::CybersourceAuthSetup`).
        *   A `PreProcessing` step then uses this `reference_id` and `return_url` in a `CybersourceAuthEnrollmentRequest` to `/risk/v1/authentications`. This can return a `step_up_url` (for challenge, stored in `RedirectForm::CybersourceConsumerAuth`) or directly provide 3DS validation data.
        *   If a challenge occurred, another `PreProcessing` step uses the `transaction_id` from the challenge redirect in a `CybersourceAuthValidateRequest` to `/risk/v1/authentication-results`.
        *   The 3DS validation data (CAVV, XID, etc., stored in `connector_metadata` as `CybersourceThreeDSMetadata`) is then used in the final `CompleteAuthorize` call to `/pts/v2/payments/` using `CybersourcePaymentsRequest`.
        *   Responses like `CybersourcePaymentsResponse` include `id`, `status` (`CybersourcePaymentStatus`), `processor_information` (network txn ID, AVS/CVN results), and `token_information` (for mandates). Error responses (`CybersourceErrorResponse`) are detailed.

*   **Response Structure - Next Actions & Status Mapping**:
    *   Connectors often return a status and a `next_action` object, especially for redirect or 3DS flows.
    *   **Adyen**: `AdyenPaymentResponse` enum can be `Response`, `RedirectionResponse`, `PresentToShopper`, etc. `AdyenRedirectAction` contains URL, method, and data. `AdyenStatus` enum is mapped to Hyperswitch `AttemptStatus`.
    *   **Airwallex**: `AirwallexPaymentsResponse` contains `status` (enum `AirwallexPaymentStatus`) and an optional `next_action: AirwallexPaymentsNextAction`. `AirwallexPaymentsNextAction` includes `url`, `method`, `data` (like JWT, 3DSMethodData), and a `stage` (e.g., `WAITING_DEVICE_DATA_COLLECTION`). These are mapped to Hyperswitch `AttemptStatus` and `RedirectForm`.

*   **Mandates and Tokenization**:
    *   **Stripe**: Uses `setup_future_usage` to create SetupIntents for tokenization.
    *   **Braintree**: Can vault payment methods during authorization/charge (`vaultPaymentMethodAfterTransacting`). Uses GraphQL mutations.
    *   **Globalpay**: Supports mandates for Card, Paypal, GooglePay, Ideal, Sofort, Eps, Giropay. The `GlobalpayPaymentsRequest` includes optional `initiator` (Merchant/Payer) and `stored_credential` (model: Recurring, sequence: First/Subsequent) fields, determined by `off_session` status and the presence of a `connector_mandate_id` (which populates `brand_reference` in the card data). The response can include a `brand_reference` in the card details, which is used as the `connector_mandate_id`.
    *   **Worldpay**: The `WorldpayPaymentsRequest.instruction` can include `token_creation` (type: Worldpay) and `customer_agreement` (type: Subscription/Unscheduled, usage: First/Subsequent, scheme_reference) for CIT/MIT flows. If a `connector_mandate_id` is provided, it's used to populate `PaymentInstrument::CardToken`. The `AuthorizedResponse.token.href` from the response is used as the `connector_mandate_id`.
    *   **Paypal**:
        *   For card payments, `setup_future_usage: OffSession` in `RouterData` translates to including `attributes.vault` with `store_in_vault: OnSuccess` and `usage_type: Merchant` in the `PaypalPaymentsRequest.payment_source.Card.CardRequestStruct`.
        *   For PayPal wallet payments, `setup_future_usage: OffSession` similarly adds `attributes.vault` to the `PaypalRedirectionStruct`.
        *   The `PaypalSetupMandatesResponse` (from `/v3/vault/payment-tokens/` endpoint) returns an `id` which is used as the `connector_mandate_id`.
        *   For subsequent payments using a token, the `connector_mandate_id` is sent in `PaymentSourceItem::Card(CardRequest::CardVaultStruct(...))` or `PaymentSourceItem::Paypal(PaypalRedirectionRequest::PaypalVaultStruct(...))`.
    *   **Cybersource**: Supports `TokenCreate` action list for creating payment instruments/customer tokens.
    *   **ACI**: Uses `registrations/{id}/payments` for subsequent payments with a stored token/mandate. `createRegistration: true` in initial payment.
    *   Connector-specific mandate IDs are stored and retrieved via `RouterData.request.mandate_id` and `MandateReference` in responses.

*   **Idempotency**:
    *   **Stripe**: `Idempotency-Key` header.
    *   **Adyen**: `Idempotency-Key` header.
    *   **Airwallex**: `request_id: String` (typically a UUID v4) in the JSON request body.
    *   **Paypal**: `PayPal-Request-Id` header.
    *   Generally handled by adding the specific header in `build_headers` or directly in the `Request` construction, or as a field in the request body.

*   **Revenue Recovery Specific Types**:
    *   Connectors like **Stripebilling** demonstrate a strong focus on revenue recovery, featuring specific types and flows. For instance, `StripebillingBillingConnectorPaymentSyncResponseData` (with `latest_charge` details) and `StripebillingRecordBackResponse` are tailored for these scenarios, distinct from standard payment/refund responses. Webhook handling in Stripebilling also heavily ties into invoice events relevant for revenue recovery.

### 5. Commonly Used Hyperswitch Types and Utilities

*   **`hyperswitch_domain_models`**:
    *   `payment_method_data::{PaymentMethodData, Card, WalletData, BankRedirectData, PayLaterData}`: Central for handling different payment types.
    *   `router_data::{RouterData, ConnectorAuthType, ErrorResponse, AccessToken}`: Core data carriers for flows.
    *   `router_request_types` & `router_response_types`: Define structures for specific flow requests/responses (e.g., `PaymentsAuthorizeData`, `PaymentsResponseData`).
    *   `address::{Address, AddressDetails}`: For billing and shipping information.
    *   `types::*`: Various supporting types.

*   **`api_models`**:
    *   `payments::*`, `refunds::*`, `webhooks::*`, `enums::*`: Define API-level DTOs and enums.

*   **`common_utils`**:
    *   `pii::{Email, IpAddress}`: For PII data.
    *   `cards::CardNumber`: For card number handling.
    *   `masking::{Secret, PeekInterface, ExposeInterface}`: Essential for managing sensitive data.
    *   `types::{StringMajorUnit, StringMinorUnit, MinorUnit, FloatMajorUnit, AmountConvertor}`: For amount conversions.
    *   `request::{Request, RequestBuilder, Method, RequestContent}`: For building HTTP requests.
    *   `ext_traits::{ByteSliceExt, BytesExt, ValueExt}`: For parsing JSON/form data from responses.
    *   `crypto`: For signature verification and hashing.
    *   `date_time`: For timestamp formatting.

*   **`common_enums`**:
    *   `enums::{Currency, CountryAlpha2, AttemptStatus, RefundStatus, CaptureMethod, PaymentMethod, PaymentMethodType, FutureUsage}`: Standardized enums used across Hyperswitch.

*   **Utility Functions from `crate::utils`**:
    *   `to_connector_meta_from_secret()` / `to_connector_meta()`: For parsing connector-specific metadata.
    *   `convert_amount()`: Helper for amount conversions.
    *   `get_unimplemented_payment_method_error_message()`: Standard error message.
    *   `is_payment_failure()`, `is_refund_failure()`: Helpers for status checks.

### 6. Conclusion

The two-file structure (`[connector_name].rs` for logic, `transformers.rs` for data) is a robust pattern that promotes separation of concerns. While the core traits provide a common interface, the implementation details within `transformers.rs` (request/response/error structs, `TryFrom` logic) and the specific API interactions in `[connector_name].rs` (auth, endpoint construction, webhook handling) are highly connector-specific. A thorough understanding of the target connector's API and careful mapping to Hyperswitch's models in `transformers.rs` are key to successful integration.

---

## How to Structure a New Connector: `[ConnectorName]` (Practical Guide)

This guide explains how to structure a new payment connector, `[ConnectorName]`, for Hyperswitch. It draws upon the patterns and best practices observed from analyzing numerous existing connectors and the detailed information now present in `docs/integrations.mdx`.

The core principle is a two-file structure within the `crates/hyperswitch_connectors/src/connectors/` directory:

1.  **`[connector_name].rs`**: The main logic file. It orchestrates the connector's interactions with Hyperswitch's core systems and implements essential traits.
2.  **`[connector_name]/transformers.rs`**: The data transformation file. It defines all connector-specific data structures (requests, responses, errors, etc.) and the logic for converting data between Hyperswitch's generic models and `[ConnectorName]`'s specific formats.

### Step 1: Initial Setup and API Familiarization

1.  **Understand `[ConnectorName]`'s API**: Before writing any code, thoroughly review `[ConnectorName]`'s API documentation. Pay close attention to:
    *   Authentication mechanisms (API keys, OAuth, custom signatures, etc.).
    *   API endpoints for different operations (authorize, capture, refund, sync, webhooks).
    *   Request and response JSON/form structures for each endpoint.
    *   Error codes and their meanings.
    *   Amount units (minor units like cents, or base units like dollars).
    *   Idempotency requirements.
    *   Webhook signature verification methods and event types.

2.  **Generate Boilerplate**: Use the provided script to create the initial file structure:
    ```bash
    sh scripts/add_connector.sh [connector_name_lowercase] [base_url_for_connector_name]
    ```
    This will create `src/connectors/[connector_name].rs` and `src/connectors/[connector_name]/transformers.rs` with some template code.

### Step 2: Implementing `[connector_name]/transformers.rs` (The Data Layer)

This file is crucial for isolating all `[ConnectorName]`-specific data details.

1.  **Define Connector-Specific Structs**:
    *   **Request Structs**: For each API operation (e.g., payment, refund), define Rust structs that mirror `[ConnectorName]`'s expected request body (e.g., `#[derive(Debug, Clone, Serialize)] pub struct [ConnectorName]PaymentsRequest { ... }`).
        *   Use `serde` attributes (`#[serde(rename_all = "camelCase")]`, `#[serde(skip_serializing_if = "Option::is_none")]`, etc.) to match `[ConnectorName]`'s JSON/form field naming conventions.
    *   **Response Structs**: Similarly, define structs for `[ConnectorName]`'s API responses (e.g., `#[derive(Debug, Clone, Deserialize, Serialize)] pub struct [ConnectorName]PaymentsResponse { ... }`).
    *   **Authentication Struct**: Define a struct to hold `[ConnectorName]`'s specific authentication credentials (e.g., `pub struct [ConnectorName]AuthType { api_key: Secret<String> }`).
    *   **Error Response Struct**: Define a struct to represent `[ConnectorName]`'s error responses (e.g., `#[derive(Debug, Clone, Deserialize, Serialize)] pub struct [ConnectorName]ErrorResponse { code: String, message: String, reason: Option<String> }`).
    *   **Status Enums**: If `[ConnectorName]` uses specific strings/codes for payment, refund, or other statuses, define enums for them (e.g., `pub enum [ConnectorName]PaymentStatus { ... }`).
    *   **Webhook Structs (if applicable)**: Define structs to deserialize webhook payloads and event types from `[ConnectorName]`.
    *   **Metadata Struct (optional)**: If you need to pass connector-specific data between different flow steps (e.g., an intermediate transaction ID), define a metadata struct (e.g., `#[derive(Clone, Debug, Serialize, Deserialize)] pub struct [ConnectorName]Meta { ... }`).

2.  **Implement `TryFrom` for Data Transformations**:
    *   **Request Transformation**:
        *   Create a helper struct: `pub struct [ConnectorName]RouterData<T> { amount: common_utils::types::MinorUnit, router_data: T }` (adjust `MinorUnit` based on `[ConnectorName]`'s currency unit).
        *   For each request struct: `impl TryFrom<&[ConnectorName]RouterData<&PaymentsAuthorizeRouterData>> for [ConnectorName]PaymentsRequest { ... }`.
            *   This implementation will map fields from Hyperswitch's generic `RouterData` (e.g., `router_data.request.amount`, `router_data.payment_method_data`, `router_data.address`) to `[ConnectorName]`'s request fields.
            *   Handle amount conversion using the appropriate utility (e.g., `utils::to_currency_base_unit_as_string`, `utils::to_currency_minor_unit_as_string`).
            *   Use `masking::Secret` for sensitive data and `.peek()` or `.expose()` when providing it to the connector.
            *   Access `router_data.router_data.request.connector_meta_data` if you need to retrieve previously stored metadata.
    *   **Response Transformation**:
        *   For each response struct: `impl<F, T> TryFrom<types::ResponseRouterData<F, [ConnectorName]PaymentsResponse, T, PaymentsResponseData>> for types::RouterData<F, T, PaymentsResponseData> { ... }`.
            *   This maps fields from `[ConnectorName]`'s response (e.g., transaction ID, status, redirect URL) back to Hyperswitch's generic `RouterData`.
            *   Map `[ConnectorName]`'s status enum/string to `common_enums::AttemptStatus` or `common_enums::RefundStatus`.
            *   Populate `resource_id: ResponseId::ConnectorTransactionId(...)`.
            *   If there's a redirect, populate `redirection_data`.
            *   Store any necessary intermediate data in `connector_metadata` using your `[ConnectorName]Meta` struct.
    *   **Authentication Transformation**:
        *   `impl TryFrom<&types::ConnectorAuthType> for [ConnectorName]AuthType { ... }`. This parses Hyperswitch's generic `ConnectorAuthType` into `[ConnectorName]`'s specific auth structure.
    *   **Status Enum Mapping**:
        *   `impl From<[ConnectorName]PaymentStatus> for common_enums::AttemptStatus { ... }`.

### Step 3: Implementing `[connector_name].rs` (The Logic Layer)

This file orchestrates the API calls and implements Hyperswitch's core connector traits.

1.  **Module and Struct Definition**:
    *   `pub mod transformers;`
    *   `use transformers as [connector_name_module];` (e.g., `use transformers as connector_name;`)
    *   Define the connector struct:
        ```rust
        #[derive(Clone)]
        pub struct [ConnectorNamePascalCase] {
            amount_converter: &'static (dyn StringMinorUnitAmountConvertor<Output = MinorUnit> + Sync), // Adjust based on currency unit
        }

        impl [ConnectorNamePascalCase] {
            pub fn new() -> &'static Self {
                &Self {
                    amount_converter: &MinorUnitForConnector, // Or StringMajorUnitForConnector, etc.
                }
            }
        }
        ```
    *   Implement marker traits for supported flows (e.g., `impl api::Payment for [ConnectorNamePascalCase] {}`, `impl api::PaymentAuthorize for [ConnectorNamePascalCase] {}`).

2.  **Implement `ConnectorCommon` Trait**:
    *   `id()`: Return `"[connector_name_lowercase]"`.
    *   `get_currency_unit()`: Return `api::CurrencyUnit::Minor` or `api::CurrencyUnit::Base` based on `[ConnectorName]`'s API.
    *   `common_get_content_type()`: Usually `"application/json"`, but can be `"application/x-www-form-urlencoded"` or other.
    *   `base_url()`: Retrieve from `connectors_conf.[connector_name_lowercase].base_url`.
    *   `get_auth_header()`: Use the `[ConnectorName]AuthType` (from `transformers.rs`) to construct the appropriate authentication headers (e.g., `Authorization: Bearer <token>`, Basic Auth, custom signature headers).
    *   `build_error_response()`: Parse `[ConnectorName]`'s error response (using `[ConnectorName]ErrorResponse` from `transformers.rs`) and map it to Hyperswitch's generic `ErrorResponse`.

3.  **Implement `ConnectorIntegration<Flow, RequestData, ResponseData>` Trait for Each Flow**:
    (e.g., `PaymentsAuthorize`, `PaymentsCapture`, `PaymentsSync`, `RefundExecute`, `RefundSync`, `PaymentsVoid`)
    *   `get_headers()`: Define request headers. Often includes `Content-Type` and auth headers. Add idempotency keys if required by `[ConnectorName]` (e.g., `Idempotency-Key`, `PayPal-Request-Id`).
    *   `get_content_type()`: Return the content type from `ConnectorCommon`.
    *   `get_url()`: Construct the full API endpoint URL for this specific flow, appending paths to `base_url()`.
    *   `get_request_body()`:
        *   Create `[connector_name_module]::[ConnectorName]RouterData { amount: converted_amount, router_data: item }`.
        *   Convert it to `[ConnectorName]`'s request struct: `let connector_req = [connector_name_module]::[ConnectorName]PaymentsRequest::try_from(&router_data_obj)?;`
        *   Return `Ok(Some(types::RequestBody::log_and_get_request_body(Box::new(connector_req), utils::Encode::<[connector_name_module]::[ConnectorName]PaymentsRequest>::url_encode_to_string_tagged)?))` for form-urlencoded or `Ok(Some(types::RequestBody::log_and_get_request_body(Box::new(connector_req), utils::Encode::<[connector_name_module]::[ConnectorName]PaymentsRequest>::encode_to_string_of_json)?))` for JSON.
    *   `build_request()`: Assemble the `services::Request` object using the above methods.
    *   `handle_response()`:
        *   Parse `[ConnectorName]`'s HTTP response into its specific response struct: `let response: [connector_name_module]::[ConnectorName]PaymentsResponse = res.response.parse_struct("[ConnectorNamePascalCase] PaymentsResponse")?;`
        *   Convert it back to Hyperswitch's `RouterData`: `types::RouterData::try_from(types::ResponseRouterData { response, data: item.data, router_data: item.router_data })?`
    *   `get_error_response()`: Usually delegates to `self.build_error_response()`.

4.  **Implement `ConnectorSpecifications` Trait**:
    *   `get_connector_about()`: Provide `ConnectorInfo` (display name, etc.).
    *   `get_supported_payment_methods()`: Define a `LazyLock<SupportedPaymentMethods>` detailing supported payment methods (Card, Wallet, etc.), types (Credit, Paypal), features (mandates, refunds), and card networks. This is highly specific to `[ConnectorName]`.

5.  **Implement `webhooks::IncomingWebhook` Trait (If Applicable)**:
    *   `get_webhook_source_verification_algorithm()`: e.g., `crypto::HmacSha256`.
    *   `get_webhook_source_verification_signature()`: Extract signature from headers.
    *   `get_webhook_source_verification_message()`: Construct the message string that `[ConnectorName]` signed.
    *   `get_webhook_object_reference_id()`: Parse webhook payload to get a reference ID (payment ID, refund ID).
    *   `get_webhook_event_type()`: Map `[ConnectorName]`'s event type to `IncomingWebhookEvent`.
    *   `get_webhook_resource_object()`: Parse and return the main data object from the webhook.
    *   `get_dispute_details()` (if dispute webhooks are supported).

### Step 4: Broader Integration and Testing

1.  **Enum Updates**: Add `[ConnectorNamePascalCase]` to the `Connector` and `RoutableConnectors` enums in `crates/common_enums/src/connector_enums.rs`.
2.  **Configuration**: Add `[ConnectorName]`'s settings (base URL, API keys, etc.) to `crates/connector_configs/toml/development.toml` and other relevant environment configuration files.
3.  **Testing**:
    *   Adapt the generated test file (usually moved to `crates/router/tests/connectors/`).
    *   Add sandbox credentials for `[ConnectorName]` to `crates/router/tests/connectors/sample_auth.toml`.
    *   Write comprehensive integration tests covering all supported flows, payment methods, authentication scenarios, and error conditions.

### Conclusion

Structuring a new Hyperswitch connector involves a clear separation of concerns: `transformers.rs` handles all data-specific definitions and mappings, while `[connector_name].rs` manages the flow logic and API interactions. A thorough understanding of `[ConnectorName]`'s API, combined with careful implementation of the `TryFrom` traits in `transformers.rs` and the core Hyperswitch traits in `[connector_name].rs`, is key to a successful integration. Refer to `docs/integrations.mdx` and existing connector implementations for detailed examples and further guidance.

---

## Connector Deep Dive: Adyen

This section provides a detailed analysis of the Adyen connector, focusing on its type definitions, data transformations, and flow implementations. This information is intended to assist developers, including AI code generation systems, in understanding and integrating new connectors with similar patterns.

### 1. Core Files

*   **`adyen.rs`**: Contains the main logic for Adyen, including trait implementations for `ConnectorCommon`, `ConnectorIntegration` for various flows (Payments, Refunds, Payouts, Disputes), `ConnectorValidation`, `ConnectorSpecifications`, and `IncomingWebhook`.
*   **`adyen/transformers.rs`**: Defines all Adyen-specific data structures (requests, responses, enums, authentication types) and implements the `TryFrom` trait for conversions between Hyperswitch's generic `RouterData` and Adyen's specific types.

### 2. Authentication (`AdyenAuthType`)

*   **Structure**:
    ```rust
    pub struct AdyenAuthType {
        pub(super) api_key: Secret<String>,
        pub(super) merchant_account: Secret<String>,
        #[allow(dead_code)]
        pub(super) review_key: Option<Secret<String>>, // Used for specific payout flows
    }
    ```
*   **Transformation**: `TryFrom<&ConnectorAuthType> for AdyenAuthType`
    *   Handles `ConnectorAuthType::BodyKey` (maps `api_key` and `key1` to `api_key` and `merchant_account`).
    *   Handles `ConnectorAuthType::SignatureKey` (maps `api_key`, `key1`, and `api_secret` to `api_key`, `merchant_account`, and `review_key` respectively).
*   **Usage**:
    *   The `api_key` is used in the `X-API-KEY` header for most API calls.
    *   The `merchant_account` is a common field in Adyen request bodies.
    *   The `review_key` is used as the `X-API-KEY` for specific payout cancel/fulfill operations that use a different Adyen endpoint.

### 3. Amount Handling

*   Adyen uses **minor currency units** (e.g., cents).
*   The `Adyen` struct holds an `amount_converter: &'static (dyn AmountConvertor<Output = MinorUnit> + Sync)`, initialized to `&MinorUnitForConnector`.
*   The helper struct `AdyenRouterData<T>` is used to bundle `RouterData<T>` with the converted `MinorUnit` amount, simplifying `TryFrom` implementations for request structs.
    ```rust
    #[derive(Debug, Serialize)]
    pub struct AdyenRouterData<T> {
        pub amount: MinorUnit,
        pub router_data: T,
    }
    ```
*   The `Amount` struct (`{ currency: enums::Currency, value: MinorUnit }`) is consistently used in Adyen request and response bodies.

### 4. Key Request Structures and Transformations

The `adyen/transformers.rs` file defines numerous request structs. The most central one is `AdyenPaymentRequest`, which is highly versatile.

*   **`AdyenPaymentRequest<'a>`**:
    *   **Fields**: Includes `amount`, `merchant_account`, `payment_method` (an enum itself), `reference` (Hyperswitch's `connector_request_reference_id`), `return_url`, `browser_info`, `shopper_interaction`, `recurring_processing_model`, `additional_data`, `shopper_reference`, `store_payment_method`, address details, etc.
    *   **`TryFrom<&AdyenRouterData<&PaymentsAuthorizeRouterData>> for AdyenPaymentRequest<'_>`**: This is a complex implementation that branches based on:
        *   **Mandate ID**: If `mandate_id` is present in `RouterData`, it constructs an `AdyenMandate` within the `payment_method` field.
            *   `ConnectorMandateId`: Uses `storedPaymentMethodId`.
            *   `NetworkMandateId`: Constructs an `AdyenCard` with `networkPaymentReference`.
            *   `NetworkTokenWithNTI`: Constructs an `AdyenNetworkTokenData` with `networkPaymentReference`.
        *   **`PaymentMethodData` variant**:
            *   `Card`: Creates `AdyenCard` (maps card number, expiry, CVC, holder name, brand).
            *   `Wallet`: Handles various wallet types (GooglePay, ApplePay, Paypal, AliPay, etc.) by creating corresponding `AdyenPaymentMethod` enum variants (e.g., `AdyenGPay`, `AdyenApplePay`, `AdyenPaypal`). Specific data like tokens (`googlePayToken`, `applePayToken`) are mapped.
            *   `PayLater`: Handles Klarna, Affirm, AfterpayClearpay, etc. Requires specific fields like email, customer ID, country, and sometimes line items.
            *   `BankRedirect`: Handles Bancontact, BLIK, EPS, iDEAL, Sofort, Trustly, etc. Often involves mapping issuer codes or specific bank data.
            *   `BankDebit`: Handles ACH, SEPA, BACS. Maps account numbers, routing/sort codes, IBAN, and owner names.
            *   `BankTransfer`: Handles various virtual account methods (Permata, BCA, BNI, etc.) and Pix.
            *   `Voucher`: Handles Boleto, Oxxo, convenience store vouchers.
            *   `GiftCard`: Handles PaySafeCard, Givex.
            *   `NetworkToken`: Creates `AdyenNetworkTokenData`.
    *   **Common Logic in `TryFrom` for `AdyenPaymentRequest`**:
        *   `amount`: From `AdyenRouterData.amount`.
        *   `merchant_account`: From `AdyenAuthType`.
        *   `shopper_interaction`: Determined by `RouterData.request.off_session` (`Ecommerce` or `ContinuedAuthentication`).
        *   `recurring_processing_model`, `store_payment_method`, `shopper_reference`: Determined by `setup_future_usage` and `off_session` flags, and customer ID.
        *   `browser_info`: Populated if 3DS is required or for certain payment methods, using `RouterData.request.get_browser_info()`.
        *   `additional_data`: Includes `authorisation_type` (for manual capture), `manual_capture` flag, `execute_three_d` flag, and `riskdata` (if present in `RouterData.request.metadata`).
        *   `return_url`: From `RouterData.request.get_router_return_url()`.
        *   Address details (`billing_address`, `delivery_address`), shopper details (`shopper_name`, `shopper_email`, `telephone_number`), `country_code`, `line_items` are populated from `RouterData`.
        *   `channel`: Can be `Web` for certain payment methods like GoPay, Vipps.
        *   `splits`: If `RouterData.request.split_payments` is `AdyenSplitPayment`, it's mapped to `AdyenSplitData`.
        *   `device_fingerprint`: Extracted from `RouterData.request.metadata`.

*   **`AdyenCaptureRequest`**:
    *   Fields: `merchant_account`, `amount`, `reference`.
    *   `TryFrom<&AdyenRouterData<&PaymentsCaptureRouterData>>`: Populates amount and merchant account. `reference` is either `capture_id` (for multiple captures) or `connector_request_reference_id` (for single capture).

*   **`AdyenRefundRequest`**:
    *   Fields: `merchant_account`, `amount`, `merchant_refund_reason`, `reference` (Hyperswitch's `refund_id`), `splits`, `store`.
    *   `TryFrom<&AdyenRouterData<&RefundsRouterData<Execute>>>`: Populates fields.

*   **`AdyenCancelRequest`**: (For Void)
    *   Fields: `merchant_account`, `reference` (Hyperswitch's `connector_request_reference_id`).
    *   `TryFrom<&PaymentsCancelRouterData>`: Populates fields.

*   **`AdyenBalanceRequest`**: (For Gift Card Balance Check - PreProcessing)
    *   Fields: `payment_method` (specifically `AdyenPaymentMethod::PaymentMethodBalance` with Givex card details), `merchant_account`.
    *   `TryFrom<&PaymentsPreProcessingRouterData>`: Extracts Givex card details.

*   **Payout Requests (e.g., `AdyenPayoutCreateRequest`, `AdyenPayoutFulfillRequest`, `AdyenPayoutCancelRequest`, `AdyenPayoutEligibilityRequest`)**:
    *   These are defined if the `payouts` feature is enabled.
    *   They map fields from `PayoutsRouterData` for different payout flows.
    *   Handle different payout methods like Bank (SEPA) and Wallet (Paypal).
    *   Include fields like `recurring.contract = "POUT"`, `shopper_reference`, `shopper_email`, `shopper_name`, `date_of_birth`, `entity_type`, `nationality`, `billing_address`.

*   **Dispute Requests (e.g., `AdyenAcceptDisputeRequest`, `AdyenDefendDisputeRequest`, `Evidence` for submit evidence)**:
    *   Map fields from `AcceptDisputeRouterData`, `DefendDisputeRouterData`, `SubmitEvidenceRouterData`.
    *   Include `dispute_psp_reference`, `merchant_account_code`.
    *   `Evidence` struct contains `defense_documents` (base64 encoded file content and type).

### 5. Key Response Structures and Transformations

*   **`AdyenPaymentResponse` (enum)**: This is a versatile enum that can deserialize into different structures based on the Adyen API response.
    *   `AdyenPaymentResponse::Response(Box<AdyenResponse>)`: Standard synchronous response.
        *   `psp_reference`: Connector transaction ID.
        *   `result_code`: An `AdyenStatus` enum (e.g., `Authorised`, `Refused`, `RedirectShopper`).
        *   `amount`, `merchant_reference`.
        *   `refusal_reason`, `refusal_reason_code`.
        *   `additional_data`: Can contain `recurring_detail_reference` (mandate ID), `network_tx_reference`.
    *   `AdyenPaymentResponse::RedirectionResponse(Box<RedirectionResponse>)`: For redirect flows.
        *   Contains `action: AdyenRedirectAction` which has `url`, `method`, `data` (form fields).
    *   `AdyenPaymentResponse::PresentToShopper(Box<PresentToShopperResponse>)`: For flows requiring shopper interaction (e.g., vouchers).
        *   Contains `action: AdyenPtsAction` with `reference`, `download_url`, `expires_at`.
    *   `AdyenPaymentResponse::QrCodeResponse(Box<QrCodeResponseResponse>)`: For QR code flows.
        *   Contains `action: AdyenQrCodeAction` with `qr_code_data`, `qr_code_url`.
    *   `AdyenPaymentResponse::RedirectionErrorResponse(Box<RedirectionErrorResponse>)`: If a redirect itself results in an error.
    *   `AdyenPaymentResponse::WebhookResponse(Box<AdyenWebhookResponse>)`: Used internally for handling webhook data in PSync.

*   **Transformation to `PaymentsResponseData`**:
    *   The `ForeignTryFrom` implementation for `RouterData<F, Req, PaymentsResponseData>` (where `Req` is `PaymentsAuthorizeData`, `PaymentsSyncData`, etc.) handles these different `AdyenPaymentResponse` variants.
    *   **Status Mapping**: `get_adyen_payment_status(is_manual_capture, adyen_status_enum, payment_method_type)` maps `AdyenStatus` to `AttemptStatus`.
        *   `Authorised` maps to `Authorized` (manual capture) or `Charged` (auto capture).
        *   `RedirectShopper`, `ChallengeShopper`, `PresentToShopper` map to `AuthenticationPending`.
        *   `Refused`, `Error` map to `Failure`.
    *   **Error Handling**: If `refusal_reason` or `refusal_reason_code` is present, or status is Failure, an `ErrorResponse` is constructed.
    *   **Redirection Data**: If `AdyenRedirectAction` is present, its `url` and `data` are mapped to `RedirectForm`.
    *   **Mandate Reference**: Extracted from `additional_data.recurring_detail_reference`.
    *   **Connector Metadata**:
        *   For QR codes (`QrCodeResponseResponse`), `get_qr_metadata` creates `QrCodeInformation`.
        *   For PresentToShopper (`PresentToShopperResponse`), `get_present_to_shopper_metadata` creates `VoucherNextStepData` or `BankTransferInstructions`.
        *   For some redirect flows (`RedirectionResponse`), `get_wait_screen_metadata` can add polling information.
    *   **Network Transaction ID**: Extracted from `additional_data.network_tx_reference`.

*   **`AdyenCaptureResponse`**:
    *   Fields: `psp_reference` (capture ID), `payment_psp_reference` (original payment ID), `status` (string, usually "received"), `amount`.
    *   `TryFrom<PaymentsCaptureResponseRouterData<AdyenCaptureResponse>> for PaymentsCaptureRouterData`: Maps to `AttemptStatus::Pending` as Adyen capture is asynchronous. `resource_id` is the `psp_reference`.

*   **`AdyenRefundResponse`**:
    *   Fields: `psp_reference` (refund ID), `status` (string, usually "received").
    *   `TryFrom<RefundsResponseRouterData<F, AdyenRefundResponse>> for RefundsRouterData<F>`: Maps to `RefundStatus::Pending`.

*   **`AdyenCancelResponse`**: (For Void)
    *   Fields: `payment_psp_reference`, `status` (`CancelStatus` enum: `Received` or `Processing`).
    *   `TryFrom<PaymentsCancelResponseRouterData<AdyenCancelResponse>> for PaymentsCancelRouterData`: Maps to `AttemptStatus::Pending`.

*   **`AdyenErrorResponse`**:
    *   Fields: `status` (HTTP status), `error_code`, `message`, `error_type`, `psp_reference`.
    *   Used in `ConnectorCommon::build_error_response` to populate Hyperswitch's `ErrorResponse`.

*   **Payout Responses (`AdyenPayoutResponse`)**:
    *   Fields: `psp_reference`, `result_code` (AdyenStatus), `response` (AdyenStatus), `amount`, `refusal_reason`.
    *   `TryFrom<PayoutsResponseRouterData<F, AdyenPayoutResponse>> for PayoutsRouterData<F>`: Maps `AdyenStatus` to `PayoutStatus`. Handles `payout_eligible` flag from `additional_data`.

*   **Dispute Responses (`AdyenDisputeResponse`)**:
    *   Fields: `error_message`, `success` (boolean).
    *   `ForeignTryFrom<(&Self, AdyenDisputeResponse)>` for `AcceptDisputeRouterData`, `DefendDisputeRouterData`, `SubmitEvidenceRouterData`: Maps to `DisputeStatus::DisputeAccepted` or `DisputeStatus::DisputeChallenged` if `success` is true, otherwise constructs an `ErrorResponse`.

### 6. Key Enums

*   **`AdyenStatus`**: Represents various states of a payment/payout (e.g., `Authorised`, `Refused`, `RedirectShopper`, `Cancelled`, `Pending`).
*   **`AdyenShopperInteraction`**: `Ecommerce`, `ContAuth` (Continued Authentication/Off-Session), `Moto`, `POS`.
*   **`AdyenRecurringModel`**: `UnscheduledCardOnFile`, `CardOnFile`. Used for tokenization.
*   **`PaymentType` (Adyen specific)**: An extensive enum mapping Hyperswitch payment methods to Adyen's specific type strings (e.g., `scheme` for cards, `paypal`, `klarna`, `ideal`, `sepadirectdebit`). This is crucial for constructing the `paymentMethod.type` field in requests.
*   **`CardBrand` (Adyen specific)**: Maps card networks like Visa, Mastercard to Adyen's brand codes (e.g., `mc`, `amex`).
*   **`WebhookEventCode`**: Maps Adyen's webhook event strings (e.g., `AUTHORISATION`, `REFUND`, `CAPTURE`) to a Rust enum.
*   **`AdyenWebhookStatus`**: Internal enum to represent webhook outcomes before mapping to `AttemptStatus`.

### 7. Webhook Handling

*   **Verification**:
    *   Algorithm: `HmacSha256`.
    *   Signature: Extracted from `additional_data.hmac_signature` within the webhook notification item.
    *   Message: Constructed by concatenating `psp_reference`, `original_reference`, `merchant_account_code`, `merchant_reference`, `amount.value`, `amount.currency`, `event_code`, and `success` status from the notification item.
    *   Secret: HMAC key is derived from the hex-decoded webhook secret configured for the merchant.
*   **Key Structs**:
    *   `AdyenIncomingWebhook`: Top-level struct containing `notification_items`.
    *   `AdyenItemObjectWH`: Wrapper for `AdyenNotificationRequestItemWH`.
    *   `AdyenNotificationRequestItemWH`: Contains the actual event data (`psp_reference`, `event_code`, `amount`, `success`, `additional_data`, etc.).
*   **Event Type Mapping**: `get_adyen_webhook_event(event_code, success_flag, dispute_status_option)` maps `WebhookEventCode` and success status to `IncomingWebhookEvent` (e.g., `AUTHORISATION` + success -> `PaymentIntentSuccess`).
*   **Resource Object**: `AdyenWebhookResponse` is created from `AdyenNotificationRequestItemWH` and returned as the resource object.
*   **Object Reference ID**: Extracted based on `event_code`. For `AUTHORISATION`, it's `merchant_reference` (PaymentAttemptId). For `CAPTURE` or `CANCELLATION`, it's `original_reference` (ConnectorTransactionId of the original payment). For `REFUND`, it's `merchant_reference` (RefundId).
*   **Dispute Details**: Parsed from `AdyenNotificationRequestItemWH` if it's a dispute event.
*   **Mandate Details**: `recurringDetailReference` from `additional_data` is used as `connector_mandate_id`.

### 8. URL Construction and Endpoints

*   **Base URLs**: Separate base URLs for payments (`connectors.adyen.base_url`), payouts (`connectors.adyen.payout_base_url`), and disputes (`connectors.adyen.dispute_base_url`) are retrieved from the `Connectors` config.
*   **`build_env_specific_endpoint` function**:
    *   If `test_mode` is true (or None), uses the base URL directly.
    *   If `test_mode` is false (live mode), it attempts to read `endpoint_prefix` from `connector_meta_data` (parsed into `AdyenConnectorMetadataObject`). The base URL often contains a placeholder like `{{merchant_endpoint_prefix}}` which is replaced by this prefix. This allows for merchant-specific live endpoints.
*   **API Version**: `ADYEN_API_VERSION` (e.g., "v68") is appended to paths.
*   **Paths**: Specific paths are appended for different operations (e.g., `/payments`, `/payments/{id}/captures`, `/payments/{id}/refunds`, `/pal/servlet/Payout/{version}/storeDetailAndSubmitThirdParty`, `/ca/services/DisputeService/v30/acceptDispute`).

### 9. Connector Validation (`ConnectorValidation` trait)

*   **`validate_connector_against_payment_request`**: Checks if the `capture_method` is supported for the given `payment_method_type`. Adyen has a detailed matrix of supported capture methods for various payment types.
*   **`validate_mandate_payment`**: Checks if the payment method data (`pm_data`) and type (`pm_type`) are supported for mandate payments by Adyen. Uses a `HashSet` of supported `PaymentMethodDataType`s.
*   **`validate_psync_reference_id`**: For PSync, Adyen requires `encoded_data` (containing redirect results) to be present.
*   **`is_webhook_source_verification_mandatory`**: Returns `true`.

### 10. Specific Flow Implementations (Highlights)

*   **Payments (Authorize, SetupMandate)**:
    *   The `AdyenPaymentRequest` is constructed as detailed above, handling various PMDs.
    *   Response handling involves parsing `AdyenPaymentResponse` and its variants.
*   **Payments (PSync)**:
    *   Used for redirect flows. `encoded_data` from the redirect is parsed into `AdyenRedirectRequestTypes` (AdyenRedirection, AdyenThreeDS, AdyenRefusal) and sent to the `/payments/details` endpoint.
    *   If `encoded_data` is not present (non-redirect flow), PSync is effectively skipped, relying on webhooks.
*   **Payments (Capture)**:
    *   Uses `/payments/{payment_id}/captures` endpoint.
    *   `AdyenCaptureRequest` includes amount and merchant account.
    *   Response `AdyenCaptureResponse` usually indicates "received", final status via webhook.
*   **Payments (Void/Cancel)**:
    *   Uses `/payments/{payment_id}/cancels` endpoint.
    *   `AdyenCancelRequest` includes merchant account and reference.
    *   Response `AdyenCancelResponse` indicates "received" or "processing".
*   **Refunds (Execute)**:
    *   Uses `/payments/{payment_id}/refunds` endpoint.
    *   `AdyenRefundRequest` includes amount, merchant account, reason.
    *   Response `AdyenRefundResponse` indicates "received".
*   **Payouts (if `payouts` feature enabled)**:
    *   Uses different base URL (`connectors.adyen.payout_base_url`) and paths like `/pal/servlet/Payout/{version}/storeDetailAndSubmitThirdParty` (Create), `/declineThirdParty` (Cancel), `/confirmThirdParty` or `/payout` (Fulfill).
    *   `AdyenPayoutCreateRequest` handles bank (SEPA) and wallet (Paypal) payouts, requiring detailed shopper and bank/wallet information.
    *   `AdyenPayoutFulfillRequest` differs for Bank/Wallet (uses `original_reference`) vs. Card (requires full card details again).
    *   Authentication for some payout flows might use the `review_key` from `AdyenAuthType`.
*   **Disputes**:
    *   Uses `connectors.adyen.dispute_base_url` and paths like `/ca/services/DisputeService/v30/acceptDispute`, `/defendDispute`, `/supplyDefenseDocument`.
    *   Requests include `dispute_psp_reference` and `merchant_account_code`.
    *   `Evidence` for submit includes base64 encoded documents.
*   **File Upload (`FileUpload` trait)**:
    *   `validate_file_upload`: Checks file type (JPEG, PNG, PDF) and size limits for `DisputeEvidence`.

This deep dive should provide a solid foundation for understanding the Adyen connector's type system and data flow, aiding in the integration of new connectors with similar characteristics.