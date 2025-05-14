# Hyperswitch Codebase Rulebook

**Last Updated:** 2025-05-14

**Objective:** This document outlines the observed coding conventions, patterns, and architectural decisions within the Hyperswitch codebase, primarily focusing on the `crates/` directory. It is intended to be a living document, updated iteratively as the codebase is explored and understood.

---

## `crates/common_enums/src/connector_enums.rs`

**Last Updated:** 2025-05-14

This file primarily defines enumerations related to payment connectors.

**Key Enums:**

1.  **`RoutableConnectors`**:
    *   Represents a subset of connectors eligible for payment routing.
    *   Derives: `Clone`, `Copy`, `Debug`, `Eq`, `Hash`, `PartialEq`, `serde::Serialize`, `serde::Deserialize`, `strum::Display`, `strum::EnumString`, `strum::EnumIter`, `strum::VariantNames`, `ToSchema`.
    *   Uses `#[router_derive::diesel_enum(storage_type = "db_enum")]` for Diesel ORM integration, stored as a database enum.
    *   Uses `#[serde(rename_all = "snake_case")]` and `#[strum(serialize_all = "snake_case")]` for consistent snake_case naming in serialization and string representations.
    *   Includes conditional compilation (`#[cfg(feature = "dummy_connector")]`) for dummy/test connectors.

2.  **`Connector`**:
    *   Represents all available connector integrations.
    *   Derives a similar set of traits as `RoutableConnectors`.
    *   Uses `#[router_derive::diesel_enum(storage_type = "text")]` for Diesel ORM integration, stored as text.
    *   Also uses `#[serde(rename_all = "snake_case")]` and `#[strum(serialize_all = "snake_case")]`.
    *   Contains a broader list of connectors, including those not necessarily used for routing (e.g., 3DS servers, risk engines).

**Key Patterns & Conventions:**

*   **Derive Macros:** Extensive use of derive macros for common traits (`serde`, `strum`, `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`, `ToSchema`). This promotes boilerplate reduction and consistency.
*   **Custom Derive for Diesel:** `#[router_derive::diesel_enum]` indicates a custom derive macro for integrating enums with the Diesel ORM.
*   **Serialization/Deserialization:** `serde` is used for JSON (and potentially other formats) serialization/deserialization, with `rename_all = "snake_case"` ensuring a consistent naming convention.
*   **String Conversion:** `strum` crate is used for `Display`, `EnumString` (parsing from string), `EnumIter` (iterating over variants), and `VariantNames` (getting variant names), also with `serialize_all = "snake_case"`.
*   **Feature Flags:** `#[cfg(feature = "dummy_connector")]` demonstrates the use of feature flags to conditionally include code, particularly for test/dummy implementations.
*   **Connector Capabilities:** The `impl Connector` block centralizes logic for determining the capabilities of different connectors (e.g., `supports_instant_payout`, `supports_access_token`). These methods often use the `matches!` macro for concise pattern matching against `(self, associated_data)` tuples.
*   **Type Conversion:** Clear `From` and `TryFrom` implementations manage the relationship between `RoutableConnectors` and `Connector`, where `RoutableConnectors` is a strict subset of `Connector`. The `TryFrom` implementation returns an error string if a `Connector` is not routable.
*   **Comments for Future Work:** Some connectors are commented out in the enum definitions, often with a note (e.g., "// Amazonpay", "// Facilitapay"). This might indicate connectors that are planned, deprecated, or partially implemented.

*(Further analysis of other files in `common_enums` and other crates will refine these observations.)*

---

## `crates/common_enums/src/enums.rs`

**Last Updated:** 2025-05-14

This file serves as a central point for defining and exporting numerous general-purpose enumerations used throughout the application. It also defines some top-level error types.

**Module Structure:**

*   Declares submodules: `accounts`, `payments`, `ui`.
    *   `pub use accounts::MerchantProductType;`
    *   `pub use payments::ProductType;`
    *   `pub use ui::*;`
*   Re-exports `RoutableConnectors` from `super::connector_enums`.
*   Contains a `diesel_exports` module:
    *   This module seems to be a convention to re-export enums with a `Db` prefix (e.g., `DbAttemptStatus as AttemptStatus`). This is likely to avoid naming conflicts or to provide specific type aliases for Diesel's schema generation or usage.

**Key Enums & Types:**

*   **Error Enums:**
    *   `ApplicationError`: Top-level application errors (e.g., `ConfigurationError`, `IoError`, `ApiClientError`). Uses `thiserror::Error`.
    *   `ApiClientError`: Specific errors related to external API client interactions (e.g., `HeaderMapConstructionFailed`, `RequestTimeoutReceived`, `ResponseDecodingFailed`). Also uses `thiserror::Error`.
        *   Includes helper methods like `is_upstream_timeout()`.

*   **Payment Lifecycle Enums:**
    *   `AttemptStatus`: Status of a payment attempt (e.g., `Started`, `Authorized`, `Charged`, `Failed`). Includes `is_terminal_status()` method.
    *   `AuthenticationType`: (e.g., `ThreeDs`, `NoThreeDs`).
    *   `CaptureMethod`: (e.g., `Automatic`, `Manual`).
    *   `CaptureStatus`: (e.g., `Started`, `Charged`, `Pending`).
    *   `IntentStatus`: Status of a payment intent (e.g., `Succeeded`, `Failed`, `RequiresCustomerAction`). Includes `is_in_terminal_state()` and `should_force_sync_with_connector()` methods.
    *   `RefundStatus`: (e.g., `Pending`, `Success`, `Failure`).
    *   `DisputeStage`: (e.g., `PreDispute`, `Dispute`).
    *   `DisputeStatus`: (e.g., `DisputeOpened`, `DisputeWon`, `DisputeLost`).
    *   `MandateStatus`: (e.g., `Active`, `Inactive`, `Revoked`).
    *   `PayoutStatus`: (e.g., `Success`, `Failed`, `Pending`).

*   **Payment Method Enums:**
    *   `PaymentMethod`: Broad categories (e.g., `Card`, `Wallet`, `BankRedirect`).
    *   `PaymentMethodType`: Specific payment methods (e.g., `ApplePay`, `GooglePay`, `Klarna`, `Sepa`). Includes `should_check_for_customer_saved_payment_method_type()` and `to_display_name()`.
    *   `CardNetwork`: (e.g., `Visa`, `Mastercard`).
    *   `CardDiscovery`: How a card was discovered (e.g., `Manual`, `SavedCard`).

*   **Currency & Country Enums:**
    *   `Currency`: Extensive list of ISO currency codes (e.g., `USD`, `EUR`, `INR`).
        *   Includes significant helper methods: `to_currency_base_unit()`, `to_currency_base_unit_asf64()`, `to_currency_lower_unit()`, `iso_4217()`, `is_zero_decimal_currency()`, `is_three_decimal_currency()`, `number_of_digits_after_decimal_point()`.
    *   `CountryAlpha2`: ISO 3166-1 alpha-2 country codes.
    *   `CountryAlpha3`: ISO 3166-1 alpha-3 country codes.
    *   `Country`: Full country names. (Transformations between these are likely in `transformers.rs`).
    *   Specific enums for states/provinces of various countries (e.g., `UsStatesAbbreviation`, `CanadaStatesAbbreviation`, etc.). These are very detailed.

*   **Connector & System Enums:**
    *   `ConnectorType`: Categorizes connectors (e.g., `PaymentProcessor`, `PaymentVas`, `TaxProcessor`).
    *   `MerchantStorageScheme`: (e.g., `PostgresOnly`, `RedisKv`).
    *   `FileUploadProvider`: (e.g., `Router`, `Stripe`).
    *   `ApiVersion`: (e.g., `V1`, `V2`).
    *   `ProcessTrackerStatus`: For background task processing (e.g., `Processing`, `New`, `Finish`).
    *   `ProcessTrackerRunner`: Identifies different background runners/workflows.

*   **User & Role Enums:**
    *   `RoleScope`: (e.g., `Organization`, `Merchant`).
    *   `PermissionGroup`: For role-based access control.
    *   `EntityType`: (e.g., `Tenant`, `Organization`, `Merchant`).

**Key Patterns & Conventions (reinforcing and adding to previous observations):**

*   **Comprehensive Enum Definitions:** Enums are heavily used to model discrete states, types, and categories across the system.
*   **Derive Macros:** Consistent use of `serde::{Serialize, Deserialize}`, `strum::{Display, EnumString, EnumIter, VariantNames}`, `utoipa::ToSchema`, `Clone`, `Copy`, `Debug`, `Eq`, `PartialEq`, `Hash`, `Default`.
*   **Diesel Integration:** `#[router_derive::diesel_enum(storage_type = "...")]` is standard for enums persisted in the database. Storage types vary (`db_enum`, `text`).
*   **Serialization Naming:** `#[serde(rename_all = "snake_case")]` or `#[serde(rename_all = "PascalCase")]` or `#[serde(rename_all = "lowercase")]` is common for consistent external representation. `strum` often mirrors this with `serialize_all`.
*   **`thiserror` for Error Enums:** Provides convenient derive macros for implementing `std::error::Error`.
*   **Helper Methods on Enums:** Many enums have `impl` blocks with methods that provide useful logic related to the enum's variants (e.g., `is_terminal_status()`, currency conversions).
*   **`Default` Trait:** Often derived or implemented, specifying a sensible default variant for an enum.
*   **Modularity:** Grouping related enums into submodules (`accounts`, `payments`, `ui`) improves organization.
*   **ISO Standards:** Adherence to ISO standards for currency (`Currency`) and country codes (`CountryAlpha2`, `CountryAlpha3`).
*   **Exhaustive State/Province Lists:** The file contains very long enums for states/provinces of many countries, indicating a need for detailed address handling.

*(This section will be further refined as more of the `common_enums` crate and other crates are analyzed.)*

---

## `crates/common_enums/src/transformers.rs`

**Last Updated:** 2025-05-14

This file is dedicated to providing transformation logic between different enum types defined in `common_enums`, particularly for country code representations, and custom `serde` implementations.

**Key Responsibilities & Patterns:**

1.  **Country Code Transformations:**
    *   Provides `const fn` implementations for converting between `Country`, `CountryAlpha2`, `CountryAlpha3`, and numeric country codes.
        *   `CountryAlpha2::from_alpha2_to_alpha3()`
        *   `Country::from_alpha2()`, `Country::to_alpha2()`
        *   `Country::from_alpha3()`, `Country::to_alpha3()`
        *   `Country::from_numeric()`, `Country::to_numeric()`
    *   This allows for efficient and type-safe conversions, often at compile time.
    *   A `NumericCountryCodeParseError` struct is defined for errors during numeric code parsing.

2.  **Custom `serde` for Country Codes (`custom_serde` module):**
    *   Defines custom serialization and deserialization logic for the `Country` enum to allow it to be represented as Alpha2, Alpha3, or numeric codes directly in serialized formats (e.g., JSON).
    *   Modules within `custom_serde`: `alpha2_country_code`, `alpha3_country_code`, `numeric_country_code`.
    *   Each submodule provides `serialize` and `deserialize` functions that work with `serde::Serializer` and `serde::Deserializer`, often using a custom `Visitor`.
    *   This pattern is crucial for flexible API design where country codes might be accepted or emitted in different standard formats.

3.  **`From` Trait Implementations for Enum Conversions:**
    *   **`From<PaymentMethodType> for PaymentMethod`**: Maps specific payment method types (e.g., `ApplePay`) to their general categories (e.g., `Wallet`).
    *   **`From<AttemptStatus> for IntentStatus`**: Translates detailed `AttemptStatus` values to the higher-level `IntentStatus`. This is a key piece of logic for managing payment lifecycle states.
    *   **`From<Option<bool>>` for various enums**:
        *   `External3dsAuthenticationRequest`
        *   `EnablePaymentLinkRequest`
        *   `MitExemptionRequest`
        *   `PresenceOfCustomerDuringPayment`
        *   This pattern converts optional boolean flags (common in API inputs where a flag might be omitted to imply a default) into more descriptive and type-safe enum variants.
        *   These enums typically also provide an `as_bool()` method for the reverse conversion.

4.  **Unit Testing:**
    *   The file includes a `#[cfg(test)] mod tests` block with comprehensive unit tests.
    *   Tests specifically target the country code serialization/deserialization logic, covering:
        *   Serialization to Alpha2, Alpha3, and numeric formats.
        *   Deserialization from these formats.
        *   Round-trip (serialize then deserialize, and vice-versa) conversions.
        *   Handling of invalid input codes during deserialization.
    *   This demonstrates a commitment to ensuring the correctness of these critical transformations.

**Overall Observations:**

*   This file centralizes transformation logic, promoting code reusability and maintainability.
*   The use of `const fn` for country code conversions is a performance-conscious choice.
*   Custom `serde` implementations showcase advanced usage of the `serde` framework to meet specific data representation needs.
*   The `From<Option<bool>>` pattern is an elegant way to handle optional boolean flags in a type-safe manner.
*   Thorough unit testing of transformation logic is a good practice observed here.

---

## Guide to Integrating a Connector (`add_connector_updated.md`)

**Last Updated:** 2025-05-14 (based on the content of `add_connector_updated.md`)

This document outlines the official process for integrating a new payment connector into the Hyperswitch system.

**Key Integration Steps & Patterns:**

1.  **Prerequisites:**
    *   Familiarity with the target connector's API.
    *   Local Hyperswitch repository setup.
    *   Test credentials for the connector.
    *   Rust nightly toolchain.

2.  **Template Generation:**
    *   A script `scripts/add_connector.sh <connector-name> <connector-base-url>` is used to generate boilerplate code for the new connector.
    *   This creates:
        *   `hyperswitch_connectors/src/connectors/<connector_name>/transformers.rs`
        *   `hyperswitch_connectors/src/connectors/<connector_name>.rs` (main connector logic)
        *   Test file initially at `hyperswitch_connectors/src/connectors/connector_name/test.rs`, which needs to be moved to `crates/router/tests/connectors/connector_name.rs`.

3.  **Request & Response Types (`transformers.rs`):**
    *   Define Rust structs representing the connector's specific API request and response formats.
    *   Implement `TryFrom` to convert Hyperswitch's internal `RouterData` (e.g., `PaymentsAuthorizeRouterData`) into the connector-specific request structs.
        *   Example: `impl TryFrom<&BillwerkRouterData<&types::PaymentsAuthorizeRouterData>> for BillwerkPaymentsRequest`
    *   Minimal data should be sent; optional fields can be ignored if not essential.
    *   Handle potential errors during transformation (e.g., missing required fields, unsupported features like 3DS for a basic integration).

4.  **Response Mapping (`transformers.rs`):**
    *   Define a connector-specific enum for payment status (e.g., `BillwerkPaymentState`).
    *   Implement `From<ConnectorPaymentState> for enums::AttemptStatus` to map the connector's status to Hyperswitch's standard `AttemptStatus`.
        *   **Important:** Default status should be `Pending`. Only explicit success or failure from the connector should map to `Charged` or `Failure`.
    *   Define Rust structs for the connector's response.
    *   Implement `TryFrom<ResponseRouterData<F, ConnectorResponse, T, PaymentsResponseData>> for RouterData<F, T, PaymentsResponseData>` to convert the connector's response back into Hyperswitch's `RouterData`.
        *   This involves populating `PaymentsResponseData` (with `resource_id`, `redirection_data`, etc.) and potentially an `ErrorResponse`.

5.  **Recommended Fields:**
    *   `connector_request_reference_id`: Merchant's reference ID.
    *   `connector_response_reference_id`: Connector's transaction ID or reference for their dashboard.
    *   `resource_id`: Typically the connector's transaction ID (`ResponseId::ConnectorTransactionId`).
    *   `redirection_data`: For flows requiring redirection (e.g., 3DS).

6.  **Error Handling:**
    *   Define a struct for the connector's error response format (e.g., `BillwerkErrorResponse`).
    *   The `build_error_response` method in `ConnectorCommon` trait handles parsing this error struct and mapping it to Hyperswitch's `ErrorResponse`.

7.  **Implementing Core Traits (`<connector_name>.rs`):**
    *   **`ConnectorCommon`**:
        *   `id()`: Returns the connector's string identifier.
        *   `get_currency_unit()`: Specifies if the connector uses `Base` or `Minor` currency units.
        *   `common_get_content_type()`: Defines the `Content-Type` for API requests (e.g., "application/json").
        *   `get_auth_header()`: Constructs the `Authorization` header based on `ConnectorAuthType`.
        *   `base_url()`: Fetches the connector's base API URL from configuration.
        *   `build_error_response()`: Parses connector-specific error responses.
    *   **`ConnectorIntegration<Flow, Request, Response>`**: Implemented for each payment flow (Authorize, Capture, Sync, RefundExecute, RefundSync, etc.).
        *   `get_url()`: Returns the specific API endpoint URL for the flow.
        *   `get_headers()`: Returns HTTP headers for the request (often uses `ConnectorCommonExt::build_headers`).
        *   `get_request_body()`: Transforms `RouterData` into the connector's request body.
        *   `build_request()`: Assembles the full `Request` object.
        *   `handle_response()`: Transforms the connector's `Response` back into `RouterData`.
        *   `get_error_response()`: Handles error responses for the specific flow (often delegates to `ConnectorCommon::build_error_response`).
    *   **`ConnectorCommonExt`**:
        *   `build_headers()`: A helper to combine common headers and auth headers.
    *   **Flow-Specific Traits**: `PaymentAuthorize`, `PaymentSync`, `PaymentCapture`, `RefundExecute`, `RefundSync`, etc., which specialize `ConnectorIntegration`.

8.  **Currency Unit Handling:**
    *   If a connector uses `Base` currency units, transformation logic (e.g., using `utils::get_amount_as_string`) is needed in the request transformer.

9.  **Utility Functions:**
    *   Leverage utility functions from `types::api` and the router data itself (e.g., `get_billing_country`, `is_auto_capture`).

10. **Control Center Configuration:**
    *   Update `crates/connector_configs/toml/development.toml` with auth details and other configurations for the new connector.
    *   Run `wasm-pack build ...` for `euclid_wasm` to update Control Center components.
    *   Update `ConnectorTypes.res` and `ConnectorUtils.res` in the Control Center codebase.
    *   Add the connector icon SVG to `public/hyperswitch/Gateway`.

11. **Testing (`crates/router/tests/connectors/<connector_name>.rs`):**
    *   Implement sanity tests provided by the template.
    *   Utilize helper functions in `tests/connector/utils`.
    *   Configure API keys in `sample_auth.toml` and set `CONNECTOR_AUTH_FILE_PATH` environment variable for testing.

12. **(Optional) Build Request/Response from JSON Schema:**
    *   Use `openapi-generator` to generate Rust models from a connector's OpenAPI/JSON schema if available.

**Overall Pattern:**
The integration process emphasizes a clear separation of concerns:
*   **`transformers.rs`**: Handles data structure definitions and transformations between Hyperswitch's internal models and the connector's specific formats.
*   **`<connector_name>.rs`**: Implements the core API interaction logic by fulfilling the required traits (`ConnectorCommon`, `ConnectorIntegration` for various flows).
*   Configuration is managed externally.
*   Testing is a critical part of the process.

---

## Stripe Connector Analysis (Connector: `stripebilling`)

**Last Updated:** 2025-05-14

The primary integration for Stripe functionalities within Hyperswitch appears to be handled by the `stripebilling` connector module, located in `crates/hyperswitch_connectors/src/connectors/stripebilling.rs` and its associated `transformers.rs` module. The main `connectors.rs` file in the `hyperswitch_connectors` crate only declares and exports `stripebilling::Stripebilling`; a separate `stripe.rs` module for general payments was not found. Thus, the `Stripe` variant in the `common_enums::Connector` enum likely maps to this `stripebilling` implementation for core payment processing.

### `stripebilling.rs` (Main Logic)

*   **Structure:**
    *   Defines a unit struct `Stripebilling`.
    *   Implements a wide range of `hyperswitch_interfaces::api` traits:
        *   `ConnectorCommon`, `ConnectorCommonExt`
        *   Payment flow traits: `Payment`, `PaymentSession`, `PaymentAuthorize`, `PaymentSync`, `PaymentCapture`, `PaymentVoid`.
        *   Refund flow traits: `Refund`, `RefundExecute`, `RefundSync`.
        *   Other traits: `ConnectorAccessToken`, `MandateSetup`, `PaymentToken`.
        *   Conditional traits for "revenue_recovery" feature: `RevenueRecoveryRecordBack`, `BillingConnectorPaymentsSyncIntegration`.
        *   Webhook handling: `webhooks::IncomingWebhook`.
*   **Key Implementations:**
    *   `ConnectorCommon`:
        *   `id()`: Returns "stripebilling".
        *   `get_currency_unit()`: Returns `api::CurrencyUnit::Minor`.
        *   `base_url()`: Fetches from `connectors.stripebilling.base_url`.
        *   `get_auth_header()`: Uses `StripebillingAuthType` (expecting `HeaderKey` with `api_key`) and adds a `stripe-version` header.
        *   `build_error_response()`: Parses `StripebillingErrorResponse` from `transformers`.
    *   `ConnectorIntegration<Flow, Request, Response>`:
        *   Many flow-specific `get_url()` and `get_request_body()` methods are marked with `Err(errors::ConnectorError::NotImplemented(...).into())`. This suggests that either:
            *   "Stripe Billing" has a limited scope of implemented flows.
            *   A more general "Stripe" connector handles other payment operations, and its location is yet to be definitively identified.
            *   The integration is a work in progress for these specific flows.
        *   For implemented flows (e.g., `PaymentsAuthorize`, `RefundExecute`), it follows the standard pattern:
            *   `get_headers()` usually delegates to `build_headers()`.
            *   `get_request_body()` uses `StripebillingRouterData` and calls `TryFrom` on the corresponding request struct in `stripebilling::transformers`.
            *   `build_request()` assembles the HTTP request.
            *   `handle_response()` parses the response struct from `stripebilling::transformers` and converts it to `RouterData`.
*   **Webhook Handling:**
    *   Implements `get_webhook_source_verification_algorithm()` (HmacSha256).
    *   `get_webhook_source_verification_signature()` extracts signature from "stripe-signature" header.
    *   `get_webhook_source_verification_message()` constructs the message to verify.
    *   `get_webhook_object_reference_id()`, `get_webhook_event_type()`, `get_webhook_resource_object()` parse webhook content using structs from `stripebilling::transformers` (e.g., `StripebillingWebhookBody`, `StripebillingInvoiceBody`).
    *   Specific logic for revenue recovery webhooks.

### `stripebilling/transformers.rs`

*   **Purpose:** Defines Stripe Billing-specific request/response data structures and their conversions to/from Hyperswitch's generic router data types.
*   **Key Structs & Enums:**
    *   `StripebillingRouterData<T>`: Generic wrapper to bundle amount with router data.
    *   `StripebillingPaymentsRequest`: Contains amount and `StripebillingCard`.
    *   `StripebillingCard`: Card details (number, expiry, CVC).
    *   `StripebillingAuthType`: For API key authentication.
    *   `StripebillingPaymentStatus`: Maps Stripe Billing payment statuses (Succeeded, Failed, Processing) to `common_enums::AttemptStatus`.
    *   `StripebillingPaymentsResponse`: Contains status and ID.
    *   `StripebillingRefundRequest`: Contains amount for refund.
    *   `RefundStatus` (connector-specific): Maps to `enums::RefundStatus`.
    *   `RefundResponse`: Contains refund ID and status.
    *   `StripebillingErrorResponse`: Structure for error responses.
    *   Webhook-related structs: `StripebillingWebhookBody`, `StripebillingInvoiceBody`, `StripebillingEventType`, `StripebillingWebhookData`, `StripebillingWebhookObject`.
    *   Revenue recovery structs: `StripebillingRecoveryDetailsData`, `StripebillingChargeStatus`, `StripebillingPaymentMethodDetails`, `StripebillingRecordBackResponse`.
*   **Transformation Logic:**
    *   `TryFrom` implementations are central for converting:
        *   `PaymentsAuthorizeRouterData` to `StripebillingPaymentsRequest`.
        *   `StripebillingPaymentsResponse` back to `PaymentsAuthorizeRouterData` (via `ResponseRouterData`).
        *   `RefundsRouterData` to `StripebillingRefundRequest`.
        *   `RefundResponse` back to `RefundsRouterData`.
        *   Similar conversions for revenue recovery types.
    *   Handles specific payment method data (e.g., `PaymentMethodData::Card`).
    *   Uses `utils::convert_amount` for currency amount conversions if needed (though `Stripebilling` uses minor units, so direct use might be less frequent here compared to base unit connectors).
*   **Constants:**
    *   `auth_headers::STRIPE_API_VERSION` and `STRIPE_VERSION` define the Stripe API version used.
*   **Feature Flags:** Uses `#[cfg(all(feature = "v2", feature = "revenue_recovery"))]` for parts related to revenue recovery.

**Initial Observations & Implications:**

*   The `stripebilling.rs` module, despite its name, seems to serve as the main integration point for Stripe payment processing, not just for "Stripe Billing" as a separate product.
*   The `NotImplemented` errors in `stripebilling.rs` for certain flow-specific methods (like `get_url` or `get_request_body` for some operations) might indicate:
    *   Those specific Stripe API actions are not yet supported by this Hyperswitch integration.
    *   They are handled by default trait implementations provided elsewhere (e.g., in `crates/hyperswitch_connectors/src/default_implementations.rs`).
    *   The functionality is achieved through a different combination of implemented flows.
*   The analysis of `stripebilling.rs` and its transformers is effectively the analysis of the "Stripe" connector.

---

## Default Trait Implementations (`crates/hyperswitch_connectors/src/default_implementations.rs`)

**Last Updated:** 2025-05-14

This file plays a crucial role in code reuse and simplifying connector development by providing default implementations for many connector traits.

**Key Patterns & Purpose:**

1.  **Macro-Driven Default Implementations:**
    *   The file defines numerous macros (e.g., `default_imp_for_authorize_session_token!`, `default_imp_for_complete_authorize!`, `default_imp_for_payouts_create!`).
    *   Each macro is responsible for implementing a specific connector capability trait (e.g., `PaymentAuthorizeSessionToken`, `PaymentsCompleteAuthorize`, `PayoutCreate`) and the corresponding generic `ConnectorIntegration<Flow, Request, Response>` trait for a list of connectors.
    *   The implementations provided by these macros are typically "default" in the sense that they often:
        *   Provide an empty implementation for the capability trait (e.g., `impl PaymentAuthorizeSessionToken for SomeConnector {}`).
        *   For the `ConnectorIntegration` trait methods (like `get_url`, `get_request_body`, `handle_response`), they might return `Err(ConnectorError::NotImplemented("...".to_string()).into())` or provide a minimal stub.

2.  **Broad Application to Connectors:**
    *   Each macro is invoked with a long list of connector structs (e.g., `connectors::Aci`, `connectors::Adyen`, ..., `connectors::Stripebilling`, ...).
    *   This means that any connector included in these macro calls will automatically receive these default trait implementations *unless* it provides its own specific implementation for that trait/flow.

3.  **Mechanism for Opt-In Implementation:**
    *   This pattern allows new connectors to be scaffolded quickly. Developers only need to implement the traits and flows that are relevant or have custom logic for that specific connector.
    *   If a flow is not supported or not yet implemented for a connector, it will inherit the default behavior (often, returning a "Not Implemented" error). This explains why some methods in `stripebilling.rs` might appear unimplemented there but the code still compiles.

4.  **Reducing Boilerplate:**
    *   Without these default implementations, every connector would need to explicitly implement every single trait defined in `hyperswitch_interfaces::api`, even if just to state it's not supported. The macros centralize this boilerplate.

5.  **Conditional Compilation (`#[cfg(feature = ...)]`):**
    *   Some macros and their invocations are guarded by feature flags (e.g., `#[cfg(feature = "payouts")]`, `#[cfg(feature = "frm")]`, `#[cfg(all(feature = "v2", feature = "revenue_recovery"))]`).
    *   This ensures that default implementations for features like Payouts or Fraud Management are only compiled in if the corresponding feature is enabled for the build.

**Impact on Connector Development:**

*   When analyzing a specific connector (e.g., `stripebilling.rs`), if a particular trait implementation is not found directly in the connector's file, it's highly probable that it's using a default implementation from this file.
*   To add or customize a flow for a connector, a developer would provide a specific `impl Trait for ConnectorName` and/or `impl ConnectorIntegration<Flow, Req, Res> for ConnectorName` in the connector's own module file, overriding the default.

**Example Macro Usage:**
```rust
// Example structure (simplified)
macro_rules! default_imp_for_some_flow {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl api::SomeFlowTrait for $path::$connector {} // Empty or minimal impl
            impl ConnectorIntegration<SomeFlow, SomeRequest, SomeResponse> for $path::$connector {
                // Default methods, often returning Err(ConnectorError::NotImplemented)
            }
        )*
    };
}

default_imp_for_some_flow!(
    connectors::Aci,
    connectors::Adyen,
    // ... many other connectors
    connectors::Stripebilling
);
```
This file is essential for understanding the baseline capabilities (or lack thereof for unimplemented flows) of many connectors and how Hyperswitch manages the broad set of potential connector functionalities.

---

## Default Trait Implementations V2 (`crates/hyperswitch_connectors/src/default_implementations_v2.rs`)

**Last Updated:** 2025-05-14

This file mirrors the purpose of `default_implementations.rs` but is specifically for **V2** versions of the connector traits and integration points. It signifies an evolution or newer version of the connector interface within Hyperswitch.

**Key Patterns & Purpose (similar to V1 defaults, but for V2 traits):**

1.  **V2 Trait Focus:**
    *   Provides default implementations for traits suffixed with `V2` (e.g., `PaymentV2`, `RefundV2`, `ConnectorIntegrationV2`).
    *   These V2 traits likely involve updated function signatures, different generic type parameters for flow-specific data (e.g., `PaymentFlowData`, `RefundFlowData`, `AccessTokenFlowData` used with `ConnectorIntegrationV2`), or refined error handling/response types.

2.  **Macro-Driven Default Implementations (V2):**
    *   Uses a set of macros analogous to those in `default_implementations.rs` but tailored for V2 traits (e.g., `default_imp_for_new_connector_integration_payment!`, `default_imp_for_new_connector_integration_refund!`).
    *   These macros implement the V2 capability traits (e.g., `PaymentAuthorizeV2`) and the generic `ConnectorIntegrationV2<Flow, FlowData, Request, Response>` trait.
    *   The default implementations are typically stubs, often returning `Err(ConnectorError::NotImplemented(...).into())`.

3.  **Broad Application to Connectors:**
    *   The macros are invoked with extensive lists of connectors, applying these V2 default implementations widely.
    *   Connectors must explicitly implement V2 traits if they support the V2 interface and have specific logic; otherwise, they inherit these defaults.

4.  **Conditional Compilation for Features:**
    *   Similar to V1 defaults, feature flags (e.g., `#[cfg(feature = "payouts")]`, `#[cfg(feature = "frm")]`) are used to conditionally compile default implementations for V2 traits related to specific product features.

**Significance:**

*   The existence of `default_implementations_v2.rs` indicates a versioned approach to the connector integration framework, allowing for non-breaking changes and gradual adoption of newer interface designs.
*   Connectors can potentially support V1, V2, or both versions of an interface, depending on what they implement.
*   When analyzing a connector, it's important to check if it implements V1 or V2 (or both) versions of the relevant traits to understand its capabilities and how it interacts with the router.
*   This file, like its V1 counterpart, is crucial for reducing boilerplate and ensuring that connectors only need to implement the logic specific to their V2 integration.

---

## Core Connector Interfaces (`crates/hyperswitch_interfaces/src/api.rs`)

**Last Updated:** 2025-05-14

This file is central to the Hyperswitch connector architecture. It defines the primary traits and structures that govern how connectors interact with the core routing engine. It establishes a contract for all payment processor integrations.

**Key Components & Traits:**

1.  **Submodules for Functionalities:**
    *   The `api.rs` file declares various public submodules, each corresponding to a specific domain of connector interaction:
        *   `authentication` & `authentication_v2`
        *   `disputes` & `disputes_v2`
        *   `files` & `files_v2`
        *   `payments` & `payments_v2`
        *   `refunds` & `refunds_v2`
        *   `payouts` & `payouts_v2` (feature-gated)
        *   `fraud_check` & `fraud_check_v2` (feature-gated)
        *   `revenue_recovery` & `revenue_recovery_v2`
    *   This modular structure organizes traits and types related to each specific functionality.

2.  **`Connector` Trait (Marker Trait):**
    *   A top-level marker trait that aggregates numerous other capability-specific traits.
    *   `pub trait Connector: Send + Refund + Payment + ConnectorRedirectResponse + webhooks::IncomingWebhook + ConnectorAccessToken + disputes::Dispute + files::FileUpload + ConnectorTransactionId + Payouts + ConnectorVerifyWebhookSource + FraudCheck + ConnectorMandateRevoke + authentication::ExternalAuthentication + TaxCalculation + UnifiedAuthenticationService + revenue_recovery::RevenueRecovery {}`
    *   Any type that implements all these constituent traits automatically implements `Connector`. This serves as a blanket trait ensuring a connector supports a minimum set of functionalities.

3.  **`ConnectorCommon` Trait:**
    *   Defines methods and properties common to all connectors:
        *   `id(&self) -> &'static str`: Unique identifier for the connector.
        *   `get_currency_unit(&self) -> CurrencyUnit`: Specifies if the connector uses base or minor currency units.
        *   `get_auth_header(&self, auth_type: &ConnectorAuthType) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError>`: Constructs authentication headers.
        *   `common_get_content_type(&self) -> &'static str`: Default content type for requests.
        *   `base_url<'a>(&self, connectors: &'a Connectors) -> &'a str`: Base API URL.
        *   `build_error_response(&self, res: types::Response, event_builder: Option<&mut ConnectorEvent>) -> CustomResult<ErrorResponse, errors::ConnectorError>`: Parses connector-specific errors into a common `ErrorResponse`.

4.  **`ConnectorIntegration<T, Req, Resp>` Trait:**
    *   This is the core generic trait for defining the integration logic for specific payment flows.
    *   Generic Parameters:
        *   `T`: Flow type (e.g., `Authorize` from `router_flow_types::payments`).
        *   `Req`: Request data type for the flow (e.g., `PaymentsAuthorizeData`).
        *   `Resp`: Response data type for the flow (e.g., `PaymentsResponseData`).
    *   Provides default implementations for many methods, allowing connectors to override only what's necessary:
        *   `get_headers()`: HTTP headers for the request.
        *   `get_content_type()`, `get_accept_type()`: MIME types.
        *   `get_http_method()`: HTTP method (defaults to POST).
        *   `get_url()`: API endpoint URL for the flow.
        *   `get_request_body()`: Constructs the request payload.
        *   `build_request()`: Assembles the complete HTTP `Request` object.
        *   `handle_response()`: Processes the connector's response and maps it to `RouterData`.
        *   `get_error_response()`, `get_5xx_error_response()`: Error handling.
    *   Connectors implement this trait for each specific flow they support (e.g., `impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for MyConnector {}`).

5.  **`ConnectorCommonExt<Flow, Req, Resp>` Trait:**
    *   An extension trait providing helper methods, notably `build_headers()`, which combines common and auth headers.

6.  **Flow-Specific Traits:**
    *   Many traits are defined that specialize `ConnectorIntegration` for particular operations (e.g., `PaymentAuthorize`, `RefundExecute`, `WebhookSourceVerification`). These often act as marker traits or provide more specific type associations.
    *   Examples: `Payment`, `Refund`, `ConnectorAccessToken`, `Dispute`, `FileUpload`, `Payouts`, `FraudCheck`, `ConnectorMandateRevoke`, `ExternalAuthentication`, `TaxCalculation`, `UnifiedAuthenticationService`.

7.  **Versioning (V1 and V2):**
    *   The presence of `_v2` suffixed modules and the existence of `connector_integration_v2.rs` (implied by its use in `default_implementations_v2.rs`) indicate a versioned interface framework. This allows the connector APIs to evolve.
    *   The `ConnectorIntegrationV2` trait (defined in `connector_integration_v2.rs`) is the V2 counterpart to `ConnectorIntegration`.

**Overall Architecture:**
*   The `hyperswitch_interfaces` crate, particularly `api.rs` and its submodules, defines the abstract contract that all connectors must fulfill.
*   It promotes a consistent structure for connector implementations through a set of common and flow-specific traits.
*   Default implementations in `ConnectorIntegration` (and further defaults in `default_implementations.rs` / `default_implementations_v2.rs`) significantly reduce boilerplate for individual connectors.
*   The design supports versioning of interfaces (V1 and V2), allowing for flexibility and backward compatibility.

---

## Connector Interface Abstraction (`crates/hyperswitch_interfaces/src/connector_integration_interface.rs`)

**Last Updated:** 2025-05-14

This file introduces an important abstraction layer to manage and interact with both V1 and V2 connector implementations seamlessly. It defines enums and traits that act as wrappers or delegates to the appropriate version of a connector's implementation.

**Key Components & Patterns:**

1.  **`RouterDataConversion<T, Req, Resp>` Trait:**
    *   **Purpose:** Defines a contract for converting between the V1 `RouterData<T, Req, Resp>` and the V2 `RouterDataV2<T, Self, Req, Resp>`.
    *   **Methods:**
        *   `from_old_router_data()`: Converts V1 `RouterData` to V2 `RouterDataV2`.
        *   `to_old_router_data()`: Converts V2 `RouterDataV2` back to V1 `RouterData`.
    *   **Significance:** This trait is fundamental for interoperability, allowing the core system to use V2 connector logic even if the initial data conforms to the V1 `RouterData` structure, and vice-versa. `ResourceCommonData` in `ConnectorIntegrationV2` typically implements this.

2.  **`ConnectorEnum` Enum:**
    *   **Definition:** `pub enum ConnectorEnum { Old(BoxedConnector), New(BoxedConnectorV2) }`
        *   `BoxedConnector` is `Box<&'static (dyn Connector + Sync)>` (V1).
        *   `BoxedConnectorV2` is `Box<&'static (dyn ConnectorV2 + Sync)>` (V2).
    *   **Purpose:** Acts as a type-safe container that can hold either a V1 or a V2 connector implementation.
    *   **Behavior:** Implements various high-level connector traits (e.g., `IncomingWebhook`, `ConnectorValidation`, `ConnectorCommon`, `ConnectorTransactionId`, `ConnectorSpecifications`). These implementations typically `match` on the `Old` or `New` variant and delegate the call to the corresponding method of the wrapped V1 or V2 connector.
    *   **`get_connector_integration()` Method:** A crucial method on `ConnectorEnum` that returns a `BoxedConnectorIntegrationInterface`. This interface then handles the V1/V2 dispatch for specific flow integrations.

3.  **`ConnectorIntegrationEnum<'a, F, ResourceCommonData, Req, Resp>` Enum:**
    *   **Definition:** `pub enum ConnectorIntegrationEnum<'a, F, ResourceCommonData, Req, Resp> { Old(BoxedConnectorIntegration<'a, F, Req, Resp>), New(BoxedConnectorIntegrationV2<'a, F, ResourceCommonData, Req, Resp>) }`
    *   **Purpose:** Wraps either a V1 or V2 specific flow integration.

4.  **`ConnectorIntegrationInterface<F, ResourceCommonData, Req, Resp>` Trait:**
    *   **Purpose:** Defines a common interface for flow-specific operations, abstracting over V1 and V2 implementations.
    *   **Implementation:** `ConnectorIntegrationEnum` implements this trait.
    *   **Methods:** Includes methods like `build_request()`, `handle_response()`, `get_error_response()`, etc.
    *   **V1/V2 Dispatch Logic:**
        *   When a method is called on a `ConnectorIntegrationEnum::New` variant (i.e., a V2 connector), it first uses `ResourceCommonData::from_old_router_data()` to convert the input `RouterData` to `RouterDataV2`.
        *   It then calls the V2-specific method (e.g., `build_request_v2()`) on the wrapped `ConnectorIntegrationV2` object.
        *   If the V2 method returns `RouterDataV2`, it's converted back to V1 `RouterData` using `ResourceCommonData::to_old_router_data()`.
        *   For `ConnectorIntegrationEnum::Old`, it directly calls the V1 method.

**Overall Architecture & Significance:**

*   This interface layer (`ConnectorEnum`, `ConnectorIntegrationInterface`, `RouterDataConversion`) provides a unified way for the router to interact with connectors, regardless of whether they are implemented against the V1 or V2 traits.
*   It encapsulates the complexity of handling two different versions of connector interfaces and data structures.
*   The `RouterDataConversion` trait is the linchpin for data compatibility between V1 and V2 flows when using V2 connectors.
*   This design facilitates a gradual migration path from V1 to V2 connector implementations and allows both to coexist within the system.
*   It ensures that higher-level routing logic doesn't need to be aware of the specific version of a connector's implementation for most operations.

---

## Common Interface Types (`crates/hyperswitch_interfaces/src/types.rs`)

**Last Updated:** 2025-05-14

This file is crucial for defining common data structures and, most importantly, type aliases that simplify working with the generic `ConnectorIntegration` trait for various payment flows.

**Key Components & Patterns:**

1.  **`Response` Struct:**
    *   `pub struct Response { pub headers: Option<http::HeaderMap>, pub response: bytes::Bytes, pub status_code: u16 }`
    *   **Purpose:** A generic struct to hold the raw HTTP response received from a connector. This includes optional headers, the response body as `bytes::Bytes`, and the HTTP status code.
    *   **Usage:** This raw response is typically processed by the `handle_response` method of a `ConnectorIntegration` implementation to parse it into specific `RouterData` or `ErrorResponse` types.

2.  **Type Aliases for `ConnectorIntegration`:**
    *   The majority of this file consists of type aliases for `dyn ConnectorIntegration<Flow, RequestType, ResponseType>`.
    *   **Syntax Example:** `pub type PaymentsAuthorizeType = dyn ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData>;`
    *   **Purpose:**
        *   **Readability & Conciseness:** These aliases make the code that uses these specific integrations much cleaner and less verbose (e.g., using `Box<PaymentsAuthorizeType>` instead of the full `Box<dyn ConnectorIntegration<...>>` signature).
        *   **Centralization:** Defines these common integration signatures in one place.
    *   **Dynamic Dispatch:** The `dyn ConnectorIntegration<...>` syntax indicates the use of trait objects, enabling dynamic dispatch. This allows the system to work with different connector implementations for the same flow at runtime.

3.  **Comprehensive Flow Coverage:**
    *   The type aliases cover a vast range of connector functionalities, including but not limited to:
        *   **Payments:** `PaymentsAuthorizeType`, `PaymentsCaptureType`, `PaymentsSyncType`, `PaymentsSessionType`, `PaymentsVoidType`, `PaymentsInitType`, `PaymentsTaxCalculationType`, `PaymentsPreProcessingType`, `PaymentsPostProcessingType`, `IncrementalAuthorizationType`, etc.
        *   **Mandates:** `SetupMandateType`, `MandateRevokeType`.
        *   **Tokenization:** `TokenizationType`.
        *   **Customer Management:** `ConnectorCustomerType`.
        *   **Refunds:** `RefundExecuteType`, `RefundSyncType`.
        *   **Payouts (feature-gated):** `PayoutCreateType`, `PayoutCancelType`, `PayoutSyncType`, etc.
        *   **Access Tokens:** `RefreshTokenType`.
        *   **Disputes:** `AcceptDisputeType`, `SubmitEvidenceType`, `DefendDisputeType`.
        *   **Webhooks:** `VerifyWebhookSourceType`.
        *   **File Handling:** `UploadFileType`, `RetrieveFileType`.
        *   **Unified Authentication Service (UAS):** `UasPreAuthenticationType`, `UasPostAuthenticationType`, `UasAuthenticationConfirmationType`, `UasAuthenticationType`.
        *   **Revenue Recovery:** `RevenueRecoveryRecordBackType`, `BillingConnectorPaymentsSyncType`.

4.  **Dependency on `hyperswitch_domain_models`:**
    *   All specific flow markers (e.g., `Authorize`, `Capture`), request data structs (e.g., `PaymentsAuthorizeData`), and response data structs (e.g., `PaymentsResponseData`) are imported from the `hyperswitch_domain_models` crate. This highlights `hyperswitch_domain_models` as the provider of core data structures for connector interactions.

5.  **Feature Gating:**
    *   Type aliases related to payouts (e.g., `PayoutCreateType`) are conditionally compiled using `#[cfg(feature = "payouts")]`, maintaining consistency with how optional features are handled elsewhere in the codebase.

**Significance:**
*   This file significantly simplifies the way developers interact with the `ConnectorIntegration` trait by providing clear, concise aliases for each specific flow.
*   It reinforces the pattern of using trait objects for dynamic dispatch to connector implementations.
*   It clearly shows the breadth of operations supported by the connector interface framework.

---

## V2 Connector Integration Core (`crates/hyperswitch_interfaces/src/connector_integration_v2.rs`)

**Last Updated:** 2025-05-14

This file defines the core traits for Version 2 (V2) of the connector integration framework. It parallels `api.rs` for V1 but introduces refinements and new patterns for how connectors interact with the system, particularly through the `ConnectorIntegrationV2` trait.

**Key Components & Traits:**

1.  **`ConnectorV2` Trait:**
    *   **Purpose:** An aggregator (marker) trait for V2 connectors, similar to the V1 `Connector` trait.
    *   **Definition:** `pub trait ConnectorV2: Send + api::refunds_v2::RefundV2 + api::payments_v2::PaymentV2 + ... {}`
    *   **Composition:** It groups numerous V2-specific capability traits (e.g., `api::refunds_v2::RefundV2`, `api::payments_v2::PaymentV2`, `api::ConnectorAccessTokenV2`, etc., which are themselves defined in submodules of `api.rs` but represent the V2 versions).
    *   **Implementation:** A blanket `impl<T: ...> ConnectorV2 for T {}` ensures any type implementing all constituent V2 traits automatically implements `ConnectorV2`.
    *   **Significance:** Defines the comprehensive contract for a fully-featured V2 connector.

2.  **`BoxedConnectorV2` Type Alias:**
    *   `pub type BoxedConnectorV2 = Box<&'static (dyn ConnectorV2 + Sync)>;`
    *   A convenient type alias for a boxed, static reference to a V2 connector trait object.

3.  **`ConnectorIntegrationV2<Flow, ResourceCommonData, Req, Resp>` Trait:**
    *   **Purpose:** The V2 counterpart to the V1 `ConnectorIntegration` trait. This is the core generic trait for defining the integration logic for specific payment flows in the V2 framework.
    *   **Generic Parameters:**
        *   `Flow`: The specific flow type (e.g., `Authorize` from `hyperswitch_domain_models::router_flow_types`).
        *   `ResourceCommonData`: **New in V2.** This generic type is crucial. It's expected to hold common data for the resource being processed and, importantly, must implement the `RouterDataConversion` trait (from `connector_integration_interface.rs`). This allows conversion between V1 `RouterData` and V2 `RouterDataV2` structures, enabling interoperability.
        *   `Req`: The request data type for the flow (e.g., `PaymentsAuthorizeData`).
        *   `Resp`: The response data type for the flow (e.g., `PaymentsResponseData`).
    *   **Supertraits/Dependencies:**
        *   `ConnectorIntegrationAnyV2<Flow, ResourceCommonData, Req, Resp>`: A helper trait primarily for boxing `ConnectorIntegrationV2` trait objects.
        *   `Sync`: Standard Rust marker trait.
        *   `api::ConnectorCommon`: **Notably, V2 integrations still depend on the V1 `ConnectorCommon` trait.** This means fundamental connector details like `id()`, `base_url()`, `get_auth_header()`, and `build_error_response()` (for V1 error parsing) are shared between V1 and V2 implementations.
    *   **Key Methods (with default implementations):**
        *   `get_headers()`: Returns HTTP headers for the request.
        *   `get_content_type()`: Defaults to "application/json".
        *   `get_http_method()`: Defaults to `Method::Post`.
        *   `get_url()`: API endpoint URL for the flow. Default returns an empty string and logs "UNIMPLEMENTED_FLOW".
        *   `get_request_body()`: Constructs the request payload. Default returns `Ok(None)`.
        *   `get_request_form_data()`: For form data. Default returns `Ok(None)`.
        *   `build_request_v2()`: Assembles the complete HTTP `Request` object using the above methods. This is the V2 equivalent of `build_request()` in V1.
        *   `handle_response_v2()`: Processes the connector's raw `types::Response` and maps it to `RouterDataV2`. Default implementation clones the input `RouterDataV2` and logs "Not Implemented".
        *   `get_error_response_v2()`: Handles generic error responses for V2. Default returns `ErrorResponse::get_not_implemented()`.
        *   `get_5xx_error_response()`: Provides more structured error parsing for 5xx HTTP status codes.
        *   `get_multiple_capture_sync_method()`: For connectors supporting multiple capture sync.
        *   `get_certificate()`, `get_certificate_key()`: For mTLS client certificate authentication.
    *   **Data Types:** Methods primarily operate on `RouterDataV2<Flow, ResourceCommonData, Req, Resp>`.

4.  **`ConnectorIntegrationAnyV2` Trait & `BoxedConnectorIntegrationV2` Type Alias:**
    *   These are helper constructs for creating and using boxed trait objects of `ConnectorIntegrationV2`, facilitating dynamic dispatch.

**Key Differences and Implications of V2 vs. V1 Integration:**

*   **`ResourceCommonData` and `RouterDataV2`:** The introduction of `ResourceCommonData` as a generic parameter in `ConnectorIntegrationV2` and the use of `RouterDataV2` are the most significant architectural changes. `ResourceCommonData`'s role in `RouterDataConversion` is pivotal for the V1/V2 abstraction layer managed by `connector_integration_interface.rs`.
*   **Method Naming Suffix `_v2`:** V2-specific methods like `build_request_v2` and `handle_response_v2` clearly distinguish them from their V1 counterparts.
*   **Shared `ConnectorCommon`:** The V2 integration still relies on the V1 `api::ConnectorCommon` trait for basic connector identification, base URLs, and V1-style error parsing. This indicates an evolutionary approach, where V2 builds upon or refines aspects of V1 rather than completely replacing all parts.
*   **Default Implementations:** Similar to V1, `ConnectorIntegrationV2` provides default implementations for most methods, often returning "Not Implemented" errors. These are then broadly applied to all connectors via macros in `crates/hyperswitch_connectors/src/default_implementations_v2.rs`, reducing boilerplate for individual connector developers.

**Significance:**
*   This file establishes the refined contract for V2 connector integrations, allowing for more structured data handling (via `RouterDataV2` and `ResourceCommonData`) and potentially new capabilities.
*   It is a core piece of Hyperswitch's versioned interface strategy, enabling the system to evolve its internal APIs while maintaining support for older connector implementations and providing a clear path for new ones.
*   The interaction between `ConnectorIntegrationV2`, `ResourceCommonData` (and its `RouterDataConversion` impl), and the V1/V2 dispatch logic in `connector_integration_interface.rs` is central to understanding how Hyperswitch manages its diverse and evolving set of connector integrations.
