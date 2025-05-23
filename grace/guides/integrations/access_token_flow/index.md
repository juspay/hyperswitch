# Access Token Flow Information

This document consolidates information related to the Access Token flow, extracted from various guides.

## From grace/guides/learning/learning.md

### Topic: Import Paths and Type Aliases (Airwallex Implementation Comparison)
(Relevant parts mentioning AccessTokenAuthRouterData, AccessTokenResponseRouterData, AccessTokenAuthType)
In `crates/hyperswitch_connectors/src/connectors/airwallex/transformers.rs`:
\`\`\`rust
// hyperswitch_types is an alias for hyperswitch_domain_models::types
use hyperswitch_types::{
    // ...
    AccessTokenResponseRouterData, 
};
\`\`\`
In `crates/hyperswitch_connectors/src/connectors/airwallex.rs`:
\`\`\`rust
use hyperswitch_domain_models::{
    types::{ 
        AccessTokenAuthRouterData, 
        // ...
    },
};
use hyperswitch_interfaces::{
    api::{self, payments::AccessTokenAuthType as AccessTokenAuthTypeTrait}, // Correct path and alias for the trait
};

// In AccessTokenAuth flow:
// .url(&AccessTokenAuthTypeTrait::get_url(self, req, connectors)?) // Using the aliased trait
\`\`\`
#### Differences:
2.  **`AccessTokenResponseRouterData`**: Correctly imported from `hyperswitch_types` (alias for `hyperswitch_domain_models::types`) in the `transformers.rs` eventually, but initial attempts might have been from `crate::types`.
4.  **`AccessTokenAuthRouterData`**: This is a type alias in `hyperswitch_domain_models::types`.
6.  **`AccessTokenAuthType` Trait**: This trait is located in `hyperswitch_interfaces::api::payments`, not directly under `hyperswitch_interfaces::api` or `hyperswitch_interfaces::types`.
#### Lessons Learned:
5.  **Trait vs. Struct**: `AccessTokenAuthType` is a trait that defines behavior (like `get_url`), while `AccessTokenAuthRouterData` is a struct (actually a type alias for `RouterData<AccessTokenAuth, ...>`) that holds data. They are used differently.

### `AccessTokenResponseRouterData` and Orphan Rules (Advanced Learnings from Real Codebase - Airwallex Example)
*   **Observation**: `AccessTokenResponseRouterData` is NOT a public type alias in `hyperswitch_domain_models::types` (or its alias `hyperswitch_types`).
*   **Solution Pattern (from `real-codebase/airwallex/transformers.rs` for `AirwallexAuthUpdateResponse`):**
    *   Example for Access Token:
        \`\`\`rust
        // In transformers.rs
        use crate::types::ResponseRouterData; // This is hyperswitch_connectors::types::ResponseRouterData
        use hyperswitch_domain_models::{
            router_data::{AccessToken, RouterData as HyperswitchRouterData},
            router_flow_types::access_token_auth::AccessTokenAuth,
            router_request_types::AccessTokenRequestData,
        };
        // ... other imports ...
        // pub struct AirwallexAuthUpdateResponse { /* ... */ }

        impl TryFrom<ResponseRouterData<AccessTokenAuth, AirwallexAuthUpdateResponse, AccessTokenRequestData, AccessToken>>
            for HyperswitchRouterData<AccessTokenAuth, AccessTokenRequestData, AccessToken>
        {
            type Error = error_stack::Report<errors::ConnectorError>;
            fn try_from(
                item: ResponseRouterData<AccessTokenAuth, AirwallexAuthUpdateResponse, AccessTokenRequestData, AccessToken>,
            ) -> Result<Self, Self::Error> {
                // 'item.response' here is AirwallexAuthUpdateResponse directly, not Result<>
                // 'item.data' is the original RouterData passed into crate::types::ResponseRouterData
                Ok(Self {
                    response: Ok(AccessToken {
                        token: item.response.token, // Assuming token is a field in AirwallexAuthUpdateResponse
                        expires: (item.response.expires_at - common_utils::date_time::now()).whole_seconds(), // Example calculation
                    }),
                    ..item.data // Spread fields from the original RouterData
                })
            }
        }
        \`\`\`

### `AccessTokenAuthType` Trait (Advanced Learnings from Real Codebase - Airwallex Example)
*   **Observation**: The trait `AccessTokenAuthType` was not found at `hyperswitch_interfaces::api::payments`. The access token flow is primarily managed by `impl api::ConnectorAccessToken for ConnectorName {}` and the corresponding `impl ConnectorIntegration<AccessTokenAuth, ...> for ConnectorName {}`.
*   **Usage in `build_request`**: Calls to `get_url`, `get_headers`, `get_request_body` within the `AccessTokenAuth` flow's `build_request` method (if these are implemented directly in the `ConnectorIntegration` block) should be `self.method_name(...)`.
*   **Lesson**: Avoid importing or using non-existent helper traits. Rely on the methods defined in the direct `ConnectorIntegration` implementation.

### `RequestContent::Empty` (Advanced Learnings from Real Codebase - Airwallex Example)
*   **Correction (21/05/2025)**: For Airwallex `AccessTokenAuth` (login), the API expects an empty JSON object `{}`. So, `RequestContent::Json(Box::new(AirwallexAuthUpdateRequest {}))` (where `AirwallexAuthUpdateRequest` is an empty struct deriving `Serialize`) is more appropriate than `RequestContent::Empty` if `Empty` means no body at all.

### `ConnectorAuthType` and Bearer Tokens (Advanced Learnings from Real Codebase - Airwallex Example)
*   **Observation**: `ConnectorAuthType` (from `hyperswitch_domain_models::router_data`) does *not* have an inherent `get_access_token()` method, nor a `TokenAuth` variant.
*   **Lesson**:
    *   After a successful `AccessTokenAuth` flow, the obtained `AccessToken` should be stored in `RouterData.access_token`.
    *   The `RouterData.connector_auth_type` should remain unchanged (e.g., as `BodyKey`), as it holds the initial credentials.
    *   For subsequent authenticated API calls, the `ConnectorCommonExt::build_headers` method should retrieve the Bearer token from `RouterData.access_token` (if the flow is not `AccessTokenAuth` itself).
    *   The `AccessTokenAuth::get_headers` method should use the initial credentials from `req.connector_auth_type` (e.g. API key, client ID) to make the login request.

### `RouterData` Construction in `AccessTokenAuth::handle_response` (Advanced Learnings from Real Codebase - Airwallex Example)
*   **Lesson**: The `AccessTokenAuth::handle_response` should return an owned `RouterData` (e.g. `RefreshTokenRouterData`). This `RouterData` should have its `response` field set to `Ok(AccessToken { ... })` and its `access_token` field set to `Some(AccessToken { ... })`. The `connector_auth_type` field should be copied from the input `data.connector_auth_type` and *not* changed to a non-existent `TokenAuth` variant. Other fields should be copied from the input `data` as appropriate for the `RefreshTokenRouterData` structure. Avoid populating fields not present in the generic `RouterData` or `RefreshTokenRouterData` definition.
*   **Correction (21/05/2025)**: When constructing `RefreshTokenRouterData`, ensure all fields are initialized, typically by using `..data.clone()` and then overriding only the necessary fields (`response`, `access_token`, `connector_http_status_code`). Removed the attempt to set `status: common_enums::AttemptStatus::Tokenized` as it's not a valid variant and status should be preserved or handled generically.

## From grace/guides/types/types.md

### Authentication (Airwallex Connector Type Mappings)
*   **`AirwallexAuthType`**: Extracts `x_api_key` and `x_client_id` from Hyperswitch's `ConnectorAuthType::BodyKey`. These are used as headers for API calls.
*   **Access Token Management**: Airwallex uses bearer tokens for payment processing.
    *   An initial request is made to obtain an access token.
    *   `AirwallexAuthUpdateResponse` (containing `token` and `expires_at`) is transformed into Hyperswitch's `AccessToken` model.

## From grace/guides/integrations/integrations.md

### Authentication Mechanisms (Learnings from Analyzing Existing Hyperswitch Connectors)
    *   **OAuth or Custom Token Flow for Bearer Token**: Connectors like Paypal and Airwallex first obtain an access token.
        *   Paypal uses a standard OAuth-like flow.
        *   Airwallex uses a custom login endpoint (`/authentication/login`) with `X-API-KEY` and `X-CLIENT-ID` headers to get a Bearer token, which is then used for subsequent API calls in an `Authorization: Bearer <token>` header.
    *   **Globalpay**: Implements an access token flow. `GlobalpayAuthType` stores `app_id` and `key`. To get an access token, a request is made to `/accesstoken` with `app_id`, a `nonce`, and a `secret` (SHA512 hash of `nonce + key`). Subsequent API calls use `Authorization: Bearer <access_token>` and an `X-GP-Version` header.
    *   **Paypal**: Implements an OAuth 2.0 client credentials flow to obtain a Bearer token. `PaypalAuthType` can be `StandardIntegration` (client_id, client_secret) or `PartnerIntegration` (client_id, client_secret, payer_id). The access token is then used in `Authorization: Bearer <token>` header. Additional headers like `PayPal-Partner-Attribution-Id`, `PayPal-Request-Id`, and `Prefer` are also used. For partner integrations, a `PayPal-Auth-Assertion` header is constructed.

### `hyperswitch_domain_models` (Commonly Used Hyperswitch Types and Utilities)
    *   `router_data::{RouterData, ConnectorAuthType, ErrorResponse, AccessToken}`: Core data carriers for flows.
