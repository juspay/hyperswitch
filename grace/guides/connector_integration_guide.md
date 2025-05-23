# Hyperswitch Connector Integration: Step-by-Step Guide

This guide provides a reusable, step-by-step process for accurately adding a new payment connector to the Hyperswitch system. It synthesizes information from the "Hyperswitch Connector Integration Assistant" and the general "Connector Integration Process".

## Phase A: Preparation & Setup
[IMPORTANT]
[If resuming an existing integration, consult `grace/connector_integration/{connector}/planner-steps.md` to determine the current progress (e.g., by identifying the last completed step or phase). Summarize this status for the user and confirm whether to proceed with the subsequent integration tasks as outlined in the `planner-steps.md`.]

### Preparation 
### Step 0: Generating Connector-Specific Prompt Templates
[IMPORTANT]
[This step can be ignored if already created eg. if grace/connector_integration/{connector}/planner-steps.md & tech-specs.md is already created, this step can be skipped]

[IMPORTANT]
you have to add all the mandatory and required fields from the reference docs !!!. Do not miss any thing and do not include any headers structs

If any of the fields params are of STRING type and we have a fixed set of values for it then use enums in that case
"serde_rename_all_mapping": {
    "camelCase": "myFieldNameExample",
    "PascalCase": "MyFieldNameExample",
    "snake_case": "my_field_name_example",
    "SCREAMING_SNAKE_CASE": "MY_FIELD_NAME_EXAMPLE",
    "kebab-case": "my-field-name-example",
    "SCREAMING-KEBAB-CASE": "MY-FIELD-NAME-EXAMPLE (not official)"
}

When tasked with preparing for a new connector integration by generating its specific `planner-steps.md` and `tech-spec.md` prompts:

1.  **Contextual Awareness (Mandatory)**:
    *   First, ensure full context from all Memory Bank files (`projectbrief.md`, `productContext.md`, `activeContext.md`, `systemPatterns.md`, `techContext.md`, `progress.md`) is loaded and understood.
    *   Second, ensure the content of `grace/guide/connector_integration_guide.md` is loaded and understood.
2.  **Information Gathering for New Connector**:
    *   Obtain the new connector's name, API documentation URL, and any other relevant initial technical specifications or requirements from the user.
3.  **Directory and File Setup**:
    *   Create a new directory: `connector_integration/{connector_name}/` (e.g., `connector_integration/newconnector/`).
    *   Copy the base template `connector_integration/template/planner-steps.md` to the new `connector_integration/{connector_name}/planner-steps.md`.
    *   Copy the base template `connector_integration/template/tech-spec.md` to the new `connector_integration/{connector_name}/tech-spec.md`.
4.  **Template Population**:
    *   Systematically populate the newly copied `planner-steps.md` and `tech-spec.md` files.
    *   Replace all generic placeholders (e.g., `{{CONNECTOR_NAME}}`, `{{connector-name-lowercase}}`, `{{CONNECTOR_API_DOCS_URL}}`) with the specific details of the new connector.
    *   Utilize the information gathered from the connector's API documentation and the provided tech specs to fill in relevant sections, such as authentication mechanisms, API endpoint details, data structures, and configuration specifics, within the structure of the copied templates.
    *   The goal is to make these prompt files highly specific to the new connector, ready to guide subsequent AI planning and code generation tasks for that particular integration.

This structured approach ensures that the generation of these initial planning and specification documents for a new connector is consistent, leverages all established project knowledge and patterns, and is tailored effectively to the target connector.

- Established a clear, atomic, multi-step workflow for creating connector-specific planning and technical specification prompts:
    1. Load full Memory Bank context.
    2. Load `guide/connector_integration_guide.md` context.
    3. Create `connector_integration/{connector_name}/`.
    4. Copy `connector_integration/template/planner-steps.md` and `connector_integration/template/tech-spec.md` into the new folder.
    5. Populate the copied templates using the new connector's technical specifications and API documentation.
    This ensures that the generation of these crucial setup files is systematic and leverages all available project knowledge.

### Step 1: Prerequisites
-   **Connector API Knowledge**: Thoroughly understand the target connector's API documentation (endpoints, request/response formats, authentication, error codes).

    Memorize the below types and import accordingly
    ``` 
    // Std / Built-in
    use time::PrimitiveDateTime;
    use uuid::Uuid;
    
    // External Crates
    use base64::Engine;
    use masking::{ExposeInterface, PeekInterface, Secret};
    use serde::{Deserialize, Serialize};
    use url::Url;
    
    // Common/Internal Utilities
    use common_enums::{enums, enums::AuthenticationType, Currency};
    use common_utils::{
        consts::{self, BASE64_ENGINE},
        date_time,
        errors::CustomResult,
        ext_traits::ValueExt,
        pii::{self, Email, IpAddress},
        request::Method,
        types::{MinorUnit, StringMajorUnit, StringMinorUnit},
    };
    
    // Project Modules - Domain Models
    use hyperswitch_domain_models::{
        payment_method_data::{
            BankDebitData, BankRedirectData, BankTransferData, Card, CardRedirectData, GiftCardData,
            PayLaterData, PaymentMethodData, VoucherData, WalletData,
        },
        router_data::{
            AccessToken, AdditionalPaymentMethodConnectorResponse, ConnectorAuthType,
            ConnectorResponseData, ErrorResponse, KlarnaSdkResponse, PaymentMethodToken, RouterData,
        },
        router_flow_types::{
            payments::{Authorize, PostSessionTokens},
            refunds::{Execute, RSync},
            VerifyWebhookSource,
            #[cfg(feature = "payouts")]
            PoFulfill,
        },
        router_request_types::{
            BrowserInformation, CompleteAuthorizeData, PaymentsAuthorizeData, PaymentsCancelData,
            PaymentsCaptureData, PaymentsPostSessionTokensData, PaymentsPreProcessingData,
            PaymentsSetupMandateRequestData, PaymentsSyncData, ResponseId,
            SetupMandateRequestData, VerifyWebhookSourceRequestData,
        },
        router_response_types::{
            MandateReference, PaymentsResponseData, PayoutsResponseData, RedirectForm,
            RefundsResponseData, VerifyWebhookSourceResponseData, VerifyWebhookStatus,
        },
        types::{
            PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
            PaymentsCompleteAuthorizeRouterData, PaymentsPostSessionTokensRouterData,
            PaymentsPreProcessingRouterData, RefreshTokenRouterData, RefundsRouterData,
            SdkSessionUpdateRouterData, SetupMandateRouterData, VerifyWebhookSourceRouterData,
        },
    };
    
    // Project Modules - Interfaces
    use hyperswitch_interfaces::{consts, errors};
    
    // API Models
    use api_models::{
        enums,
        payments::{KlarnaSessionTokenResponse, SessionToken},
        webhooks::IncomingWebhookEvent,
        #[cfg(feature = "payouts")]
        payouts::{PayoutMethodData, Wallet as WalletPayout},
    };
    
    // Crate (local module) imports
    use crate::{
        constants,
        types::{
            PaymentsCaptureResponseRouterData, PaymentsResponseRouterData,
            PaymentsSessionResponseRouterData, PayoutsResponseRouterData, RefundsResponseRouterData,
            ResponseRouterData,
        },
        unimplemented_payment_method,
        utils::{
            self, missing_field_err, to_connector_meta, to_connector_meta_from_secret,
            AccessTokenRequestInfo, AddressData, AddressDetailsData, BrowserInformationData, CardData,
            CardData as CardDataUtil, ForeignTryFrom, PaymentMethodTokenizationRequestData,
            PaymentsAuthorizeRequestData, PaymentsCompleteAuthorizeRequestData,
            PaymentsPostSessionTokensRequestData, PaymentsPreProcessingRequestData,
            PaymentsSetupMandateRequestData, PaymentsSyncRequestData, RouterData as _,
            RouterData as OtherRouterData,
        },
    };
    ```
    For more types use `crates/hyperswitch_domain_models/**/types.rs` , `crates/common_utils/src`
    
-   **Sandbox Credentials**: Obtain API credentials (API keys, secrets, etc.) for the connector's sandbox/testing environment.
    
### Step 2: Generate Connector Template
-   **Run Script**:
    ```bash
    sh scripts/add_connector.sh <connector-name-lowercase> <connector-base-url>
    ```
    (Replace placeholders with actual connector name and its base API URL).
-   **Verify Output Structure**:
    -   `crates/hyperswitch_connectors/src/connectors/<connector-name-lowercase>/transformers.rs`
    -   `crates/hyperswitch_connectors/src/connectors/<connector-name-lowercase>.rs` (main logic)
    -   `crates/hyperswitch_connectors/src/connectors/<connector-name-lowercase>/test.rs`
-   **Manual File Move (Test File)**:
    -   From: `crates/hyperswitch_connectors/src/connectors/<connector-name-lowercase>/test.rs`
    -   To: `crates/router/tests/connectors/<connector-name-lowercase>.rs`

## Phase B: Core Logic Implementation (Guided by Integration Assistant)

This phase follows the "Hyperswitch Connector Integration Assistant" flow.

### Step 3: Payment Method Selection
-   Identify supported payment methods (e.g., Cards, Wallets, Bank Transfers).
-   [<|> Critical] Start with one primary method (e.g., Cards).

### Step 4: Flow Selection
-   For the selected payment method (e.g., Cards), determine the primary flow to implement first (e.g., Authorization, Capture, Refund).
-   **Consult `flow_guide`**: Use the `flow_guide` (from `memory-bank/techContext.md` or `integrations.md` a similar reference) to understand Hyperswitch's standard flows ->

preprocessing_flow
tokenization_flow
authorize_flow
cancel_flow
capture_flow
psync_flow
access_token_flow
refund 
rsync

<|> give examples of how flows work

<|> [Ignore]
complete_authorize_flow
incremental_authorization_flow
post_session_tokens_flow
reject_flow
session_update_flow
setup_mandate_flow
update_metadata_flow
<|> [Ignore]


-   **Decision Criteria**:
    -   Analyze the connector's API request/response formats for the chosen payment method and flow.
    -   Compare with Hyperswitch's implementation requirements and the `flow_guide` to select the best-fit Hyperswitch flow.
    -   Document the reasoning for the chosen flow.

### Step 5: API Documentation Deep Dive
-   For the selected flow (e.g., Authorize for Cards):
    -   Identify the exact API documentation URL(s).
    -   List required API endpoints (primary and any secondary).

### Step 6: Amount Type Specification
-   For the selected flow and connector:
    -   Locate the amount field(s) in the connector's API documentation.
    -   Determine the amount format required by the connector (e.g., StringMinorUnit, StringMajorUnit, FloatMinorUnit, MinorUnit).
    -   Note: Hyperswitch internally uses `MinorUnit`. Conversion will be necessary if the connector differs. The `connector-template/transformers.rs` provides a `...RouterData<T>` wrapper for this.

### Step 7: Connector Body Analysis (Field Compilation & Body Generation)
1.  **Field Compilation**:
    -   Extract all required and optional fields from the connector's API documentation for the selected flow's request and response.
    -   Validate field completeness against Hyperswitch's needs for that flow (refer to `PaymentsAuthorizeData`, etc., in `crates/hyperswitch_domain_models/src/router_request_types.rs`).
2.  **Body Generation (Conceptual)**:
    -   For each API endpoint involved in the flow, document the expected JSON request and response bodies.
    -   Format: `"Field_name" : "type (as per docs)" : "example_value (from docs)" : "Optional/Mandatory"`
    -   Example:
        `{$API_NAME}_CONNECTOR_REQUEST_BODY_JSON:`
        ```json
        {
          "amount" : "integer" : "1000": "Mandatory",
          "currency" : "string" : "USD": "Mandatory",
          "card_number" : "string" : "4242...": "Mandatory"
        }
        ```
        `{$API_NAME}_CONNECTOR_RESPONSE_BODY_JSON:`
        ```json
        {
          "id" : "string" : "txn_123": "Mandatory",
          "status" : "string" : "succeeded": "Mandatory"
        }
        ```

## Phase C: Transformer Implementation (`transformers.rs`)

This phase focuses on coding the `.../<connector-name-lowercase>/transformers.rs` file.

### Step 8: Type Discovery & Struct Definition
-   Based on Step 7, define Rust structs for the connector's API requests and responses.
-   **Adhere to `TYPE_DISCOVERY` Rules** (from `memory-bank/techContext.md`):
    1.  **Use Hyperswitch Types**: `pii::Email`, `enums::CountryAlpha2`, `api_models::payments::Currency` (or appropriate currency type from `storage_enums` or `common_enums`), etc.
    2.  **Security Handling**: `cards::CardNumber`, `masking::Secret<String>` for sensitive data.
    3.  **Enum Creation**: Define enums for connector-specific statuses (e.g., payment status, refund status) and implement `From<ConnectorStatus> for HyperswitchStatus`.
    4.  **Optional Field Handling**: Use `#[serde(skip_serializing_if = "Option::is_none")]`.
    5.  **Field Renaming**: Use `#[serde(rename = "api_field_name")]`.
    6.  **Case Conventions**: Use `#[serde(rename_all = "camelCase")]` or `#[serde(rename_all = "snake_case")]` as per connector API.
    <|> remame all
-   Define the connector's authentication struct (e.g., `MyConnectorAuthType`) and implement `TryFrom<&ConnectorAuthType>`.
-   Define the connector's error response struct (e.g., `MyConnectorErrorResponse`).

### Step 9: Struct Generation (Code Implementation)
-   Implement the structs defined in Step 8, deriving `serde::Serialize` and/or `serde::Deserialize`.
-   Example (Card Payment Request):
    ```rust
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")] // Or connector's convention
    pub struct MyConnectorPaymentRequest {
        pub amount: StringMinorUnit, // Or connector's required amount type
        pub email: Option<pii::Email>, // Use Hyperswitch types
        pub card: MyConnectorCardDetails, // Nested struct
        // ... other fields
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct MyConnectorCardDetails {
        #[serde(rename = "cardNumber")]
        pub number: cards::CardNumber,
        // ... other card fields
    }
    ```

### Step 10: Implement `TryFrom` Traits for Data Conversion
-   In `transformers.rs`, implement:
    -   `TryFrom<&{{project-name}}RouterData<&HyperswitchRequestData>> for {{project-name}}RequestStruct`
        (e.g., `TryFrom<&MyConnectorRouterData<&PaymentsAuthorizeData>> for MyConnectorPaymentRequest`)
    -   `TryFrom<ResponseRouterData<F, {{project-name}}ResponseStruct, T, HyperswitchResponseData>> for RouterData<F, T, HyperswitchResponseData>`
        (e.g., `TryFrom<ResponseRouterData<Authorize, MyConnectorPaymentResponse, PaymentsAuthorizeData, PaymentsResponseData>> for PaymentsAuthorizeRouterData`)

        <|> add example for each type
        give ref. domainmodels/types

-   Handle amount conversions within these traits or via the `{{project-name}}RouterData` wrapper (using `utils::convert_amount` in the main logic file is also common).
-   Refer to `connector-template/transformers.rs` and existing connectors (e.g., `hipay/transformers.rs`, adyen/transformers) for patterns.

1. payment status psync status mapping (cybersource-> diff, hipay -> same)
2. refunds status 
3. Construct auth type in transformers.rs


## Phase D: Main Logic Implementation (`<connector_name>.rs`)

This phase focuses on coding the `.../<connector-name-lowercase>.rs` file.

### Step 11: Implement Core Traits
1.  **Define Connector Struct**: `pub struct MyConnector;` (The template uses `{{project-name | downcase | pascal_case}}`).
2.  **Implement Marker Traits**: `api::Payment`, `api::PaymentAuthorize`, etc. (The template provides these).
3.  **Implement `ConnectorCommonExt`**:
    -   `build_headers()`: Typically combines Content-Type and auth headers. (Template provides a good base).
4.  **Implement `ConnectorCommon`**:
    -   `id()`: Return snake_case name.
    -   `get_currency_unit()`: Return `api::CurrencyUnit::Minor` or `api::CurrencyUnit::Base`. (Fill `todo!()` from template).
    -   `common_get_content_type()`: Usually `"application/json"`.
    -   `base_url()`: Fetch from `connectors.{{project-name}}.base_url`.
    -   `get_auth_header()`: Construct auth headers using the auth type from `transformers.rs`.
    -   `build_error_response()`: Parse connector error response (from `transformers.rs`) into Hyperswitch `ErrorResponse`.
5.  **Implement `ConnectorIntegration<Flow, RequestData, ResponseData>` for each required flow**:
    -   `get_headers()`: Usually `self.build_headers(...)`.
    -   `get_content_type()`: Usually `self.common_get_content_type()`.
    -   `get_url()`: Construct full API endpoint URL. (Fill `NotImplemented` from template).
    -   `get_request_body()`: Use `TryFrom` impl from `transformers.rs` to build request body. (Fill `NotImplemented` if template is basic).
    -   `build_request()`: Assemble `services::Request`.
    -   `handle_response()`: Parse `types::Response` using types from `transformers.rs` and convert to `RouterData`.
    -   `get_error_response()`: Usually `self.build_error_response(...)`.
6.  **Implement `ConnectorSpecifications`**:
    -   `get_connector_about()`: Return `ConnectorInfo`.
    -   `get_supported_payment_methods()`: Return `SupportedPaymentMethods`.
    -   `get_supported_webhook_flows()`: Return slice of `common_enums::EventClass`.
7. <|> remove **Implement `webhooks::IncomingWebhook`** (if applicable).

## Phase E: Registration, Configuration & Testing

### Step 12: Update Core Enums
-   **File**: `crates/common_enums/src/connector_enums.rs`
-   Add new connector (PascalCase) to:
    -   `Connector` enum.
    -   `RoutableConnectors` enum (if applicable).
-   Update `From<RoutableConnectors> for Connector` and `TryFrom<Connector> for RoutableConnectors` implementations.

### Step 13: Configuration
1.  **Backend Configuration**:
    -   **File**: `crates/connector_configs/toml/development.toml` (and/or other environments).
    -   Add section:
        ```toml
        [<connector-name-lowercase>]
        base_url = "https://api.connector.com"
        # secondary_base_url = "..." # If needed

        [<connector-name-lowercase>.connector_auth.HeaderKey] # Or .BodyKey, etc.
        api_key = "your_sandbox_api_key"
        ```
2.  **Control Center (UI) Configuration** (in `hyperswitch-control-center` repo):
    -   `src/screens/HyperSwitch/Connectors/ConnectorTypes.res`: Add to `connectorName` enum.
    -   `src/screens/HyperSwitch/Connectors/ConnectorUtils.res`: Update `connectorList` and related functions.
    -   `public/hyperswitch/Gateway/`: Add SVG icon (UPPERCASE name).
    -   Rebuild Wasm: `wasm-pack build ...` (see `memory-bank/techContext.md` for full command).

### Step 14: Implement and Run Tests
1.  **Adapt Boilerplate Test File**:
    -   **Location**: `crates/router/tests/connectors/<connector-name-lowercase>.rs`.
    -   Update test struct and `get_data()` to use your connector.
    -   Update `get_auth_token()`: Add credentials to `crates/router/tests/connectors/sample_auth.toml` (use env vars for real secrets).
    -   Implement `get_default_payment_info()` and `payment_method_details()`.
2.  **Run Existing Tests**: Ensure boilerplate tests pass.
3.  **Add Specific Tests**: For connector-specific functionalities.
4.  **Run Integration Tests**:
    ```bash
    export CONNECTOR_AUTH_FILE_PATH="/path/to/hyperswitch/crates/router/tests/connectors/sample_auth.toml"
    cargo test --package router --test connectors -- <connector-name-lowercase> --test-threads=1
    ```

## Phase F: Optional Steps

### Step 15: Generate Types from OpenAPI/JSON Schema (Optional)
-   If connector provides a schema:
    -   Install `openapi-generator`.
    -   Run generation script (see `memory-bank/techContext.md` for command).
    -   Refine generated types in `temp_generated_types.rs` and integrate into `transformers.rs`.

---
This guide should be treated as a living document and updated as the core Hyperswitch integration patterns evolve.
