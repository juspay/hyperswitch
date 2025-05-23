# Preprocessing Flow Information

This document consolidates information related to the Preprocessing flow, extracted from various guides.

## From grace/guides/learning/learning.md

### Topic: Import Paths and Type Aliases (Airwallex Implementation Comparison)
(Relevant parts mentioning PaymentsPreProcessingRouterData)
In `crates/hyperswitch_connectors/src/connectors/airwallex/transformers.rs`:
\`\`\`rust
// hyperswitch_types is an alias for hyperswitch_domain_models::types
use hyperswitch_types::{
    PaymentsAuthorizeRouterData, PaymentsCaptureRouterData, PaymentsCancelRouterData,
    PaymentsCompleteAuthorizeRouterData, PaymentsPreProcessingRouterData, PaymentsSyncRouterData,
    AccessTokenResponseRouterData, 
};
\`\`\`

### Request Data Accessor Traits (Advanced Learnings from Real Codebase - Airwallex Example)
- **Observation**: Methods like `get_amount()`, `get_currency()`, `get_browser_info()`, `get_router_return_url()` are provided by traits defined in `crate::utils` (e.g., `PaymentsAuthorizeRequestData`, `PaymentsPreProcessingRequestData`).
- **Import**: These traits must be imported into the scope where the methods are used (typically `transformers.rs`).
    \`\`\`rust
    // Example in transformers.rs
    use crate::utils::{PaymentsAuthorizeRequestData, PaymentsPreProcessingRequestData /*, etc. */};
    \`\`\`

### Connector Transaction ID Source (`RouterData.reference_id`) (Advanced Learnings from Real Codebase - Airwallex Example)
- **Observation**: For flows like Authorize, Capture, Void, CompleteAuthorize that operate on an existing payment intent (created during PreProcessing), the `connector_transaction_id` (which is the Airwallex `payment_intent_id`) is stored in `RouterData.reference_id`.

### Amount Representation in `AirwallexIntentRequest` (Advanced Learnings from Real Codebase - Airwallex Example)
*   **Observation**: The `real-codebase`'s `AirwallexIntentRequest.amount` is `String`. It converts the `i64` amount from `PaymentsPreProcessingData` to a minor unit string using `utils::to_currency_base_unit`.
*   **Lesson**: My `AirwallexIntentRequest.amount` was `StringMinorUnit`. Changing it to `String` and using `crate::utils::to_currency_base_unit` aligns with `real-codebase` and resolves type errors.

## From grace/guides/types/types.md

### Core Request Transformation (Payments) (Airwallex Connector Type Mappings)
Hyperswitch's `PaymentsAuthorizeRouterData` is transformed into an `AirwallexPaymentsRequest`. This often involves a two-step process:
1.  **Intent Creation (Preprocessing)**: `PaymentsPreProcessingRouterData` is transformed into `AirwallexIntentRequest`.
    *   `request_id`: A UUID for the request.
    *   `amount`: Converted to currency base unit string.
    *   `currency`.
    *   `merchant_order_id`: Hyperswitch's `connector_request_reference_id`.
    *   `referrer_data`: Static data (`type: "hyperswitch"`, `version: "1.0.0"`) identifying Hyperswitch.
    This step typically returns an `intent_id` and `client_secret` from Airwallex, which are then used in the confirmation step.

## From grace/guides/integrations/integrations.md

### Request Structure - Intent/Confirm Pattern (Learnings from Analyzing Existing Hyperswitch Connectors)
    *   Some connectors like Airwallex (and Stripe) use a two-step process: first create a "Payment Intent" (or equivalent) which returns an intent ID and often a client secret. Then, a second call is made to "confirm" this intent with payment details.
    *   For Airwallex, PreProcessing step creates an intent (`AirwallexIntentRequest` to `/payment_intents/create`). The Authorize step then confirms this intent using the `intent_id` in the URL path (`AirwallexPaymentsRequest` to `/payment_intents/{intent_id}/confirm`).

### Flow: `PreProcessing` (Shift4 Connector - Part of Request/Response Patterns)
        *   For 3DS, a `PreProcessing` step is made to the `/3d-secure` endpoint using `Cards3DSRequest` (containing card details and `return_url`). This returns a `Shift4ThreeDsResponse` with `enrolled` status, `redirectUrl`, and a `token`.
        *   The `CompleteAuthorize` step then uses this `token` (from `connector_metadata`) in a `CardsNon3DSRequest` (specifically `CardPayment::CardToken`) to the `/charges` endpoint.

### Flow: `PreProcessing` (Cybersource Connector - Part of Request/Response Patterns)
        *   For 3DS, an initial `Authorize` call with card details might go to `/risk/v1/authentication-setups` returning an `access_token`, `device_data_collection_url`, and `reference_id` (stored in `RedirectForm::CybersourceAuthSetup`).
        *   A `PreProcessing` step then uses this `reference_id` and `return_url` in a `CybersourceAuthEnrollmentRequest` to `/risk/v1/authentications`. This can return a `step_up_url` (for challenge, stored in `RedirectForm::CybersourceConsumerAuth`) or directly provide 3DS validation data.
        *   If a challenge occurred, another `PreProcessing` step uses the `transaction_id` from the challenge redirect in a `CybersourceAuthValidateRequest` to `/risk/v1/authentication-results`.
        *   The 3DS validation data (CAVV, XID, etc., stored in `connector_metadata` as `CybersourceThreeDSMetadata`) is then used in the final `CompleteAuthorize` call to `/pts/v2/payments/` using `CybersourcePaymentsRequest`.
