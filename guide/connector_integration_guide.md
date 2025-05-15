# Hyperswitch Connector Integration: Step-by-Step Guide

This guide provides a reusable, step-by-step process for accurately adding a new payment connector to the Hyperswitch system. It synthesizes information from the "Hyperswitch Connector Integration Assistant" and the general "Connector Integration Process".

## Phase A: Preparation & Setup

### Step 1: Prerequisites
-   **Rust Nightly Toolchain**: Ensure `rustup toolchain install nightly` is done.
-   **Connector API Knowledge**: Thoroughly understand the target connector's API documentation (endpoints, request/response formats, authentication, error codes).
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
-   Start with one primary method (e.g., Cards).

### Step 4: Flow Selection
-   For the selected payment method (e.g., Cards), determine the primary flow to implement first (e.g., Authorization, Capture, Refund).
-   **Consult `flow_guide`**: Use the `flow_guide` (from `memory-bank/techContext.md` or a similar reference) to understand Hyperswitch's standard flows (DirectAuthorization, PreprocessingBasedAuthorization, etc.).
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
-   Handle amount conversions within these traits or via the `{{project-name}}RouterData` wrapper (using `utils::convert_amount` in the main logic file is also common).
-   Refer to `connector-template/transformers.rs` and existing connectors (e.g., `stripebilling/transformers.rs`) for patterns.

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
7.  **Implement `webhooks::IncomingWebhook`** (if applicable).

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

## Common Pitfalls and Lessons Learned

Based on our experience with connector integrations (such as HiPay), here are key pitfalls to watch out for:

### 1. Authentication Mechanism Issues

- **Verify the auth type carefully**: Some connectors use `HeaderKey`, others use `BodyKey`, and some require complex multi-step authentication. Read the API docs thoroughly.
- **Check for Base64 encoding requirements**: Many Basic Auth implementations require Base64 encoding of credentials.
- **Use the correct credential handling**: For sensitive data, leverage `PeekInterface` rather than `expose()` when possible.

### 2. Amount Handling Variations

- **Major vs Minor Units**: Different payment processors expect amounts in different formats:
  - Major units (like 10.99 for dollars/euros) using `StringMajorUnit`
  - Minor units (like 1099 for cents) using `StringMinorUnit`
- Always check the API documentation carefully for amount format requirements.
- Verify by examining sample requests in the payment processor's documentation.

### 3. Request Format Requirements

- Some connectors expect `application/x-www-form-urlencoded` while others require `multipart/form-data`
- For FormData requests, you may need a custom serialization helper function
- Check content type requirements for each endpoint as they may vary within the same API

### 4. URL Construction Patterns

- Many connectors use different base URLs for different services (e.g., tokenization vs payment processing)
- Some connectors require path parameters, others use query parameters
- Ensure proper URL construction by testing each endpoint's format separately

### 5. Status Mapping Complexity

- Payment processors often use numeric or custom string codes for statuses
- Create comprehensive status mapping that covers all possible states
- Pay special attention to pending, partial success, and error states
- Document status mapping clearly for future reference

### 6. Structure Definition Completeness

- Ensure all required fields are included in request/response structures
- Handle optional fields appropriately with `Option<T>` and proper serde attributes
- Use `#[serde(rename = "field_name")]` when field names don't match Rust naming conventions
- Use `#[serde(skip_serializing_if = "Option::is_none")]` for optional fields

### 7. Testing All Flows

- Test the entire payment lifecycle: authorization, capture, refund, void
- Test both successful and error scenarios
- Verify 3DS flows if the connector supports them
- Test synchronization endpoints separately

By keeping these lessons in mind, you can avoid common pitfalls and accelerate connector integrations.

---
This guide should be treated as a living document and updated as the core Hyperswitch integration patterns evolve.
