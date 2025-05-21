# Connector Learnings

_This file will store general learnings, best practices, and insights gained during connector development._

## Dlocal Integration Learnings (Session 1 - 2025-05-21)

### Dependency Management:
- **`chrono` Crate:** Essential for generating timestamps like `X-Date` header. Must be added to `Cargo.toml` if not present.
  ```toml
  chrono = { version = "0.4.38", features = ["serde"] } // Example version
  ```
- **`serde::Serialize` Derive:** Required for any struct that will be serialized to JSON, especially request bodies. Import `use serde::Serialize;` and add `#[derive(Serialize)]`.

### Type System and Conversions:
- **`StringMinorUnit` vs. `MinorUnit` vs. `f64` (Major Unit):**
    - Dlocal API expects amounts in `f64` (major units).
    - Internal Hyperswitch amounts might be `MinorUnit` (i64) or `StringMinorUnit`.
    - Conversion path: `StringMinorUnit` -> `i64` (minor) -> `f64` (major).
    - `StringMinorUnit` needs a method like `get_amount_as_i64()` (or parse string to i64).
    - `connector_utils::to_major_unit_as_f64(minor_amount_i64, currency)` is crucial for the final step.
- **`Email` vs. `Secret<String, EmailStrategy>`:**
    - `RouterData.request.email` is typically `Option<common_utils::pii::Email>`.
    - If a connector request struct field needs `Secret<String, common_utils::pii::EmailStrategy>`, direct assignment might fail.
    - The `real-codebase` for Dlocal uses `Option<Email>` in its `Payer` struct. My initial implementation aimed for `Secret<String, EmailStrategy>`.
    - The fix involved using `item.router_data.request.get_email_for_connector()` which likely handles the conversion or provides the correct type.
- **`ResponseId` for URLs:** `ResponseId` cannot be directly used in `format!`. It needs to be converted to a `String` using methods like `get_connector_transaction_id()`.
- **`RequestContent::Empty`:** For GET requests or requests with no body, use `RequestContent::Empty`, not `RequestContent::None`.

### HMAC Signature Generation:
- **`Secret<T>.expose()` vs. `Secret<T>.peek()`:** When dealing with `&Secret<String>` for HMAC keys, `expose()` takes ownership and causes a move error. `peek()` provides a `&String` reference, suitable for `as_bytes()`. Requires `use masking::PeekInterface;`.
- **Request Body for Signature:**
    - The exact JSON string of the request body is needed for Dlocal's HMAC signature.
    - `RequestContent::Json(Box<dyn ErasedMaskSerialize>)` cannot be easily stringified directly using a `.value()` method.
    - **Pattern:**
        1. Create the concrete request struct.
        2. Serialize this struct to a JSON string (`serde_json::to_string(&concrete_struct)`). This string is used for the signature.
        3. Box the *same concrete struct instance* into `RequestContent::Json(Box::new(concrete_struct))` for the actual HTTP request body. This ensures the signature matches the sent body.

### Trait Usage and Imports:
- **Helper Methods on `RouterData`:** Methods like `get_billing_address()`, `get_optional_billing_full_name()`, `get_email_for_connector()` are often provided by traits (e.g., `crate::utils::RouterData as _`). Ensure these traits are imported.
- **`RefundsRequestData` Trait:** Provides `get_connector_refund_id()`. Import `use crate::utils::RefundsRequestData;`.
- **`StringExt` Trait:** Provides `.parse_struct()` on `Bytes`. Import `use common_utils::ext_traits::StringExt;`.

### General Rust Practices:
- **Import `consts`:** For `NO_ERROR_CODE`, `NO_ERROR_MESSAGE`, use `hyperswitch_interfaces::consts;`.
- **Unused Imports:** The compiler will warn about these. Remove them to keep the code clean.
- **Boxed Options:** For fields like `redirection_data: Box<Option<RedirectForm>>`, assign `Box::new(None)` instead of just `None`.

### Dlocal Specifics:
- **Currency Unit:**
    - Dlocal API documentation suggests amounts in base/major units (e.g., "amount": 100.00).
    - However, the `real-codebase` for Dlocal sets `get_currency_unit()` to `api::CurrencyUnit::Minor` and passes amounts as `i64` (minor units) in request structs.
    - **Decision:** Align with `real-codebase` and use `api::CurrencyUnit::Minor`. This implies that `DlocalPaymentsRequest` and `DlocalRefundRequest` in `transformers.rs` should expect `i64` (minor unit) amounts, and the `DlocalRouterData` should also handle `i64`. The `to_major_unit_as_f64` utility will not be needed if amounts are consistently `i64` minor units.
- **Authentication Headers:** `X-Login`, `X-Trans-Key`, `X-Date`, `X-Version`, `User-Agent`, and the `Authorization: V2-HMAC-SHA256, Signature: ...` header are all constructed dynamically in the `build_request` method for each flow due to the signature's dependency on the request body and path.
- **URL Path for Signature:** The signature must include the request path and query string (e.g., `/secure_payments` or `/payments/ID/status`). `url::Url::parse()` and `.path()`, `.query()` methods are useful here.

## Dlocal Integration Learnings (Session 2 - 2025-05-21 - Post Second Build)

### Import Resolution:
- **`PaymentsResponseRouterData`:** This type alias is located in `crate::types`, not `hyperswitch_domain_models::router_response_types`. Correct import: `use crate::types::PaymentsResponseRouterData;`.
- **`ConnectorRequestReference`:** This item was not found in `hyperswitch_domain_models::router_data`. It might have been removed or renamed. If not essential, remove the import. (In my case, it was an incorrect addition).
- **`RouterData` Type:** When `RouterData` is used as a type (e.g., in function signatures or `try_from` calls), it usually refers to `hyperswitch_domain_models::router_data::RouterData`. Ensure this is imported directly, not just the `RouterData as _` trait alias from `crate::utils`.

### Amount Handling (Revisited):
- **`StringMinorUnit::new()` Privacy:** The `new()` constructor for `StringMinorUnit` is private. Use `StringMinorUnit::from(String)` or `StringMinorUnit::from(i64)` (if available and public) instead.
- **`to_major_unit_as_f64` Location:** This function, if needed, is typically in `crate::utils` (often aliased as `connector_utils`). The `real-codebase` for Dlocal uses `i64` minor units, so this conversion might be side-stepped if the API can handle minor units or if the `transformers.rs` structs are adjusted to use `i64`.
    - **Alignment Decision:** Based on `real-codebase` and `get_currency_unit()` being `Minor`, the `DlocalPaymentsRequest` and `DlocalRefundRequest` in `transformers.rs` should use `i64` for amount. The `DlocalRouterData` struct should also be adapted to hold `i64`. This simplifies amount handling and removes the need for `to_major_unit_as_f64` in `dlocal.rs`.

### Enum Variants:
- **`RequestContent::NoContent`:** For requests with no body (e.g., GET requests, or POSTs that Dlocal expects no body for like Void/Cancel if applicable), use `RequestContent::NoContent`. The `real-codebase` for Dlocal's GET requests in `build_request` simply doesn't call `.set_body()`, which is equivalent. For POSTs that expect no body, `RequestContent::NoContent` is appropriate.

## Dlocal Integration Learnings (Session 3 - 2025-05-21 - Post Third Build)

### Type Alias Generics:
- **`PaymentsResponseRouterData<R>`:** This alias in `crate::types` is defined with a single generic parameter `R`. If used with more than one (e.g., `PaymentsResponseRouterData<A, B>`), it causes an E0107 error.
    - **Solution:** Either correct the usage to provide only one generic argument if that's the intent, or if the alias is not flexible enough, use the full underlying type `ResponseRouterData<F, Resp, ReqBody, Output>` directly in the function signatures or `TryFrom` implementations. For the Dlocal `PaymentsSyncRouterData` `TryFrom` implementation, using the full `ResponseRouterData<PaymentsSyncRouterData, DlocalPaymentsResponse, PaymentsSyncData, PaymentsResponseData>` was the path taken.

### Trait Imports for Methods:
- **`AddressDetailsData`:** Methods like `get_full_name()` and `get_country()` on `hyperswitch_domain_models::address::AddressDetails` are provided by the `crate::utils::AddressDetailsData` trait. This trait must be imported.
- **`PaymentsAuthorizeRequestData` (and similar for other flows):** Methods like `get_email_for_connector()` or `get_webhook_url()` on `PaymentsAuthorizeData` (or `RefundsData`, etc.) are often part of specific request data traits (e.g., `crate::utils::PaymentsAuthorizeRequestData`). These need to be in scope.
    - **Note on `get_webhook_url()`:** This method might be on `item.router_data.request` (the specific `PaymentsAuthorizeData` etc.) rather than directly on `item.router_data` (the `RouterData` wrapper).

### Field Access in `TryFrom` for `RouterData` Wrappers:
- **Accessing `response` fields:** When implementing `TryFrom<ResponseRouterData<F, Resp, T, Output>> for RouterData<F, T, Output>`, the connector-specific response fields (like `id`, `order_id`) are on `item.response` (which is `Resp`), not directly on `item`.
- **Spreading `item.data`:** The `..item.data` spread is correct if `item.data` is of the same type as the `RouterData` struct being constructed (e.g., `RouterData<F, T, Output>`). If the `TryFrom` is for a more specific type alias that *is* `item.data` (e.g., `TryFrom<ResponseRouterData<...>> for PaymentsSyncRouterData`), then `..item.data` is correct. The E0308 mismatched types error for `..item.data` in the `PaymentsSyncRouterData` `TryFrom` was because the `TryFrom` should have been for the outer `RouterData<PaymentsSyncRouterData, _, _>` type.

### `RouterData::try_from` Resolution:
- The call `RouterData::try_from(response_router_data_instance)` relies on a specific `impl TryFrom<SpecificResponseRouterData> for SpecificRouterData` being available.
- If `handle_response` returns, for example, `CustomResult<PaymentsAuthorizeRouterData, ...>`, then the `TryFrom` implementation in `transformers.rs` must be `impl TryFrom<ResponseRouterData<Authorize, DlocalPaymentsResponse, PaymentsAuthorizeData, PaymentsResponseData>> for PaymentsAuthorizeRouterData`.
- The compiler errors (E0277, E0308) related to this often mean the `TryFrom` signature in `transformers.rs` does not exactly match what `dlocal.rs`'s `handle_response` expects, or the `RouterData::try_from` call is being made on the generic `RouterData` type instead of the specific type alias for which the `TryFrom` is implemented.
- **Resolution:** Ensure `handle_response` returns the specific `RouterData` alias (e.g., `PaymentsAuthorizeRouterData`) and that the `TryFrom` in `transformers.rs` is defined for this specific alias, converting from the appropriate `ResponseRouterData`. My previous fix correctly changed the `TryFrom` target to the generic `RouterData<F, T, PaymentsResponseData>`, which should work if `F` and `T` are correctly inferred by the compiler from the `handle_response` signature. The latest errors indicate that the `TryFrom` in `transformers.rs` needs to be for the specific `RouterData` aliases (e.g., `PaymentsAuthorizeRouterData`) rather than the generic `RouterData<F, T, OpResponse>`.

## Dlocal Integration Learnings (Session 4 - 2025-05-21 - Post Fourth Build)

### Module Paths for Flow Types and Traits:
- **Flow Types (Authorize, PSync, etc.):** These are located in `hyperswitch_interfaces::api`. So, use `api::Authorize`, `api::PSync`, etc., after `use hyperswitch_interfaces::api;`. The alias `hyperswitch_types` (for `hyperswitch_domain_models::types`) does not contain an `api` submodule.
- **`Capturable`, `Refundable` Traits:** These traits are defined in `hyperswitch_interfaces::types`. Import as `use hyperswitch_interfaces::types as hyperswitch_connector_types;` and use `hyperswitch_connector_types::Capturable`.

### Request Data Structs:
- **`PaymentsAuthorizeData`, `PaymentsSyncData`, `PaymentsCaptureData`, `RefundsData`:** These concrete request data structs (used as the `T` generic in `RouterData<F, T, Op>`) are located in `hyperswitch_domain_models::router_request_types`. They should be imported from there when defining the `Payable` and `Refundable` trait implementations.
- **Privacy of Type Aliases in `hyperswitch_domain_models::types`:** Some type aliases like `hyperswitch_domain_models::types::PaymentsCaptureData` might point to structs that are not publicly re-exported in a way that makes them directly usable as a generic argument in another crate. It's safer to use the direct path from `router_request_types`.

### `TryFrom` Implementation for `RouterData` Aliases:
- The `handle_response` functions in `dlocal.rs` return specific `RouterData` aliases (e.g., `PaymentsAuthorizeRouterData`).
- The `TryFrom` implementation in `transformers.rs` should be for these specific aliases, not for the generic `RouterData<F, T, OpResponse>`.
  ```rust
  // Example for Authorize flow in transformers.rs
  use hyperswitch_domain_models::types::PaymentsAuthorizeRouterData; // This is RouterData<api::Authorize, PaymentsAuthorizeData, PaymentsResponseData>
  // ...
  impl TryFrom<ResponseRouterData<api::Authorize, DlocalPaymentsResponse, PaymentsAuthorizeData, PaymentsResponseData>> for PaymentsAuthorizeRouterData {
      // ...
  }
  ```
  This ensures that when `PaymentsAuthorizeRouterData::try_from(...)` is called in `dlocal.rs`, it matches this specific implementation.

## Dlocal Integration Learnings (Session 5 - 2025-05-21 - Post Fifth Build)

### Trait Paths for Flow Markers:
- **`RefundableFlow` Trait:** This trait (or similar flow-specific marker traits if they exist) is not found in `hyperswitch_interfaces::types`. The generic parameter `F` in `RefundsRouterData<F>` is typically one of the specific flow structs like `Execute` or `RSync` from `hyperswitch_domain_models::router_flow_types::refunds`. Adding a separate trait bound like `F: RefundableFlow` is often unnecessary and can lead to "trait not found" errors if the trait doesn't exist or isn't in scope. The specific type of `F` already constrains the `RefundsRouterData`.
    - **Solution:** Remove the `where F: hyperswitch_connector_types::RefundableFlow` bound from `TryFrom<&DlocalRouterData<&RefundsRouterData<F>>> for DlocalRefundRequest`.

## Dlocal Integration Learnings (Session 6 - 2025-05-21 - Post Sixth Build)

### Duplicate Imports (E0252):
- **Cause:** Copy-pasting code blocks or manual editing can lead to importing the same item multiple times.
- **Solution:** Carefully review `use` statements and remove any duplicates. The Rust compiler usually provides good hints for this.

### Type Annotations for Trait Methods (E0283):
- **Cause:** When a method is part of a generic trait implementation (e.g., `get_content_type` in `ConnectorCommonExt` which is generic over `Flow, Req, Resp`), and the compiler cannot infer these generics in a specific call site, it may ask for type annotations.
- **Solution for Dlocal:**
    - The `build_headers` in `ConnectorCommonExt` was calling `self.get_content_type()`. For Dlocal, this was problematic.
    - The fix was to ensure that `build_request` in each specific flow (Authorize, Capture, etc.) directly uses `self.common_get_content_type()` when constructing the `Content-Type` header, as this method is non-generic and directly implemented by `Dlocal`.
    - The generic `build_headers` in `ConnectorCommonExt` for Dlocal now returns an empty Vec, as all headers are built in the flow-specific `build_request`.

### Method Scope (E0599 - `get_email_for_connector`):
- **Cause:** Calling a method on the wrong struct. `get_email_for_connector` is a helper method on `RouterData` (via `crate::utils::RouterData as _`), not directly on `RouterData.request` (which is, e.g., `PaymentsAuthorizeData`).
- **Solution:** Call the method on the correct instance: `item.router_data.get_email_for_connector()`.

### Enum Variant for No Request Body (E0599 - `RequestContent::NoContent`):
- **Cause:** Using an incorrect or non-existent variant for `RequestContent` when no body is intended.
- **Solution:** For POST/PUT requests that genuinely have no body but might need `Content-Length: 0`, use `RequestContent::Empty`. For GET/DELETE requests, simply do not call `.set_body()` on the `RequestBuilder`.

## Dlocal Integration Learnings (Session 7 - Post Tooling Error Fix)

### HMAC Signature Content (Re-evaluation):
- **Initial Approach:** My `generate_dlocal_hmac_signature` function took `request_body_str: Option<&str>` and `request_path_and_query: &str`, and conditionally used one or the other.
- **Dlocal Documentation & `real-codebase` Discrepancy:**
    - Dlocal's cURL examples show POST signatures based on `X-Login + X-Date + RequestBody`.
    - Dlocal's cURL examples show GET signatures based on `X-Login + X-Date + RequestPathAndQuery`.
    - The `real-codebase`'s generic `ConnectorCommonExt::build_headers` function prepares a signature string `X-Login + X-Date + RequestBodyContentString`. For GET requests where `RequestBodyContentString` would be empty, this results in a signature of `X-Login + X-Date`. This contradicts Dlocal's GET example.
- **Refined Approach for `generate_dlocal_hmac_signature`:**
    - The helper function should take a single `data_for_signature: &str` parameter.
    - The calling code in `build_request` for each flow will be responsible for constructing this `data_for_signature` string correctly:
        - For POST/PUT: `data_for_signature` will be the `request_body_str`.
        - For GET: `data_for_signature` will be the `request_path_and_query`.
    - The signature string itself will then be `X-Login + X-Date + data_for_signature`.
    - This makes the helper simpler and places the logic for what data to sign closer to the context of the HTTP method being used, aligning better with Dlocal's distinct examples.
- **Header Construction Strategy:**
    - Flow-specific `get_headers` methods should return `Ok(Vec::new())`.
    - All headers, including `Content-Type` (for POST/PUT) and all Dlocal-specific auth headers (`X-Login`, `X-Trans-Key`, `X-Date`, `X-Version`, `User-Agent`, `Authorization`), will be constructed within each flow's `build_request` method.
    - The `RequestBuilder::attach_default_headers()` call should be removed from `build_request` if all headers are being manually set.

## Dlocal Integration Learnings (Session 8 - Comparison with `real-codebase`)

### Header Generation Strategy (Alignment Implemented):
- **Adopted `real-codebase` Approach:**
    - Centralized all header generation (dynamic Dlocal auth headers: X-Login, X-Trans-Key, X-Date, X-Version; Authorization with HMAC; Content-Type) into `ConnectorCommonExt::build_headers`.
    - `ConnectorCommonExt::build_headers` now retrieves the request body string (handling different `RequestContent` variants), generates timestamps, extracts auth details, constructs the signature payload (`X-Login + X-Date + RequestBodyString`), generates the HMAC, and assembles all headers.
    - For GET requests, `RequestBodyString` is empty, so the signature payload becomes `X-Login + X-Date`. This differs from Dlocal's documentation (which includes path for GET) but aligns with `real-codebase`.
- **Changes Made:**
    - Removed the standalone `generate_dlocal_hmac_signature` helper function.
    - Flow-specific `get_headers` methods now directly call `self.build_headers(req, connectors)`.
    - Flow-specific `build_request` methods now use `RequestBuilder::new().attach_default_headers().headers(self.get_headers(...))...`, relying on the centralized header generation.
    - Ensured `DlocalAuthType` in `transformers.rs` correctly provides `x_login`, `x_trans_key`, and `secret_key`.

### `ConnectorSpecifications` Implementation (Alignment Implemented):
- **Adopted `real-codebase` Approach:**
    - Added static definitions for `DLOCAL_SUPPORTED_PAYMENT_METHODS`, `DLOCAL_CONNECTOR_INFO`, and `DLOCAL_SUPPORTED_WEBHOOK_FLOWS` using `lazy_static!`.
    - Implemented `ConnectorSpecifications for Dlocal` to provide these static details.
- **Changes Made:**
    - Included `lazy_static` in imports.
    - Defined the static variables and the `impl ConnectorSpecifications` block in `dlocal.rs`.

### Imports (Alignment Implemented):
- **Adopted `real-codebase` Imports:**
    - Ensured `common_utils::date_time` is used for timestamps.
    - Ensured `common_utils::crypto::SignMessage` and `common_utils::crypto::HmacSha256` are used for HMAC generation.
    - Added `hex::encode` for signature encoding.
    - Added `lazy_static::lazy_static`.
    - Other necessary imports for types like `ConnectorInfo`, `SupportedPaymentMethods`, etc., from `hyperswitch_domain_models` and `api_models` are included.

### Other Minor Alignments:
- **`build_error_response`:** Aligned the `reason` field mapping with `real-codebase` (using `response.param`).
- **RSync URL:** Aligned the RSync URL to `{}refunds/{}` as per `real-codebase` (DLocal docs suggest `{}refunds/{}/status`).
- **`get_request_body` for PSync/Void/RSync:** Ensured these return `Ok(RequestContent::NoContent)` as these are GET or bodyless POSTs.
- **`get_content_type` for PSync/RSync:** Returns `""` as these are GET requests.
- **Webhook Event Type:** `get_webhook_event_type` now returns `Ok(IncomingWebhookEvent::EventNotSupported)` aligning with `real-codebase`.
