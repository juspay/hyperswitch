# Active Context

## Current Connector Status
- **Connector:** Dlocal
- **Status:** Core payment flows implemented. `dlocal.rs` refactored to align with `real-codebase` for header generation and `ConnectorSpecifications`.
- **Current Phase:** Refactoring complete. Pending testing and further error resolution if any.

## Recent Decisions
- **HMAC Signature & Header Strategy Alignment (Implemented):**
    - Aligned with `real-codebase`'s strategy of centralizing all header generation (including dynamic auth and HMAC) in `ConnectorCommonExt::build_headers`.
    - Removed `generate_dlocal_hmac_signature` helper.
    - Flow-specific `get_headers` now call the common `build_headers`.
    - Flow-specific `build_request` methods simplified to use `attach_default_headers()` and call `get_url`, `get_headers`, `get_request_body`.
    - GET request signature aligns with `real-codebase` (X-Login + X-Date + RequestBodyString [empty for GET]), differing from Dlocal docs (which include path for GET).
- **`ConnectorSpecifications` (Implemented):** Added static data and `impl ConnectorSpecifications` as per `real-codebase`.
- **Amount Handling Strategy:** Maintained alignment with `real-codebase` (i.e., use `i64` minor units and set `CurrencyUnit::Minor`).
- **Error Handling:** Documented common compilation errors and their solutions in `guides/errors/errors.md`.
- **Dependency Management:** `chrono`, `lazy_static`, `hex`, `ring` (indirectly via `common_utils::crypto`) are used.
- **Type Conversions:** Previous type mismatch resolutions maintained.
- **Import Resolution:** Previous import corrections maintained.
- **Enum Variants:** `RequestContent::NoContent` for empty GET/POST bodies, `RequestContent::Empty` for POST/PUT with `Content-Length: 0` but no actual body.
- **Module Paths & Trait Imports:** Previous corrections maintained.
- **Syntax Error from Tooling:** Documented and resolved past errors.
- **`Dlocal` Struct:** Removed unused `amount_converter` field from the `Dlocal` struct in `dlocal.rs` to align with `real-codebase` and resolve dead code warning.
