# Progress

## Implementation Status
- **Dlocal Connector (2025-05-21 Session 10 - Post Refactoring):**
    - Refactored `crates/hyperswitch_connectors/src/connectors/dlocal.rs` to align with `real-codebase`.
    - Centralized header generation (including HMAC) in `ConnectorCommonExt::build_headers`.
    - Removed `generate_dlocal_hmac_signature` helper.
    - Simplified flow-specific `build_request` methods.
    - Added `ConnectorSpecifications` implementation with static data.
    - Updated imports and aligned various functions (e.g., `build_error_response`, PSync/RSync URLs) with `real-codebase` patterns.
    - Memory bank (`activeContext.md`, `progress.md`) and guides (`patterns.md`, `learnings.md`) updated to reflect these changes.
    - Removed unused `amount_converter` field from `Dlocal` struct.
    - Next step: Test the refactored code (`cargo build`). Build was successful with one warning (now resolved).

## Known Issues
- **Dlocal Connector (2025-05-21 Session 11 - Build Successful):**
    - Discrepancy noted and maintained: `real-codebase` HMAC for GET requests (`X-Login + X-Date + RequestBodyString` where RequestBodyString is empty) differs from Dlocal documentation (`X-Login + X-Date + PathAndQuery`). Current implementation follows `real-codebase`. This might need adjustment if Dlocal API rejects GET requests.
    - Unused import warnings and TODOs for card parsing/payer document in `transformers.rs` might still exist and need to be addressed in a future session if not critical for current functionality.
# Progress

## What Works (Airwallex Connector - As of 21/05/2025)

- **Initial Scaffolding**: `airwallex.rs` and `airwallex/transformers.rs` created.
- **Core Structs**: Basic request and response structs for Airwallex API interactions defined in `transformers.rs`.
- **Authentication Flow**: `AccessTokenAuth` flow implemented to obtain Bearer tokens. Logic for storing and using these tokens in subsequent requests has been refined.
- **Payment Flows (Initial Implementation)**:
    - PreProcessing (`payment_intents/create`)
    - Authorize (`payment_intents/{id}/confirm`)
    - PSync (`payment_intents/{id}`)
    - Capture (`payment_intents/{id}/capture`)
    - Void (Cancel) (`payment_intents/{id}/cancel`)
    - CompleteAuthorize (`payment_intents/{id}/confirm_continue`)
- **Refund Flows (Initial Implementation)**:
    - Execute (`refunds/create`)
    - RSync (`refunds/{id}`)
- **Webhook Handling (Basic)**: Initial structure for webhook object parsing and event type mapping.
- **Error Handling**: Basic error response mapping from Airwallex error structure.
- **Compilation Status**: Successfully compiles after addressing multiple errors.

## What's Left to Build (Airwallex Connector)

- **Address Warnings**: The last `cargo build` showed several unused import warnings in `airwallex.rs` and `airwallex/transformers.rs`. These should be cleaned up.
- **Thorough Testing**: Extensive testing using `cargo test --package router --test connectors -- airwallex --test-threads=1` with sandbox credentials.
- **Debugging**: Address runtime errors and logical flaws identified during testing.
- **Refinement based on `real-codebase`**: Continue to compare with `real-codebase/airwallex/` to ensure parity and correctness, especially for:
    - Detailed error mapping.
    - Complete 3DS handling nuances (the `ConnectorRedirectResponse` implementation is currently a placeholder).
    - All payment method variations supported by Airwallex.
    - Mandate and Session flows if required.
- **Documentation**: Update `guides/learnings/learning.md` with any new insights from testing. Ensure `guides/errors/errors.md` and `guides/patterns/patterns.md` capture any new generalizable learnings.

## Current Status (21/05/2025)

- **Airwallex Connector**:
    - All previously identified compilation errors have been resolved.
        - `transformers.rs`: Corrected amount conversion in `AirwallexPaymentsCaptureRequest` using `Option<String>` and `crate::utils::to_currency_base_unit`.
        - `airwallex.rs`:
            - Fixed `AccessTokenAuth` request body to send an empty JSON object.
            - Corrected `RefreshTokenRouterData` initialization.
            - Implemented `ConnectorRedirectResponse` trait with correct method signatures and types.
        - `router/src/types/api.rs`: Fixed `Airwallex` connector instantiation.
    - The code now compiles successfully.
    - Some unused import warnings remain, which `cargo fix` might have addressed (output capture pending).
- **Memory Bank**: `activeContext.md` and `progress.md` (this file) updated.
- **Guides**: `guides/learnings/learning.md` has been updated with insights from the latest debugging session.

## Known Issues (Resolved in this session)

- **`transformers.rs: E0308` for `StringMinorUnit::from(i64)`**: Resolved by changing `AirwallexPaymentsCaptureRequest.amount` to `Option<String>` and using `crate::utils::to_currency_base_unit`.
- **`airwallex.rs: E0599 RequestContent::Empty`**: Resolved by changing `get_request_body` for `AccessTokenAuth` to send `RequestContent::Json(Box::new(AirwallexAuthUpdateRequest {}))`.
- **`airwallex.rs: E0063 RefreshTokenRouterData missing fields`**: Resolved by using `..data.clone()` in the initializer.
- **`router/src/types/api.rs: E0423 expected value, found struct`**: Resolved by calling `connector::Airwallex::new()`.
- **`airwallex.rs: E0277 trait bound Airwallex: Connector not satisfied`**: Resolved by correctly implementing `ConnectorRedirectResponse` for `Airwallex` with the proper trait definition from `hyperswitch_interfaces::api`.

## Evolution of Project Decisions

- **Authentication Handling**: Refined the approach for Bearer token authentication to store the token in `RouterData.access_token` and use it directly in `build_headers`, rather than trying to modify `ConnectorAuthType`.
- **Connector ID Usage**: Shifted to using `RouterData.reference_id` for `payment_intent_id` in most payment flows post-PreProcessing, aligning with standard Hyperswitch patterns.
- **Trait Implementations**: Ensured necessary traits like `ConnectorRedirectResponse` are implemented with correct signatures by referring to their definitions in `hyperswitch_interfaces`.
