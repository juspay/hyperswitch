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
    -   If the connector expects amounts in a different unit (e.g., base unit like dollars instead of minor unit like cents), handle the conversion. This might involve a wrapper struct like `MyconnectorRouterData<T>` that takes care of amount formatting.

#### 3.2. Main Logic (`.../<connector-name-lowercase>.rs`)

This file contains the core implementation of the connector's behavior by implementing various traits.

-   **Connector Struct**: Define a public struct for your connector (e.g., `pub struct MyConnectorPascalCase;`).
-   **Basic Trait Implementations**: Implement empty marker traits like `api::Payment`, `api::PaymentSession`, etc.
-   **`ConnectorCommonExt`**: Implement this trait, which usually involves a standard `build_headers` method that combines content type and authorization headers.
-   **`ConnectorCommon`**:
    -   `id()`: Return the snake_case name of the connector (e.g., `"myconnector"`).
    -   `get_currency_unit()`: Specify `api::CurrencyUnit::Minor` or `api::CurrencyUnit::Base`.
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
