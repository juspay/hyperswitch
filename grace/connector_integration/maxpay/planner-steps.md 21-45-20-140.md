# Implementation Plan: Maxpay Integration

## Phase A: Preparation & Setup

- [x] **Step 1.1: Verify Prerequisites**
  - **Task**: Ensure Rust nightly toolchain is installed, Maxpay API documentation is understood, and sandbox credentials are obtained.
  - **Files**: N/A
  - **Step Dependencies**: None
  - **User Instructions**:
    - Run `rustup toolchain install nightly` if not already installed.
    - Review the Maxpay API documentation at https://gateway.maxpay.com/api/payout
    - Obtain sandbox API credentials for Maxpay.

- [x] **Step 1.2: Generate Connector Template**
  - **Task**: Run the `add_connector.sh` script to generate boilerplate files for the new connector.
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/maxpay/transformers.rs` (created)
    - `crates/hyperswitch_connectors/src/connectors/maxpay.rs` (created)
    - `crates/hyperswitch_connectors/src/connectors/maxpay/test.rs` (created)
  - **Step Dependencies**: Step 1.1
  - **User Instructions**:
    - Execute: `sh scripts/add_connector.sh maxpay https://gateway-sandbox.maxpay.com`
    - Manually move `crates/hyperswitch_connectors/src/connectors/maxpay/test.rs` to `crates/router/tests/connectors/maxpay.rs`.

## Phase B: Core Logic Implementation (Payout Flow)

- [x] **Step 2.1: Analyze API for Payout Flow**
  - **Task**: Based on Maxpay API docs and the tech-spec.md, confirm the exact API endpoints, request/response fields, and mapping to a Hyperswitch Payout flow. Confirm amount type (float in major units).
  - **Files**: (Documentation/Notes - no direct code changes)
  - **Step Dependencies**: Step 1.2
  - **User Instructions**: Review tech-spec.md for Payout flow details.

## Phase C: Transformer Implementation (`transformers.rs` - Payout Flow)

- [x] **Step 3.1: Define Authentication Structures**
  - **Task**: Define the `MaxpayAuthType` struct and implement `TryFrom<&ConnectorAuthType>` for it, as specified in the tech-spec.md. Define the `MaxpayErrorResponse` struct.
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/maxpay/transformers.rs`: Add auth struct, error response struct, and TryFrom impl.
  - **Step Dependencies**: Step 2.1

- [x] **Step 3.2: Define Router Data Wrapper**
  - **Task**: Define the `MaxpayRouterData<T>` struct to wrap Hyperswitch router data with amount formatting.
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/maxpay/transformers.rs`: Add router data wrapper.
  - **Step Dependencies**: Step 3.1

- [x] **Step 3.3: Define Request & Response Structs (Payout Flow)**
  - **Task**: Define `MaxpayPayoutRequest` and `MaxpayPayoutResponse` structs, as well as supporting structures like `MaxpayCardDetails`, `MaxpayFullCard`, and `MaxpayTokenCard`. Define status enums and `From` impls.
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/maxpay/transformers.rs`: Add request/response structs and status enums.
  - **Step Dependencies**: Step 3.2

- [x] **Step 3.4: Implement `TryFrom` for Request (Payout Flow)**
  - **Task**: Implement `TryFrom<&MaxpayRouterData<&PayoutsData>> for MaxpayPayoutRequest` to transform Hyperswitch payout data to Maxpay request format.
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/maxpay/transformers.rs`: Add `TryFrom` implementation for request.
  - **Step Dependencies**: Step 3.3

- [x] **Step 3.5: Implement `TryFrom` for Response (Payout Flow)**
  - **Task**: Implement `TryFrom<ResponseRouterData<PoFulfill, MaxpayPayoutResponse, PayoutsData, PayoutsResponseData>> for PayoutsRouterData` to transform Maxpay response to Hyperswitch format.
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/maxpay/transformers.rs`: Add `TryFrom` implementation for response.
  - **Step Dependencies**: Step 3.3

## Phase D: Main Logic Implementation (`maxpay.rs` - Payout Flow)

- [x] **Step 4.1: Implement `ConnectorCommon`**
  - **Task**: Define the `pub struct Maxpay;`. Implement `ConnectorCommon` trait methods: `id`, `get_currency_unit`, `common_get_content_type`, `base_url`, `get_auth_header`, `build_error_response`.
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/maxpay.rs`: Add struct and `ConnectorCommon` impl.
  - **Step Dependencies**: Step 3.1, Step 3.3

- [x] **Step 4.2: Implement `ConnectorIntegration` for Payout Flow**
  - **Task**: Implement `ConnectorIntegration<PoFulfill, PayoutsData, PayoutsResponseData>` for `Maxpay`. This includes `get_headers`, `get_content_type`, `get_url`, `get_request_body`, `build_request`, `handle_response`, `get_error_response`.
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/maxpay.rs`: Add `ConnectorIntegration` impl for Payout.
  - **Step Dependencies**: Step 3.4, Step 3.5, Step 4.1

- [x] **Step 4.3: Implement `ConnectorSpecifications`**
  - **Task**: Implement `ConnectorSpecifications` trait methods: `get_connector_about`, `get_supported_payment_methods`, `get_supported_webhook_flows`.
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/maxpay.rs`: Add `ConnectorSpecifications` impl.
  - **Step Dependencies**: Step 4.1

## Phase E: Webhook Implementation

- [x] **Step 5.1: Define Webhook Structures**
  - **Task**: Define `MaxpayWebhookDetails` and `MaxpayWebhookStatus` structures for handling webhook data.
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/maxpay/transformers.rs`: Add webhook structs and enums.
  - **Step Dependencies**: Step 3.3

- [x] **Step 5.2: Implement `IncomingWebhook` Trait**
  - **Task**: Implement `webhooks::IncomingWebhook` trait for `Maxpay`, including methods for extracting object reference IDs, event types, and resource objects from webhook payloads.
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/maxpay.rs`: Add `IncomingWebhook` impl.
  - **Step Dependencies**: Step 5.1, Step 4.1

## Phase F: Registration, Configuration & Testing

- [x] **Step 6.1: Update Core Enums**
  - **Task**: Add `Maxpay` to `Connector` and `RoutableConnectors` enums. Update `From` / `TryFrom` impls.
  - **Files**:
    - `crates/common_enums/src/connector_enums.rs`: Modify enums and impls.
  - **Step Dependencies**: Step 4.1

- [x] **Step 6.2: Backend Configuration**
  - **Task**: Add configuration for `maxpay` in `development.toml` including base URL and auth details. Add test credentials to `sample_auth.toml`.
  - **Files**:
    - `crates/connector_configs/toml/development.toml`: Add connector config.
    - `crates/router/tests/connectors/sample_auth.toml`: Add sandbox credentials.
  - **Step Dependencies**: Step 4.1
  - **User Instructions**: Ensure sandbox credentials in `sample_auth.toml` are correct.

- [x] **Step 6.3: Implement Payout Test**
  - **Task**: Create and implement a test file for the Maxpay payout functionality. Include tests for card payout and token payout.
  - **Files**:
    - `crates/router/tests/connectors/maxpay.rs`: Implement payout tests.
  - **Step Dependencies**: Step 6.1, Step 6.2
  - **User Instructions**: Run the test with appropriate environment variables.

## Post-Implementation Tasks

- [ ] **Step 7.1: Run All Tests**
  - **Task**: Run all implemented tests to ensure everything is working correctly.
  - **Files**: N/A
  - **Step Dependencies**: All previous steps
  - **User Instructions**: 
    ```bash
    export CONNECTOR_AUTH_FILE_PATH="crates/router/tests/connectors/sample_auth.toml"
    cargo test --package router --test connectors -- maxpay --test-threads=1
    ```

- [ ] **Step 7.2: Update Documentation**
  - **Task**: Update any documentation related to the Maxpay connector implementation. Document any specific behaviors or edge cases.
  - **Files**: Relevant documentation files
  - **Step Dependencies**: All previous steps

## Implementation Notes and Key Considerations

1. **Authentication**: Maxpay uses body-based authentication, not header authentication. The credentials (`merchant_account` and `merchant_password`) are sent directly in the request body.

2. **Amount Handling**: Maxpay expects amounts as floats in major units (e.g., 10.00 for $10). A conversion is needed from Hyperswitch's `MinorUnit`.

3. **Webhook Flow**: Maxpay sends webhook callbacks to update the status of payouts. The callback URL is specified in the initial payout request.

4. **Card Data**: The implementation should support both full card details and token-based payouts.

5. **Error Handling**: Ensure proper error handling for various error scenarios, including invalid credentials, invalid card data, and general API errors.

6. **Testing**: Include tests for successful payouts, error cases, and webhook processing.
