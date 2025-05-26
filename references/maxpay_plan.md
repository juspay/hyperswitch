# Maxpay Connector Implementation Plan

## Phase 1: Initial Setup and Boilerplate

- [x] Step 1: Generate connector boilerplate
  - **Task**: Run the add_connector.sh script to generate the initial boilerplate code for Maxpay connector
  - **Files**: N/A (script will generate files)
  - **Step Dependencies**: None
  - **User Instructions**: Run `./scripts/add_connector.sh maxpay` from the project root

- [x] Step 2: Move test file to correct location
  - **Task**: Move the generated test file from hyperswitch_connectors to router tests directory
  - **Files**:
    - Move from: `crates/hyperswitch_connectors/src/connectors/maxpay/test.rs`
    - Move to: `crates/router/tests/connectors/maxpay.rs`
  - **Step Dependencies**: Step 1
  - **User Instructions**: Ensure the file is moved after boilerplate generation

- [ ] Step 2.1: Build after initial setup
  - **Task**: Run cargo build to ensure the boilerplate code compiles correctly
  - **Files**: N/A
  - **Step Dependencies**: Step 2
  - **User Instructions**: Run `cargo build` from the project root

## Phase 2: Type Definitions and Data Models

- [x] Step 3: Define core enums and types
  - **Task**: Create the core enums (MaxpayTransactionType, MaxpayStatus) and authentication types in transformers.rs
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/maxpay/transformers.rs`: Define MaxpayTransactionType, MaxpayStatus, MaxpayAuth enums
  - **Step Dependencies**: Step 2
  - **User Instructions**: None

- [x] Step 4: Define authorization request/response types
  - **Task**: Create request and response types for AUTH and AUTH3D transactions following Maxpay API specification
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/maxpay/transformers.rs`: Add MaxpayAuthRequest, MaxpayAuthResponse structs with proper serde attributes
  - **Step Dependencies**: Step 3
  - **User Instructions**: None

- [x] Step 5: Define capture and sync types
  - **Task**: Create request/response types for SETTLE (capture) and CHECK (sync) operations
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/maxpay/transformers.rs`: Add MaxpayCaptureRequest, MaxpaySyncRequest, and response types
  - **Step Dependencies**: Step 3
  - **User Instructions**: None

- [x] Step 6: Define refund and tokenization types
  - **Task**: Create request/response types for refund operations and TOKENIZE transactions
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/maxpay/transformers.rs`: Add MaxpayRefundRequest, MaxpayRefundResponse, MaxpayTokenizeRequest, MaxpayTokenizeResponse
  - **Step Dependencies**: Step 3
  - **User Instructions**: None

- [x] Step 7: Define webhook types
  - **Task**: Create webhook data structures for both callback v1.0 (form-urlencoded) and v2.0 (JSON)
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/maxpay/transformers.rs`: Add MaxpayWebhookV1, MaxpayWebhookV2 structs
  - **Step Dependencies**: Step 3
  - **User Instructions**: None

- [x] Step 7.1: Build after type definitions
  - **Task**: Run cargo build to ensure all type definitions compile correctly
  - **Files**: N/A
  - **Step Dependencies**: Step 7
  - **User Instructions**: Run `cargo build` from the project root
  - **Status**: Fixed compilation errors related to StringMinorUnit access, card number conversion, and email field access

## Phase 3: Type Conversions and Transformers

- [x] Step 8: Implement authentication type conversions
  - **Task**: Implement TryFrom trait for converting ConnectorAuthType to MaxpayAuth
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/maxpay/transformers.rs`: Implement TryFrom<&ConnectorAuthType> for MaxpayAuth
  - **Step Dependencies**: Step 3
  - **User Instructions**: None
  - **Status**: Implemented using BodyKey pattern (api_key as merchant_account, key1 as merchant_password)

- [x] Step 9: Implement status mapping
  - **Task**: Create conversions from Maxpay status to Hyperswitch payment status enums
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/maxpay/transformers.rs`: Implement From<MaxpayStatus> for AttemptStatus and other status mappings
  - **Step Dependencies**: Step 3
  - **User Instructions**: None
  - **Status**: Implemented for both AttemptStatus and RefundStatus

- [x] Step 10: Implement authorization transformers
  - **Task**: Create TryFrom implementations for converting Hyperswitch PaymentsAuthorizeData to MaxpayAuthRequest
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/maxpay/transformers.rs`: Implement request transformation logic including amount conversion using common_utils
  - **Step Dependencies**: Steps 4, 8, 9
  - **User Instructions**: None
  - **Status**: Implemented with proper amount conversion from StringMinorUnit to f64 major units

- [x] Step 11: Implement response transformers
  - **Task**: Create TryFrom implementations for converting Maxpay responses to Hyperswitch response types
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/maxpay/transformers.rs`: Implement response transformation for auth, capture, sync, refund responses
  - **Step Dependencies**: Steps 4, 5, 6, 9
  - **User Instructions**: None
  - **Status**: Implemented for payments and refunds (Execute and RSync)

- [x] Step 11.1: Build after transformers
  - **Task**: Run cargo build to ensure all transformers compile correctly
  - **Files**: N/A
  - **Step Dependencies**: Step 11
  - **User Instructions**: Run `cargo build` from the project root
  - **Status**: Build successful with only minor warning about unused import

## Phase 4: Core Connector Implementation

- [x] Step 12: Implement ConnectorCommon trait
  - **Task**: Implement basic connector traits including id(), base_url(), get_currency_unit(), and auth header methods
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/maxpay.rs`: Implement ConnectorCommon trait with proper configuration
  - **Step Dependencies**: Steps 1-3
  - **User Instructions**: None
  - **Status**: Implemented with CurrencyUnit::Base and authentication in request body

- [x] Step 13: Implement authorization flow
  - **Task**: Implement ConnectorIntegration trait for Authorize including get_url, get_request_body, and handle_response methods
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/maxpay.rs`: Implement full authorization flow with 3DS detection
  - **Step Dependencies**: Steps 10, 11, 12
  - **User Instructions**: None
  - **Status**: Implemented authorization flow with proper request/response handling

- [x] Step 14: Implement capture flow
  - **Task**: Implement ConnectorIntegration trait for Capture operation (SETTLE transaction type)
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/maxpay.rs`: Implement capture flow using reference from authorization
  - **Step Dependencies**: Steps 5, 11, 12
  - **User Instructions**: None
  - **Status**: Implemented capture flow with SETTLE transaction type

- [x] Step 15: Implement payment sync flow
  - **Task**: Implement ConnectorIntegration trait for PSync operation (CHECK transaction type)
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/maxpay.rs`: Implement sync flow to check payment status
  - **Step Dependencies**: Steps 5, 11, 12
  - **User Instructions**: None
  - **Status**: Implemented sync flow with CHECK transaction type using POST method

- [x] Step 16: Implement refund flow
  - **Task**: Implement ConnectorIntegration trait for Refund operations
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/maxpay.rs`: Implement refund flow with proper endpoint (/api/refund)
  - **Step Dependencies**: Steps 6, 11, 12
  - **User Instructions**: None
  - **Status**: Implemented refund Execute and RSync flows with proper endpoint

- [x] Step 16.1: Build after core implementation
  - **Task**: Run cargo build to ensure all core connector implementations compile correctly
  - **Files**: N/A
  - **Step Dependencies**: Step 16
  - **User Instructions**: Run `cargo build` from the project root
  - **Status**: Build successful, all core flows implemented and compiling

## Phase 5: Advanced Features

- [x] Step 17: Implement 3D Secure handling
  - **Task**: Add support for AUTH3D and SALE3D transaction types with redirect flow handling
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/maxpay.rs`: Update authorization flow to handle 3DS redirects
    - `crates/hyperswitch_connectors/src/connectors/maxpay/transformers.rs`: Add 3DS-specific response handling
  - **Step Dependencies**: Step 13
  - **User Instructions**: None
  - **Status**: Implemented automatic 3DS detection based on callback_url and redirect_url presence, added custom status mapping for AuthenticationPending when redirect URL is present

- [x] Step 18: Implement tokenization support
  - **Task**: Add support for TOKENIZE transaction type for storing card tokens
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/maxpay.rs`: Implement tokenization flow
    - `crates/hyperswitch_connectors/src/connectors/maxpay/transformers.rs`: Add tokenization transformers
  - **Step Dependencies**: Steps 6, 12
  - **User Instructions**: None
  - **Status**: Implemented ConnectorIntegration trait for PaymentMethodToken with proper request/response transformers. TOKENIZE transaction type generates billToken for recurring payments.

- [x] Step 19: Implement webhook processing
  - **Task**: Implement IncomingWebhook trait for processing Maxpay callbacks (both v1.0 and v2.0)
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/maxpay.rs`: Implement webhook signature verification and processing
    - `crates/hyperswitch_connectors/src/connectors/maxpay/transformers.rs`: Add webhook parsing logic
  - **Step Dependencies**: Steps 7, 12
  - **User Instructions**: None
  - **Status**: Implemented webhook parsing for both v1.0 (form-urlencoded) and v2.0 (JSON) formats, added signature verification using SHA256 hash, and mapped webhook events to Hyperswitch webhook types

- [ ] Step 19.1: Build after advanced features
  - **Task**: Run cargo build to ensure all advanced features compile correctly
  - **Files**: N/A
  - **Step Dependencies**: Step 19
  - **User Instructions**: Run `cargo build` from the project root

## Phase 6: Error Handling and Edge Cases

- [ ] Step 20: Implement comprehensive error handling
  - **Task**: Create error code mapping and proper error responses for all Maxpay error codes
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/maxpay/transformers.rs`: Add MaxpayErrorCode enum and error mapping
    - `crates/hyperswitch_connectors/src/connectors/maxpay.rs`: Update all flows with proper error handling
  - **Step Dependencies**: Steps 13-16
  - **User Instructions**: None

- [ ] Step 21: Add test mode handling
  - **Task**: Implement special handling for test mode (e.g., adding "+" to phone numbers to avoid test declines)
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/maxpay/transformers.rs`: Add test mode detection and phone number modification
  - **Step Dependencies**: Step 10
  - **User Instructions**: None

- [ ] Step 21.1: Build after error handling
  - **Task**: Run cargo build to ensure all error handling code compiles correctly
  - **Files**: N/A
  - **Step Dependencies**: Step 21
  - **User Instructions**: Run `cargo build` from the project root

## Phase 7: Testing

- [ ] Step 22: Add unit tests for transformers
  - **Task**: Create comprehensive unit tests for all transformer functions and type conversions
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/maxpay/transformers.rs`: Add test module with unit tests
  - **Step Dependencies**: Steps 8-11
  - **User Instructions**: None

- [ ] Step 23: Add integration tests
  - **Task**: Create integration tests for all payment flows using test cards and credentials
  - **Files**:
    - `crates/router/tests/connectors/maxpay.rs`: Implement integration tests for auth, capture, sync, refund flows
  - **Step Dependencies**: Steps 13-16
  - **User Instructions**: None

- [ ] Step 24: Add webhook tests
  - **Task**: Create tests for webhook signature verification and processing
  - **Files**:
    - `crates/router/tests/connectors/maxpay.rs`: Add webhook processing tests for both v1.0 and v2.0
  - **Step Dependencies**: Step 19
  - **User Instructions**: None

- [ ] Step 24.1: Build and test after testing phase
  - **Task**: Run cargo build and cargo test to ensure all tests compile and pass
  - **Files**: N/A
  - **Step Dependencies**: Step 24
  - **User Instructions**: Run `cargo build` and `cargo test` from the project root

## Phase 8: Configuration and Documentation

- [ ] Step 25: Update connector configuration
  - **Task**: Add Maxpay configuration to development.toml and other config files
  - **Files**:
    - `config/development.toml`: Add Maxpay base URLs and supported payment methods
    - `config/config.example.toml`: Add example Maxpay configuration
  - **Step Dependencies**: None
  - **User Instructions**: None

- [ ] Step 26: Add connector to supported list
  - **Task**: Update connector lists to include Maxpay in the supported connectors
  - **Files**:
    - Update relevant connector registry files to include Maxpay
  - **Step Dependencies**: Step 25
  - **User Instructions**: Check which files need updating for connector registration

- [ ] Step 26.1: Final build and test
  - **Task**: Run final cargo build and cargo test to ensure the complete integration compiles and all tests pass
  - **Files**: N/A
  - **Step Dependencies**: Step 26
  - **User Instructions**: Run `cargo build` and `cargo test` from the project root

## Summary

This implementation plan breaks down the Maxpay connector integration into 33 manageable steps across 8 phases (including build verification steps):

1. **Initial Setup** (Steps 1-2): Generate boilerplate and organize files
2. **Type Definitions** (Steps 3-7): Define all request/response types
3. **Type Conversions** (Steps 8-11): Implement transformers between Hyperswitch and Maxpay types
4. **Core Implementation** (Steps 12-16): Implement basic payment flows
5. **Advanced Features** (Steps 17-19): Add 3DS, tokenization, and webhooks
6. **Error Handling** (Steps 20-21): Comprehensive error handling and test mode support
7. **Testing** (Steps 22-24): Unit and integration tests
8. **Configuration** (Steps 25-26): Update configuration files

Key considerations throughout implementation:
- Use existing types from hyperswitch_domain_models
- Leverage common_utils for amount conversion
- Follow existing connector patterns for consistency
- Ensure PCI compliance with proper data masking
- Support both test and production environments
- Handle both webhook versions (1.0 form-urlencoded and 2.0 JSON)
- Run cargo build after each phase without feature flags to ensure compilation

The implementation follows a logical progression where each step builds upon previous ones, ensuring a systematic and thorough integration of the Maxpay connector.
