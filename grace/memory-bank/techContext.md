# Tech Context

## Technologies Used

- [List primary technologies, languages, frameworks, etc.]

## Development Setup

- [Describe how to set up the development environment]

## Technical Constraints

- [List any technical constraints or limitations]

## Dependencies

- [List key external dependencies]

## Tool Usage Patterns

- [Describe patterns for using specific tools or utilities]

## Connector Integration Process

This section details the steps required to integrate a new payment connector into the Hyperswitch system.

### 1. Prerequisites

-   **Rust Nightly Toolchain**: Ensure the Rust nightly toolchain is installed. If not, install it using:
    ```bash
    rustup toolchain install nightly
    ```
-   **Connector API Knowledge**: Develop a thorough understanding of the API provided by the connector you intend to integrate.
-   **Sandbox Credentials**: Obtain API credentials for the connector's sandbox or testing environment.

### 2. Generate Template

-   **Run Script**: Use the provided shell script to generate the boilerplate code for the new connector:
    ```bash
    sh scripts/add_connector.sh <connector-name-lowercase> <connector-base-url>
    ```
    Replace `<connector-name-lowercase>` with the desired name for your connector (e.g., `myconnector`) and `<connector-base-url>` with its base API URL.
-   **Output Structure**: The script will create the following files and directories:
    -   `crates/hyperswitch_connectors/src/connectors/<connector-name-lowercase>/transformers.rs`: For request/response structs and data transformation logic.
    -   `crates/hyperswitch_connectors/src/connectors/<connector-name-lowercase>.rs`: Main logic file for the connector.
    -   `crates/hyperswitch_connectors/src/connectors/<connector-name-lowercase>/test.rs`: Boilerplate test file.
-   **Manual File Move**: After generation, manually move the test file:
    -   From: `crates/hyperswitch_connectors/src/connectors/<connector-name-lowercase>/test.rs`
    -   To: `crates/router/tests/connectors/<connector-name-lowercase>.rs`

### 3. Implement Connector Logic

#### 3.1. Transformers (`.../<connector-name-lowercase>/transformers.rs`)

This file is responsible for defining the data structures specific to the connector and the logic for converting data between Hyperswitch's generic types and these specific types.

-   **Request/Response Structs**:
    -   Define Rust structs for the connector's API requests (e.g., `MyconnectorPaymentsRequest`, `MyconnectorCard`). These should derive `serde::Serialize`.
    -   Define Rust structs for the connector's API responses (e.g., `MyconnectorPaymentsResponse`). These should derive `serde::Deserialize` and `serde::Serialize`.
-   **Authentication Struct**:
    -   Define a struct to handle authentication details (e.g., `MyconnectorAuthType`).
    -   Implement `TryFrom<&ConnectorAuthType>` for this struct to parse Hyperswitch's generic auth type.
-   **Payment Status Enum**:
    -   Define an enum for the connector's specific payment statuses (e.g., `MyconnectorPaymentStatus`).
    -   Implement `From<MyconnectorPaymentStatus> for common_enums::AttemptStatus` to map these to Hyperswitch's standard attempt statuses.
-   **Error Response Struct**:
    -   Define a struct for the connector's error responses (e.g., `MyconnectorErrorResponse`).
-   **Data Conversion (`TryFrom` traits)**:
    -   Implement `TryFrom` to convert `RouterData` (Hyperswitch's internal representation) into your connector-specific request structs.
    -   Implement `TryFrom` to convert connector-specific response structs back into `RouterData`.
-   **Amount Conversion**:
    -   If the connector expects amounts in a different unit (e.g., base unit like dollars instead of minor unit like cents), handle the conversion. The `connector-template/transformers.rs` provides a `{{project-name | downcase | pascal_case}}RouterData<T>` wrapper struct as a pattern for this, which includes an `amount` field (typically `StringMinorUnit`) and the generic `router_data`. The `utils::convert_amount` function is often used in the main logic file for this conversion.

#### 3.2. Main Logic (`.../<connector-name-lowercase>.rs`)

This file contains the core implementation of the connector's behavior by implementing various traits.

-   **Connector Struct**: Define a public struct for your connector (e.g., `pub struct MyConnectorPascalCase;`).
-   **Basic Trait Implementations**: Implement empty marker traits like `api::Payment`, `api::PaymentSession`, etc.
-   **`ConnectorCommonExt`**: Implement this trait, which usually involves a standard `build_headers` method that combines content type and authorization headers.
-   **`ConnectorCommon`**:
    -   `id()`: Return the snake_case name of the connector (e.g., `"myconnector"`).
    -   `get_currency_unit()`: Specify `api::CurrencyUnit::Minor` (e.g., cents) or `api::CurrencyUnit::Base` (e.g., dollars). The `connector-template/mod.rs` includes a `todo!()` for this, prompting the developer to check the connector's documentation.
    -   `common_get_content_type()`: Typically `"application/json"`.
    -   `base_url()`: Retrieve the base URL from the configuration (`connectors.myconnector.base_url`).
    -   `get_auth_header()`: Implement logic to construct the necessary authentication headers using the auth type defined in `transformers.rs`.
    -   `build_error_response()`: Parse the connector-specific error response (from `transformers.rs`) and map it to Hyperswitch's generic `ErrorResponse`.
-   **`ConnectorIntegration<Flow, RequestData, ResponseData>`**: Implement this for each payment flow (Authorize, PSync, Capture, Void) and refund flow (Execute, RSync), and other operations like PaymentMethodToken. For each flow:
    -   `get_headers()`: Usually delegates to `self.build_headers()`.
    -   `get_content_type()`: Usually delegates to `self.common_get_content_type()`.
    -   `get_url()`: Construct the full API endpoint URL for the specific flow.
    -   `get_request_body()`: Use the types and `TryFrom` impls from `transformers.rs` to build the request body.
    -   `build_request()`: Assemble the `services::Request` object (method, URL, headers, body).
    -   `handle_response()`: Parse the `types::Response` using types from `transformers.rs` and convert it back to `RouterData`.
    -   `get_error_response()`: Typically delegates to `self.build_error_response()`.
-   **`IncomingWebhook`**: If the connector supports webhooks, implement the methods to handle incoming webhook notifications.
-   **`ConnectorSpecifications`**:
    -   `get_connector_about()`: Return a static `ConnectorInfo` struct with display name and description.
    -   `get_supported_payment_methods()`: Return a static `SupportedPaymentMethods` detailing supported methods, features (refunds, mandates), capture methods, and card networks.
    -   `get_supported_webhook_flows()`: Return a static slice of supported `common_enums::EventClass`.
    -   **Template Usage**: The `connector-template/mod.rs` provides extensive boilerplate for these trait implementations, with many methods pre-filled with common patterns or marked with `Err(errors::ConnectorError::NotImplemented(...).into())` or `todo!()` where connector-specific logic is required.

#### 3.3. Guided Connector Implementation (Using the Integration Assistant)

The Hyperswitch Connector Integration Assistant provides a structured approach to implementing the core logic of a new connector. It guides you through selecting payment methods, flows, understanding API documentation, specifying amount types, and generating request/response bodies and structures.

**Step 2: Payment Method Selection**
1. Cards
   *(Further payment methods can be added based on connector capabilities)*

**Step 3: Flow Selection**
For the [SELECTED_METHOD] (e.g., Cards), determine which flow to implement first:
1. Authorization
2. Capture
3. Refunds
   *(The assistant will help decide based on integration requirements and API capabilities, as detailed in the `flow_guide` below).*

**Step 4: API Documentation**
For the [SELECTED_FLOW]:
*   Please share the exact API documentation URL or Upload Document PDF.
*   Identify Required endpoints:
    *   Primary:
    *   Secondary (if any):

**Step 5: Amount Type Specification**
For [SELECTED_FLOW] in [CONNECTOR_NAME]:
*   Locate the amount field in the API documentation and show it to the end User.
*   What amount format does the API require? Provide your suggestion.
    1.  StringMinorUnit
    2.  StringMajorUnit
    3.  FloatMinorUnit
    4.  MinorUnit (Hyperswitch's default internal representation)

**Step 6: Create the Connector BODY**
📋 Step-by-Step Instructions

1.  **FIELD_COMPILATION**
    *   `extract_all_required_fields_from_docs()`
    *   `validate_field_completeness()`

2.  **BODY_GENERATION**
    *   `generate_request_body()`
    *   `generate_response_body()`

    Format each output as: "Field_name : type(as mentioned in docs) : example"
    (Include nested fields with the same structure)

    For ALL the APIs given to you, do this (Replace API_NAME with appropriate name):

    `{$API_NAME}_CONNECTOR_REQUEST_BODY_JSON:`
    ```json
    {
      "field_name_1" : "type" : "all_example_values_given_in_doc/url": "Optional or Mandatory",
      "field_name_2" : "type" : "all_example_values_given_in_doc/url": "Optional or Mandatory"
    }
    ```

    `{$API_NAME}_CONNECTOR_RESPONSE_BODY_JSON:`
    ```json
    {
      "field_name_1" : "type" : "all_example_values_given_in_doc/url": "Optional or Mandatory",
      "field_name_2" : "type" : "all_example_values_given_in_doc/url": "Optional or Mandatory"
    }
    ```

**PHASE 1: TYPE_DISCOVERY**

RULES:

1.  **Use Hyperswitch Types where appropriate.** For example:
    ```rust
    // Instead of:
    pub struct PaymentRequest {
        country: Option<String>,
        email: String,
        currency: String,
        postal_code: Option<String>,
    }

    // Use:
    pub struct PaymentRequest {
        country: Option<enums::CountryAlpha2>,
        email: pii::Email,
        currency: api_models::Currency, // Assuming api_models::payments::Currency or similar
        postal_code: Option<Secret<String>>,
    }
    ```

2.  **Security Handling:**
    ```rust
    // Instead of:
    pub struct CustomerDetails {
        card_number: String,
        cvv: String,
    }

    // Use:
    pub struct CustomerDetails {
        card_number: cards::CardNumber,
        cvv: Secret<String>,
    }
    ```

3.  **Enum Creation:**
    ```rust
    // Instead of:
    pub struct PaymentMethod {
        payment_type: String, // Could be "card", "wallet", "bank_transfer"
    }

    // Use:
    #[derive(Debug, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub enum PaymentType {
        Card,
        Wallet,
        BankTransfer,
    }

    pub struct PaymentMethod {
        payment_type: PaymentType,
    }
    ```

4.  **Optional Field Handling:**
    ```rust
    // Instead of:
    pub struct ShippingDetails {
        address_line1: Option<String>,
        address_line2: Option<String>,
    }

    // Use:
    pub struct ShippingDetails {
        #[serde(skip_serializing_if = "Option::is_none")]
        address_line1: Option<String>,
        
        #[serde(skip_serializing_if = "Option::is_none")]
        address_line2: Option<String>,
    }
    ```

5.  **Field Renaming:**
    ```rust
    pub struct PaymentInfo {
        #[serde(rename = "payment_id")]
        pub id: String,
        
        #[serde(rename = "payment_status")]
        pub status: PaymentStatus, // Assuming PaymentStatus is a defined enum
    }
    ```

6.  **Case Conventions:**
    ```rust
    // For an API expecting camelCase:
    #[derive(Debug, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct RefundRequest {
        refund_amount: Amount, // Will be serialized as "refundAmount"
        refund_reason: String, // Will be serialized as "refundReason"
    }

    // For an API expecting snake_case:
    #[derive(Debug, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub struct PaymentResponse {
        PaymentId: String, // Will be serialized as "payment_id"
        PaymentStatus: String, // Will be serialized as "payment_status"
    }
    ```

---

**PHASE 2: STRUCT_GENERATION**

🛠️ For all APIs:
*   Generate `PROPOSED_HS_REQUEST_STRUCT`
*   Generate `PROPOSED_HS_RESPONSE_STRUCT`

💡 Example:
    ```rust
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CardPaymentRequest {
        pub amount: MinorUnit, // Example: {amount type decided in Step 5}
        pub email: pii::Email,
        pub card: CardDetails, // Assuming CardDetails is a defined struct

        #[serde(skip_serializing_if = "Option::is_none")]
        pub country: Option<enums::CountryAlpha2>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub postal_code: Option<Secret<String>>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub first_name: Option<Secret<String>>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub last_name: Option<Secret<String>>,

        #[serde(skip_serializing_if = "Option::is_none")]
        pub address1: Option<Secret<String>>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CardPaymentResponse {
        pub payment_id: String,
        pub status: PaymentStatus, // Assuming PaymentStatus is a defined enum
        pub amount: MinorUnit, // Example
        pub created_at: DateTime<Utc>, // Assuming use of chrono::DateTime

        #[serde(skip_serializing_if = "Option::is_none")]
        pub customer_id: Option<String>,
    }
    ```

---

**Flow Guide (`flow_guide`)**

This section helps in determining the appropriate Hyperswitch flow to implement based on the connector's API capabilities, particularly for the "Cards" payment method.

```yaml
flow_guide:
  payment_method: "Cards"
  note: "This flow logic applies only to the Cards payment method."
  ai_agent_instruction: |
    You will help determine which flow to implement by asking the user and suggesting the best fit based on the integration requirements and API capabilities.

  decision_criteria:
    - Understand the API request and response formats from provided docs/urls
    - Compare those with Hyperswitch’s implementation requirements.

  flows:
    - name: "DirectAuthorization"
      description: "For direct authorization with optional 3DS handling."
      required_traits:
        - impl: "ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData>"
        - condition: "requires_3ds"
        - impl_if_true: "ConnectorIntegration<CompleteAuthorize, CompleteAuthorizeData, PaymentsResponseData>"
      steps:
        - START: "Authorization"
        - IF: "requires3DS"
          THEN:
            - "Authorization -> CompleteAuth"
            - "CompleteAuth -> END"
        - ELSE:
            - "Authorization -> END"
      api_calls:
        count: 1 or 2
        calls:
          - name: "Authorization"
            purpose: "Initiate payment authorization"
          - name: "CompleteAuth"
            purpose: "Complete 3DS flow (if required)"

    - name: "PreprocessingBasedAuthorization"
      description: "Used when preprocessing is required before authorization."
      required_traits:
        - impl: "ConnectorIntegration<PreProcessing, PaymentsPreProcessingData, PaymentsResponseData>"
        - impl: "ConnectorIntegration<CompleteAuthorize, CompleteAuthorizeData, PaymentsResponseData>"
      steps:
        - START: "Preprocessing"
        - "Preprocessing -> ConnectorAPI"
        - "ConnectorAPI -> CompleteAuth"
        - "CompleteAuth -> END"
      api_calls:
        count: 3
        calls:
          - name: "PreProcessing"
            purpose: "Execute any pre-checks or token generation"
          - name: "ConnectorAPI"
            purpose: "Handle actual authorization logic"
          - name: "CompleteAuth"
            purpose: "Complete post-auth steps if needed"

    - name: "TokenizationBasedAuthorization"
      description: "Used when card tokenization is required before authorization."
      required_traits:
        - impl: "ConnectorIntegration<PaymentMethodToken, PaymentMethodTokenizationData, PaymentsResponseData>"
        - impl: "ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData>"
        - condition: "requires_3ds"
        - impl_if_true: "ConnectorIntegration<CompleteAuthorize, CompleteAuthorizeData, PaymentsResponseData>"
      steps:
        - START: "Tokenize"
        - "Tokenize -> Authorization"
        - IF: "requires3DS"
          THEN:
            - "Authorization -> CompleteAuth"
            - "CompleteAuth -> END"
        - ELSE:
            - "Authorization -> END"
      api_calls:
        count: 2 or 3
        calls:
          - name: "Tokenize"
            purpose: "Convert card details into token"
          - name: "Authorization"
            purpose: "Initiate payment"
          - name: "CompleteAuth"
            purpose: "Complete 3DS if required"

    - name: "AccessTokenBasedAuthorization"
      description: "Used when you must first get an access token from the connector."
      required_traits:
        - impl: "ConnectorIntegration<AccessToken, AccessTokenRequestData, PaymentsResponseData>"
        - impl: "ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData>"
        - condition: "requires_3ds"
        - impl_if_true: "ConnectorIntegration<CompleteAuthorize, CompleteAuthorizeData, PaymentsResponseData>"
      steps:
        - START: "GetAccessToken"
        - "GetAccessToken -> Authorization"
        - IF: "requires3DS"
          THEN:
            - "Authorization -> CompleteAuth"
            - "CompleteAuth -> END"
        - ELSE:
            - "Authorization -> END"
      api_calls:
        count: 2 or 3
        calls:
          - name: "GetAccessToken"
            purpose: "Retrieve auth token from connector"
          - name: "Authorization"
            purpose: "Initiate payment"
          - name: "CompleteAuth"
            purpose: "Complete 3DS flow if required"
```

Once you've reviewed and compared both the connector's API capabilities and Hyperswitch's architecture using the `flow_guide`, present the options to the end user for a decision:
`[OPTION] Implement Flow 1 --> [OPTION] Implement Flow 2 --> [OPTION] Provide Valid Reasoning`

Ensure the reasoning behind the selected flow is documented and aligns with API capabilities and Hyperswitch architecture.

---

**Hyperswitch Context (`[HYPERSWITCH_CONTEXT]`)**

This section provides context on Hyperswitch's internal request and response types, crucial for implementing the `TryFrom` traits in your connector's `transformers.rs`.

◆ **For Requests:**
Refer to `crates/hyperswitch_domain_models/src/router_request_types.rs` for available Hyperswitch request structs. You will typically implement `TryFrom` for one of these to convert it into your connector-specific request struct.

Key Request Structs (examples from the provided context):
*   `PaymentsAuthorizeData`: For payment authorization.
*   `PaymentsCaptureData`: For capturing authorized payments.
*   `PaymentsIncrementalAuthorizationData`: For incremental authorizations.
*   `PaymentMethodTokenizationData`: For tokenizing payment methods.
*   `PaymentsPreProcessingData`: For pre-processing steps.
*   `CompleteAuthorizeData`: For completing 3DS or other multi-step authorizations.
*   `PaymentsSyncData`: For synchronizing payment status.
*   `PaymentsCancelData`: For voiding/cancelling payments.
*   `RefundsData`: For processing refunds.
*   `AccessTokenRequestData`: For obtaining access tokens.
*   `CustomerDetails`: For customer information.
*   `VerifyWebhookSourceRequestData`: For verifying webhook authenticity.
*   `MandateRevokeRequestData`: For revoking mandates.
*   `SetupMandateRequestData`: For setting up new mandates.

(The file `crates/hyperswitch_domain_models/src/router_request_types.rs` contains the full, up-to-date list and definitions.)

◆ **For Responses:**
Hyperswitch expects connector responses to be converted into one of its standard response enums or structs, primarily `PaymentsResponseData` or `RefundsResponseData`.

Key Response Enum (`PaymentsResponseData` - from `crates/hyperswitch_domain_models/src/router_response_types.rs`):
*   `TransactionResponse`: For typical transaction outcomes.
    *   `resource_id`: Connector's transaction ID.
    *   `redirection_data`: If redirection is needed (e.g., for 3DS).
    *   `mandate_reference`: If a mandate is created.
*   `MultipleCaptureResponse`: For scenarios involving multiple captures.
*   `SessionResponse`: For session-based flows.
*   `SessionTokenResponse`: For returning session tokens.
*   `TransactionUnresolvedResponse`: If the transaction state is ambiguous.
*   `TokenizationResponse`: For payment method tokenization responses.
*   `ConnectorCustomerResponse`: For responses related to customer creation/management on the connector side.
*   `ThreeDSEnrollmentResponse`: For 3DS enrollment checks.
*   `PreProcessingResponse`: For pre-processing step outcomes.
*   `IncrementalAuthorizationResponse`: For incremental authorization outcomes.
*   `PostProcessingResponse`: For post-processing step outcomes.
*   `SessionUpdateResponse`: For session update outcomes.

(The file `crates/hyperswitch_domain_models/src/router_response_types.rs` contains the full, up-to-date list and definitions for response types like `PaymentsResponseData`, `RefundsResponseData`, etc.)

---

**Step 11: Transformer Implementation**

This step involves the practical application of the type discovery and struct generation phases.
*   Identify required Hyperswitch request/response types from the API specifications and the `[HYPERSWITCH_CONTEXT]` above.
*   Implement the `TryFrom` traits in your connector's `transformers.rs` file (e.g., `crates/hyperswitch_connectors/src/connectors/<your-connector>/transformers.rs`).
    *   `YourConnectorRequestType::try_from(RouterData<Flow, HyperswitchRequestData, HyperswitchResponseData>)`
    *   `RouterData<Flow, HyperswitchRequestData, HyperswitchResponseData>::try_from(YourConnectorResponseType)`
*   Refer to existing `transformers.rs` files in other connectors (e.g., `crates/hyperswitch_connectors/src/connectors/stripebilling/transformers.rs`) for practical examples of how these transformations are implemented, including error handling and mapping various fields. The `connector-template/transformers.rs` also provides a solid boilerplate for these structs and `TryFrom` implementations, including placeholders and `TODO` comments.

### 4. Update Core Enums

-   **Location**: `crates/common_enums/src/connector_enums.rs`.
-   **`Connector` Enum**: Add a new variant for your connector in PascalCase (e.g., `MyConnectorPascalCase`).
-   **`RoutableConnectors` Enum**: If your connector is a payment processor that should be available for routing, add the same PascalCase variant here.
-   **`From` / `TryFrom` Implementations**: Update the `From<RoutableConnectors> for Connector` and `TryFrom<Connector> for RoutableConnectors` implementations to include your new connector.

### 5. Implement Tests

-   **Location**: `crates/router/tests/connectors/<connector-name-lowercase>.rs`.
-   **Adapt Boilerplate**:
    -   Modify the generated test struct (e.g., `MyConnectorPascalCaseTest`) and its `get_data()` method to use your new connector struct and the `Connector` enum variant you added.
    -   Update `get_auth_token()` to retrieve authentication details for your connector. This requires setting up credentials in `crates/router/tests/connectors/sample_auth.toml`. **Important: Do not commit actual secret keys to `sample_auth.toml` if it's tracked by git; use environment variables or a git-ignored local override for real secrets.**
    -   Implement `get_default_payment_info()` and `payment_method_details()` to provide appropriate test data.
-   **Run Existing Tests**: Ensure all boilerplate tests pass with your implementation.
-   **Add Specific Tests**: Add new tests for any connector-specific functionalities or flows not covered by the template.

### 6. Configuration

#### 6.1. Backend Configuration

-   **File**: `crates/connector_configs/toml/development.toml` (and/or other environment-specific TOML files like `production.toml`).
-   **Content**: Add a new section for your connector:
    ```toml
    [<connector-name-lowercase>]
    base_url = "https://api.connector.com"
    # secondary_base_url = "https://token.connector.com" # If needed
    # Add other connector-specific configs here

    [<connector-name-lowercase>.connector_auth.HeaderKey] # Or .BodyKey, .SignatureKey etc.
    api_key = "your_sandbox_api_key"
    # other_auth_field = "value"
    ```

#### 6.2. Control Center (UI) Configuration

These changes are made in the `hyperswitch-control-center` repository.

-   **`ConnectorTypes.res`**:
    -   Path: `src/screens/HyperSwitch/Connectors/ConnectorTypes.res`
    -   Action: Add your connector (PascalCase) as a new variant to the `connectorName` enum.
-   **`ConnectorUtils.res`**:
    -   Path: `src/screens/HyperSwitch/Connectors/ConnectorUtils.res`
    -   Action:
        -   Add your connector to the `connectorList` array.
        -   Update `getConnectorNameString`, `getConnectorNameTypeFromString`, `getConnectorInfo`, and `getDisplayNameForConnectors` functions to include cases for your new connector.
-   **Connector Icon**:
    -   Path: `public/hyperswitch/Gateway/`
    -   Action: Add an SVG icon for your connector, named in uppercase (e.g., `MYCONNECTORPASCALCASE.SVG`).
-   **Build Wasm**: After making UI changes, rebuild the WebAssembly module for the Control Center:
    ```bash
    # Adjust paths as necessary
    wasm-pack build --target web \
      --out-dir /path/to/hyperswitch-control-center/public/hyperswitch/wasm \
      --out-name euclid /path/to/hyperswitch/crates/euclid_wasm \
      -- --features dummy_connector
    ```

### 7. Run Integration Tests

-   **Set Auth Path**: Ensure the `CONNECTOR_AUTH_FILE_PATH` environment variable points to your `sample_auth.toml` file:
    ```bash
    export CONNECTOR_AUTH_FILE_PATH="/path/to/hyperswitch/crates/router/tests/connectors/sample_auth.toml"
    ```
-   **Execute Tests**: Run the tests specifically for your connector:
    ```bash
    cargo test --package router --test connectors -- <connector-name-lowercase> --test-threads=1
    ```
    (The `--test-threads=1` flag is often recommended for connector tests as they might interact with external services and have rate limits or state dependencies.)

### 8. Optional: Generate Types from JSON/OpenAPI Schema

If the connector provides a JSON Schema or OpenAPI specification for its API:

-   **Install Tool**: `brew install openapi-generator` (or equivalent for your OS).
-   **Generate Code**:
    ```bash
    export CONNECTOR_NAME="<connector-name-lowercase>"
    export SCHEMA_PATH="<path-to-json-or-yaml-schema-file>"
    openapi-generator generate -g rust -i ${SCHEMA_PATH} -o temp && \
    cat temp/src/models/* > crates/hyperswitch_connectors/src/connectors/${CONNECTOR_NAME}/temp_generated_types.rs && \
    rm -rf temp && \
    # Basic cleanup - may need manual refinement
    sed -i'' -r "s/^pub use.*//;s/^pub mod.*//;s/^\/.*//;s/^.\*.*//;s/crate::models:://g;" crates/hyperswitch_connectors/src/connectors/${CONNECTOR_NAME}/temp_generated_types.rs && \
    cargo +nightly fmt -- crates/hyperswitch_connectors/src/connectors/${CONNECTOR_NAME}/temp_generated_types.rs
    ```
-   **Refine**: The generated code in `temp_generated_types.rs` will likely need manual cleanup and adaptation to fit Hyperswitch's patterns (e.g., using `masking::Secret` for sensitive fields, deriving necessary traits). Integrate these types into your `transformers.rs`.
