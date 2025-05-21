You are an AI task planner responsible for breaking down the integration of a new Hyperswitch payment connector into manageable steps.

Your goal is to create a detailed, step-by-step plan that will guide the code generation process for integrating the connector, based on a provided technical specification for that connector.

First, carefully review the following inputs:

<connector_integration_request>
{{CONNECTOR_INTEGRATION_REQUEST}}
</connector_integration_request>

<hyperswitch_connector_development_rules>
{{HYPERSWITCH_CONNECTOR_DEVELOPMENT_RULES}}
</hyperswitch_connector_development_rules>

<connector_technical_specification>
{{CONNECTOR_TECHNICAL_SPECIFICATION}}
</connector_technical_specification>

<hyperswitch_connector_template_files>
{{HYPERSWITCH_CONNECTOR_TEMPLATE_FILES}}
</hyperswitch_connector_template_files>

After reviewing these inputs, your task is to create a comprehensive, detailed plan for implementing the new connector.

Before creating the final plan, analyze the inputs and plan your approach. Wrap your thought process in <brainstorming> tags. Consider the following:
- Understanding the specific connector's API: authentication, payment flows (Authorize, Capture, Sync, Refund, etc.), data models, error handling, and any unique features.
- Mapping the connector's API capabilities to Hyperswitch traits (ConnectorCommon, ConnectorIntegration, ConnectorSpecifications) and data structures (PaymentsAuthorizeData, PaymentsResponseData, RouterData, etc.).
- Identifying all necessary data transformations and how they will be implemented in `transformers.rs` (request/response structs, enums, TryFrom implementations).
- Planning the sequence of implementation: typically starting with authentication, then a primary payment flow (e.g., Authorize), followed by other flows and features.
- Defining test cases for each implemented flow and feature.
- Listing all configuration steps required (backend `development.toml`, `sample_auth.toml`, and potentially Control Center UI changes).

Break down the integration process into small, manageable steps that can be executed sequentially by a code generation AI. Each step should focus on a specific aspect of the connector integration.

When creating your plan, follow these guidelines, referencing the structure from the Hyperswitch Connector Integration Guide:

1.  **Phase A: Preparation & Setup**: Cover prerequisites and template generation.
2.  **Phase B: Core Logic Implementation (Guided by Integration Assistant)**: Detail steps for understanding the connector's API for a chosen flow (e.g., Authorize for Cards), including payment method selection, flow selection (consulting `flow_guide`), API documentation deep dive, amount type specification, and connector body analysis.
3.  **Phase C: Transformer Implementation (`transformers.rs`)**: Detail steps for type discovery, struct definition (request, response, auth, error), and `TryFrom` trait implementations for data conversion for the chosen flow.
4.  **Phase D: Main Logic Implementation (`<connector_name>.rs`)**: Detail steps for implementing `ConnectorCommon`, `ConnectorCommonExt`, and `ConnectorIntegration` for the chosen flow.
5.  **Phase E: Registration, Configuration & Testing**: Detail steps for updating core enums, backend configuration, and implementing/running initial tests for the chosen flow.
6.  **Iteration for Other Flows/Features**: Include sections for subsequently implementing other payment flows (Capture, PSync, Void, Refunds), payment methods, and features like webhooks, following a similar pattern of Phases B-E.
7.  **Phase F: Optional Steps**: Include steps like generating types from OpenAPI if applicable.
8.  **User Instructions**: Include any manual steps the user needs to perform (e.g., verifying sandbox credentials, specific API testing).

Present your plan using the following markdown-based format. Each step must be atomic and self-contained enough to be implemented in a single code generation iteration.

# Implementation Plan: Spreedly Integration

## Phase A: Preparation & Setup

- [ ] **Step 1.1: Verify Prerequisites**
  - **Task**: Ensure Rust nightly toolchain is installed, Spreedly API documentation is understood, and sandbox credentials are obtained.
  - **Files**: N/A
  - **Step Dependencies**: None
  - **User Instructions**:
    - Run `rustup toolchain install nightly` if not already installed.
    - Review the Spreedly API documentation at https://developer.spreedly.com.
    - Obtain sandbox API keys/secrets for Spreedly (Environment Key and Access Secret) and have them ready.

- [ ] **Step 1.2: Generate Connector Template**
  - **Task**: Run the `add_connector.sh` script to generate boilerplate files for the new connector.
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/spreedly/transformers.rs` (created)
    - `crates/hyperswitch_connectors/src/connectors/spreedly.rs` (created)
    - `crates/hyperswitch_connectors/src/connectors/spreedly/test.rs` (created)
  - **Step Dependencies**: Step 1.1
  - **User Instructions**:
    - Execute: `sh scripts/add_connector.sh spreedly https://core.spreedly.com/v1`
    - Manually move `crates/hyperswitch_connectors/src/connectors/spreedly/test.rs` to `crates/router/tests/connectors/spreedly.rs`.

## Phase B: Core Logic Implementation (Authorize Flow for Cards)

- [ ] **Step 2.1: Analyze API for Authorize Flow (Cards)**
  - **Task**: Based on Spreedly API docs and the `flow_guide`, determine the exact API endpoints, request/response fields, and map to a Hyperswitch Authorize flow. Document amount type.
  - **Files**: (Documentation/Notes - no direct code changes yet)
  - **Step Dependencies**: Step 1.2
  - **User Instructions**: Review API docs for card authorization, focusing on the `/v1/gateways/{gateway_token}/authorize.json` endpoint, required fields, authentication mechanism, and status codes.

## Phase C: Transformer Implementation (`transformers.rs` - Authorize Flow)

- [ ] **Step 3.1: Define Authentication Structures**
  - **Task**: Define the `SpreedlyAuthType` struct and implement `TryFrom<&ConnectorAuthType>` for it. Define the `SpreedlyErrorResponse` struct.
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/spreedly/transformers.rs`: Add auth struct, error response struct, and TryFrom impl.
  - **Step Dependencies**: Step 2.1

- [ ] **Step 3.2: Define Request & Response Structs (Authorize Flow)**
  - **Task**: Define `SpreedlyPaymentsRequest` and `SpreedlyPaymentsResponse` structs adhering to `TYPE_DISCOVERY` rules. Define connector-specific payment status enum and `From` impl to `common_enums::AttemptStatus`.
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/spreedly/transformers.rs`: Add request/response structs and status enum.
  - **Step Dependencies**: Step 3.1

- [ ] **Step 3.3: Implement `TryFrom` for Request (Authorize Flow)**
  - **Task**: Implement `TryFrom<&SpreedlyRouterData<&PaymentsAuthorizeData>> for SpreedlyPaymentsRequest`.
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/spreedly/transformers.rs`: Add `TryFrom` implementation for request.
  - **Step Dependencies**: Step 3.2

- [ ] **Step 3.4: Implement `TryFrom` for Response (Authorize Flow)**
  - **Task**: Implement `TryFrom<ResponseRouterData<Authorize, SpreedlyPaymentsResponse, PaymentsAuthorizeData, PaymentsResponseData>> for PaymentsAuthorizeRouterData`.
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/spreedly/transformers.rs`: Add `TryFrom` implementation for response.
  - **Step Dependencies**: Step 3.2

## Phase D: Main Logic Implementation (`spreedly.rs` - Authorize Flow)

- [ ] **Step 4.1: Implement `ConnectorCommon`**
  - **Task**: Define the `pub struct Spreedly;`. Implement `ConnectorCommon` trait methods: `id`, `get_currency_unit`, `common_get_content_type`, `base_url`, `get_auth_header`, `build_error_response`.
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/spreedly.rs`: Add struct and `ConnectorCommon` impl.
  - **Step Dependencies**: Step 3.1, Step 3.2

- [ ] **Step 4.2: Implement `ConnectorIntegration` for Authorize Flow**
  - **Task**: Implement `ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData>` for `Spreedly`. This includes `get_headers`, `get_content_type`, `get_url`, `get_request_body`, `build_request`, `handle_response`, `get_error_response`.
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/spreedly.rs`: Add `ConnectorIntegration` impl for Authorize.
  - **Step Dependencies**: Step 3.3, Step 3.4, Step 4.1

## Phase E: Registration, Configuration & Testing (Authorize Flow)

- [ ] **Step 5.1: Update Core Enums**
  - **Task**: Add `Spreedly` to `Connector` and `RoutableConnectors` enums. Update `From` / `TryFrom` impls.
  - **Files**:
    - `crates/common_enums/src/connector_enums.rs`: Modify enums and impls.
  - **Step Dependencies**: Step 4.1

- [ ] **Step 5.2: Backend Configuration**
  - **Task**: Add configuration for `spreedly` in `development.toml` including base URL and auth details. Add credentials to `sample_auth.toml`.
  - **Files**:
    - `crates/connector_configs/toml/development.toml`: Add connector config.
    - `crates/router/tests/connectors/sample_auth.toml`: Add sandbox credentials.
  - **Step Dependencies**: Step 1.1, Step 4.1
  - **User Instructions**: Ensure sandbox credentials in `sample_auth.toml` are correct and not committed if real secrets.

- [ ] **Step 5.3: Implement Basic Authorize Test**
  - **Task**: Adapt the boilerplate test file to create and run a basic authorize payment test for `Spreedly`. Update `get_data`, `get_auth_token`, `get_default_payment_info`.
  - **Files**:
    - `crates/router/tests/connectors/spreedly.rs`: Implement authorize test.
  - **Step Dependencies**: Step 5.1, Step 5.2
  - **User Instructions**: Run the test: `export CONNECTOR_AUTH_FILE_PATH="crates/router/tests/connectors/sample_auth.toml" && cargo test --package router --test connectors -- spreedly::test_authorize_success --test-threads=1`

## Phase F: Capture Flow Implementation

- [ ] **Step 6.1: Define Request & Response Structs (Capture Flow)**
  - **Task**: Define `SpreedlyCaptureRequest` and `SpreedlyCaptureResponse` structs adhering to `TYPE_DISCOVERY` rules.
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/spreedly/transformers.rs`: Add capture request/response structs.
  - **Step Dependencies**: Phase E completed

- [ ] **Step 6.2: Implement `TryFrom` for Capture Request and Response**
  - **Task**: Implement the necessary TryFrom traits for capture requests and responses.
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/spreedly/transformers.rs`: Add TryFrom implementations.
  - **Step Dependencies**: Step 6.1

- [ ] **Step 6.3: Implement `ConnectorIntegration` for Capture Flow**
  - **Task**: Implement `ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData>` for `Spreedly`.
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/spreedly.rs`: Add `ConnectorIntegration` impl for Capture.
  - **Step Dependencies**: Step 6.2

- [ ] **Step 6.4: Implement Capture Test**
  - **Task**: Add test for the capture flow.
  - **Files**:
    - `crates/router/tests/connectors/spreedly.rs`: Add capture test.
  - **Step Dependencies**: Step 6.3

## Phase G: Payment Sync Flow Implementation

- [ ] **Step 7.1: Define Response Structs (PSync Flow)**
  - **Task**: Define `SpreedlyPSyncResponse` struct adhering to `TYPE_DISCOVERY` rules.
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/spreedly/transformers.rs`: Add PSync response struct.
  - **Step Dependencies**: Phase F completed

- [ ] **Step 7.2: Implement `TryFrom` for PSync Response**
  - **Task**: Implement the necessary TryFrom trait for PSync responses.
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/spreedly/transformers.rs`: Add TryFrom implementation.
  - **Step Dependencies**: Step 7.1

- [ ] **Step 7.3: Implement `ConnectorIntegration` for PSync Flow**
  - **Task**: Implement `ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData>` for `Spreedly`.
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/spreedly.rs`: Add `ConnectorIntegration` impl for PSync.
  - **Step Dependencies**: Step 7.2

- [ ] **Step 7.4: Implement PSync Test**
  - **Task**: Add test for the payment sync flow.
  - **Files**:
    - `crates/router/tests/connectors/spreedly.rs`: Add PSync test.
  - **Step Dependencies**: Step 7.3

## Phase H: Refund Flow Implementation

- [ ] **Step 8.1: Define Request & Response Structs (Refund Flow)**
  - **Task**: Define `SpreedlyRefundRequest` and `SpreedlyRefundResponse` structs adhering to `TYPE_DISCOVERY` rules. Define connector-specific refund status enum.
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/spreedly/transformers.rs`: Add refund request/response structs and refund status enum.
  - **Step Dependencies**: Phase G completed

- [ ] **Step 8.2: Implement `TryFrom` for Refund Request and Response**
  - **Task**: Implement the necessary TryFrom traits for refund requests and responses.
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/spreedly/transformers.rs`: Add TryFrom implementations.
  - **Step Dependencies**: Step 8.1

- [ ] **Step 8.3: Implement `ConnectorIntegration` for Refund Flow**
  - **Task**: Implement `ConnectorIntegration<Execute, RefundsData, RefundsResponseData>` for `Spreedly`.
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/spreedly.rs`: Add `ConnectorIntegration` impl for Execute (Refund).
  - **Step Dependencies**: Step 8.2

- [ ] **Step 8.4: Implement Refund Sync Flow**
  - **Task**: Implement `ConnectorIntegration<RSync, RefundSyncData, RefundsResponseData>` for `Spreedly`.
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/spreedly.rs`: Add `ConnectorIntegration` impl for RSync.
  - **Step Dependencies**: Step 8.3

- [ ] **Step 8.5: Implement Refund Tests**
  - **Task**: Add tests for the refund and refund sync flows.
  - **Files**:
    - `crates/router/tests/connectors/spreedly.rs`: Add refund and refund sync tests.
  - **Step Dependencies**: Step 8.4

## Phase I: Connector Specifications & Final Testing

- [ ] **Step 9.1: Implement `ConnectorSpecifications` Trait**
  - **Task**: Implement the `ConnectorSpecifications` trait with appropriate payment method support.
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/spreedly.rs`: Add `ConnectorSpecifications` implementation.
  - **Step Dependencies**: All previous phases completed

- [ ] **Step 9.2: Comprehensive Testing**
  - **Task**: Run all tests to ensure the connector works as expected across all implemented flows.
  - **Files**: N/A
  - **Step Dependencies**: Step 9.1
  - **User Instructions**: Run the tests: `export CONNECTOR_AUTH_FILE_PATH="crates/router/tests/connectors/sample_auth.toml" && cargo test --package router --test connectors -- spreedly::` (this will run all spreedly tests)

## Summary of Implementation Approach

This implementation plan outlines the step-by-step process for integrating Spreedly with Hyperswitch, focusing on core payment flows: Authorize, Capture, Payment Sync, and Refund. The implementation follows the standard Hyperswitch connector pattern, ensuring proper separation of concerns between data transformation (`transformers.rs`) and integration logic (`spreedly.rs`).

Key considerations:
1. Authentication using HTTP Basic Auth with Environment Key and Access Secret
2. Amount handling in minor units (cents)
3. Mapping of Spreedly transaction statuses to Hyperswitch payment/refund statuses
4. Full support for card payments with the four essential payment flows
5. Comprehensive test coverage for all implemented flows

The implementation will proceed in phases, starting with basic authorization and then building additional flows incrementally, ensuring each component is thoroughly tested before proceeding to the next phase.
