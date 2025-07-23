# Implementation Plan: Mpgs Connector

## Phase 1: Setup and Configuration

- [ ] Step 1: Move the test file
  - **Task**: Move the auto-generated test file to the correct location.
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/mpgs/test.rs` -> `crates/router/tests/connectors/mpgs.rs`
  - **Step Dependencies**: None

- [ ] Step 2: Add Mpgs to the Connectors enum
  - **Task**: Add `Mpgs` to the `Connectors` enum in `crates/common_enums/src/enums.rs`.
  - **Files**:
    - `crates/common_enums/src/enums.rs`
  - **Step Dependencies**: Step 1

- [ ] Step 3: Add Mpgs to the configuration files
  - **Task**: Add the Mpgs connector to the `config/development.toml` and `config/docker_compose.toml` files.
  - **Files**:
    - `config/development.toml`
    - `config/docker_compose.toml`
  - **Step Dependencies**: Step 2

## Phase 2: Transformer Implementation

- [ ] Step 4: Implement Mpgs Auth and RouterData structs
  - **Task**: Implement the `MpgsAuthType` and `MpgsRouterData` structs in `transformers.rs`.
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/mpgs/transformers.rs`
  - **Step Dependencies**: Step 3

- [ ] Step 5: Implement Mpgs Request and Response structs
  - **Task**: Implement the request and response structs for all supported flows (Authorize, Capture, Void, Refund, Sync) in `transformers.rs`.
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/mpgs/transformers.rs`
  - **Step Dependencies**: Step 4

- [ ] Step 6: Implement `TryFrom` for Mpgs PaymentsRequest
  - **Task**: Implement the `TryFrom` trait to convert from `MpgsRouterData<&PaymentsAuthorizeRouterData>` to `MpgsPaymentsRequest`.
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/mpgs/transformers.rs`
  - **Step Dependencies**: Step 5

- [ ] Step 7: Implement `TryFrom` for Mpgs PaymentsResponse
  - **Task**: Implement the `TryFrom` trait to convert from `ResponseRouterData<F, MpgsPaymentsResponse, T, PaymentsResponseData>` to `RouterData<F, T, PaymentsResponseData>`.
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/mpgs/transformers.rs`
  - **Step Dependencies**: Step 6

- [ ] Step 8: Implement `TryFrom` for Mpgs RefundRequest
  - **Task**: Implement the `TryFrom` trait to convert from `&RefundsRouterData<Execute>` to `MpgsRefundRequest`.
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/mpgs/transformers.rs`
  - **Step Dependencies**: Step 7

- [ ] Step 9: Implement `TryFrom` for Mpgs RefundResponse
  - **Task**: Implement the `TryFrom` trait to convert from `ResponseRouterData<Execute, MpgsRefundResponse, RefundsRouterData<Execute>, RefundsResponseData>` to `RefundsRouterData<Execute>`.
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/mpgs/transformers.rs`
  - **Step Dependencies**: Step 8

## Phase 3: Core Logic Implementation

- [ ] Step 10: Implement `ConnectorCommon` for Mpgs
  - **Task**: Implement the `ConnectorCommon` trait for the `Mpgs` struct in `mpgs.rs`.
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/mpgs.rs`
  - **Step Dependencies**: Step 9

- [ ] Step 11: Implement `ConnectorIntegration` for Authorize
  - **Task**: Implement the `ConnectorIntegration` trait for the `Authorize` flow.
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/mpgs.rs`
  - **Step Dependencies**: Step 10

- [ ] Step 12: Implement `ConnectorIntegration` for Capture
  - **Task**: Implement the `ConnectorIntegration` trait for the `Capture` flow.
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/mpgs.rs`
  - **Step Dependencies**: Step 11

- [ ] Step 13: Implement `ConnectorIntegration` for Void
  - **Task**: Implement the `ConnectorIntegration` trait for the `Void` flow.
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/mpgs.rs`
  - **Step Dependencies**: Step 12

- [ ] Step 14: Implement `ConnectorIntegration` for Sync
  - **Task**: Implement the `ConnectorIntegration` trait for the `Sync` flow.
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/mpgs.rs`
  - **Step Dependencies**: Step 13

- [ ] Step 15: Implement `ConnectorIntegration` for Refund
  - **Task**: Implement the `ConnectorIntegration` trait for the `Refund` flow.
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/mpgs.rs`
  - **Step Dependencies**: Step 14

## Phase 4: Testing

- [ ] Step 16: Implement integration tests
  - **Task**: Implement the integration tests for all supported flows in `crates/router/tests/connectors/mpgs.rs`.
  - **Files**:
    - `crates/router/tests/connectors/mpgs.rs`
  - **Step Dependencies**: Step 15
  - **User Instructions**: Add test credentials for Mpgs to `crates/router/tests/connectors/sample_auth.toml`.
