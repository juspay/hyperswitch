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
