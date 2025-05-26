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

## Airwallex Implementation Comparison

### Topic: Import Paths and Type Aliases

#### My Initial (Problematic) Approach:
In `crates/hyperswitch_connectors/src/connectors/airwallex/transformers.rs`:
```rust
// Attempting to import RouterData wrappers from crate::types
use crate::{
    types::{
        RefundsResponseRouterData, ResponseRouterData, ResponseRouterDataCommon, 
        PaymentsAuthorizeRouterData, // ... and other Payments...RouterData types
    },
};

// Attempting to import AccessTokenResponseRouterData from hyperswitch_types (alias for hyperswitch_domain_models::types)
use hyperswitch_types::{
    AccessTokenResponseRouterData, // ...
};
```
In `crates/hyperswitch_connectors/src/connectors/airwallex.rs`:
```rust
// Attempting to import various types from hyperswitch_domain_models::types directly or via aliases
use hyperswitch_domain_models::{
    types::{ 
        AccessTokenAuthRouterData, 
        ResponseRouterData as HyperswitchDomainResponseRouterData, 
        ConnectorTransactionIdType, 
        // ... other ...RouterData types
    },
};
use hyperswitch_interfaces::{
    api::{self, payments::AccessTokenAuthType}, // Trying to get AccessTokenAuthType trait
};

// In AccessTokenAuth flow:
// .url(&types::AccessTokenAuthType::get_url(self, req, connectors)?) // Error: AccessTokenAuthType not in types
// RouterData::try_from(types::ResponseRouterData { ... }) // Error: ResponseRouterData not in types (or wrong one)
```

#### Reference (Corrected) Approach:
In `crates/hyperswitch_connectors/src/connectors/airwallex/transformers.rs`:
```rust
use crate::{ // crate::types is hyperswitch_interfaces::types
    types::{
        RefundsResponseRouterData, ResponseRouterData, 
    },
};
use hyperswitch_interfaces::types::ResponseRouterDataCommon; // Correct path for this trait

// hyperswitch_types is an alias for hyperswitch_domain_models::types
use hyperswitch_types::{
    PaymentsAuthorizeRouterData, PaymentsCaptureRouterData, PaymentsCancelRouterData,
    PaymentsCompleteAuthorizeRouterData, PaymentsPreProcessingRouterData, PaymentsSyncRouterData,
    AccessTokenResponseRouterData, 
};
```
In `crates/hyperswitch_connectors/src/connectors/airwallex.rs`:
```rust
use hyperswitch_domain_models::{
    router_request_types::{CompleteAuthorizeData}, // Specific request data struct
    types::{ 
        AccessTokenAuthRouterData, // This is a type alias in hyperswitch_domain_models::types
        ResponseRouterData as HyperswitchDomainResponseRouterData, // Alias for clarity
        ConnectorTransactionIdType, // Enum from hyperswitch_domain_models::types
        // Other ...RouterData wrappers are also from here
    },
};
use hyperswitch_interfaces::{
    api::{self, payments::AccessTokenAuthType as AccessTokenAuthTypeTrait}, // Correct path and alias for the trait
};

// In AccessTokenAuth flow:
// .url(&AccessTokenAuthTypeTrait::get_url(self, req, connectors)?) // Using the aliased trait
// RouterData::try_from(HyperswitchDomainResponseRouterData { ... }) // Using the aliased type
```

#### Differences:
1.  **`ResponseRouterDataCommon`**: Initially tried to import from `crate::types`, but it's directly in `hyperswitch_interfaces::types`.
2.  **`AccessTokenResponseRouterData`**: Correctly imported from `hyperswitch_types` (alias for `hyperswitch_domain_models::types`) in the `transformers.rs` eventually, but initial attempts might have been from `crate::types`.
3.  **`Payments...RouterData` Wrappers**: These are type aliases defined in `hyperswitch_domain_models::types` (accessible via `hyperswitch_types`). My initial imports in `transformers.rs` from `crate::types` were problematic because `crate::types` (i.e. `hyperswitch_interfaces::types`) might not re-export all of them or might have its own versions.
4.  **`AccessTokenAuthRouterData`**: This is a type alias in `hyperswitch_domain_models::types`.
5.  **`ConnectorTransactionIdType`**: This enum is in `hyperswitch_domain_models::types`.
6.  **`AccessTokenAuthType` Trait**: This trait is located in `hyperswitch_interfaces::api::payments`, not directly under `hyperswitch_interfaces::api` or `hyperswitch_interfaces::types`.
7.  **`CompleteAuthorizeData` vs `PaymentsCompleteAuthorizeData`**: The request type struct is `CompleteAuthorizeData` (in `router_request_types`), while the router data wrapper is `PaymentsCompleteAuthorizeRouterData` (a type alias in `hyperswitch_domain_models::types`).
8.  **`ResponseRouterData` Alias**: In `airwallex.rs`, `hyperswitch_domain_models::types::ResponseRouterData` was aliased to `HyperswitchDomainResponseRouterData` to avoid conflict with `crate::types::ResponseRouterData` (which is `hyperswitch_interfaces::types::ResponseRouterData`).

#### Lessons Learned:
1.  **Clarity on Crate Structure**: The distinction between `hyperswitch_domain_models`, `hyperswitch_interfaces`, and their respective `types` modules is crucial. `crate::types` is an alias for `hyperswitch_interfaces::types`. `hyperswitch_types` is often used as an alias for `hyperswitch_domain_models::types`.
2.  **Specific vs. Generic Types**: `RouterData` is a generic struct. Specific instances like `PaymentsAuthorizeRouterData` are type aliases. Traits like `AccessTokenAuthType` are distinct from these data structures.
3.  **Compiler Hints are Key**: The compiler's suggestions ("no `TypeName` in `module`", "a similar name exists") are invaluable for pinpointing the correct modules and type/trait names.
4.  **Aliasing for Clarity**: When type names might clash or paths are long, aliasing (`use ... as ...`) can improve readability and prevent errors, as done with `AccessTokenAuthTypeTrait` and `HyperswitchDomainResponseRouterData`.
5.  **Trait vs. Struct**: `AccessTokenAuthType` is a trait that defines behavior (like `get_url`), while `AccessTokenAuthRouterData` is a struct (actually a type alias for `RouterData<AccessTokenAuth, ...>`) that holds data. They are used differently.
6.  **`ResponseRouterDataCommon` Location**: This trait is in `hyperswitch_interfaces::types`.
7.  **Router Data Wrappers**: Types like `PaymentsAuthorizeRouterData` are typically aliases found in `hyperswitch_domain_models::types`.

## Advanced Learnings from Real Codebase (Airwallex Example - 21/05/2025)

### Topic: Correcting Common Import and Type Usage Errors

#### 1. `ResponseRouterDataCommon` Trait
*   **Observation**: This trait, previously thought to be at `hyperswitch_interfaces::types::ResponseRouterDataCommon`, was not found in use in `real-codebase/airwallex/transformers.rs` for constraining `TryFrom` implementations.
*   **Lesson**: It's likely deprecated, internal, or was a misunderstanding. `TryFrom` implementations for `RouterData` should not be constrained by it. The `where Self: ResponseRouterDataCommon<...>` clause should be removed.

#### 2. `AccessTokenResponseRouterData` and Orphan Rules
*   **Observation**: `AccessTokenResponseRouterData` is NOT a public type alias in `hyperswitch_domain_models::types` (or its alias `hyperswitch_types`).
*   **Problem**: Directly implementing `TryFrom<hyperswitch_domain_models::router_data::RouterData<...>> for hyperswitch_domain_models::router_data::RouterData<...>>` violates Rust's orphan rule (E0117), as both `TryFrom` and `RouterData` are foreign.
*   **Solution Pattern (from `real-codebase/airwallex/transformers.rs` for `AirwallexAuthUpdateResponse`):**
    *   The `TryFrom` implementation should be on the local wrapper type `crate::types::ResponseRouterData`.
    *   Signature: `impl<F, T> TryFrom<crate::types::ResponseRouterData<F, ActualConnectorResponseType, RequestBodyType, TargetResponseType>> for hyperswitch_domain_models::router_data::RouterData<F, RequestBodyType, TargetResponseType>`
    *   Example for Access Token:
        ```rust
        // In transformers.rs
        use crate::types::ResponseRouterData; // This is hyperswitch_connectors::types::ResponseRouterData
        use hyperswitch_domain_models::{
            router_data::{AccessToken, RouterData as HyperswitchRouterData},
            router_flow_types::access_token_auth::AccessTokenAuth,
            router_request_types::AccessTokenRequestData,
        };
        // ... other imports ...
        // pub struct AirwallexAuthUpdateResponse { /* ... */ }

        impl TryFrom<ResponseRouterData<AccessTokenAuth, AirwallexAuthUpdateResponse, AccessTokenRequestData, AccessToken>>
            for HyperswitchRouterData<AccessTokenAuth, AccessTokenRequestData, AccessToken>
        {
            type Error = error_stack::Report<errors::ConnectorError>;
            fn try_from(
                item: ResponseRouterData<AccessTokenAuth, AirwallexAuthUpdateResponse, AccessTokenRequestData, AccessToken>,
            ) -> Result<Self, Self::Error> {
                // 'item.response' here is AirwallexAuthUpdateResponse directly, not Result<>
                // 'item.data' is the original RouterData passed into crate::types::ResponseRouterData
                Ok(Self {
                    response: Ok(AccessToken {
                        token: item.response.token, // Assuming token is a field in AirwallexAuthUpdateResponse
                        expires: (item.response.expires_at - common_utils::date_time::now()).whole_seconds(), // Example calculation
                    }),
                    ..item.data // Spread fields from the original RouterData
                })
            }
        }
        ```
*   **Lesson**: Use the local `crate::types::ResponseRouterData` as the input for `TryFrom` when the target is `hyperswitch_domain_models::router_data::RouterData` to satisfy orphan rules. The `crate::types::ResponseRouterData` struct likely holds the connector's response directly (not as a `Result`).

#### 3. `ConnectorTransactionIdType` vs `api_models::payments::PaymentIdType`
*   **Observation**: The type previously assumed to be `ConnectorTransactionIdType` (and thought to be in `hyperswitch_domain_models::types`) is actually `api_models::payments::PaymentIdType`.
*   **Usage in Webhooks**: When constructing `api_models::webhooks::ObjectReferenceId::PaymentId(...)`, the argument should be of type `api_models::payments::PaymentIdType`.
*   **Import**: `use api_models::payments::PaymentIdType;` (assuming `api_models` is a direct dependency of the connector crate).
*   **Example**:
    ```rust
    // In airwallex.rs (webhook handling)
    use api_models::payments::PaymentIdType;
    use hyperswitch_domain_models::id_type; // For id_type::PaymentId
    use std::borrow::Cow;
    // ...
    // let source_id_string = details.source_id; // From webhook payload
    // let payment_id = id_type::PaymentId::try_from(Cow::Owned(source_id_string))?;
    // Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
    //     PaymentIdType::PaymentIntentId(payment_id)
    // ))
    ```

#### 4. `id_type` Module Location
*   **Observation**: The `id_type` module (containing `PaymentId`, `MerchantId`, etc.) is located at `common_utils::id_type`.
*   **Import**: `use common_utils::id_type;`

#### 5. `AccessTokenAuthType` Trait
*   **Observation**: The trait `AccessTokenAuthType` was not found at `hyperswitch_interfaces::api::payments`. The access token flow is primarily managed by `impl api::ConnectorAccessToken for ConnectorName {}` and the corresponding `impl ConnectorIntegration<AccessTokenAuth, ...> for ConnectorName {}`.
*   **Usage in `build_request`**: Calls to `get_url`, `get_headers`, `get_request_body` within the `AccessTokenAuth` flow's `build_request` method (if these are implemented directly in the `ConnectorIntegration` block) should be `self.method_name(...)`.
*   **Lesson**: Avoid importing or using non-existent helper traits. Rely on the methods defined in the direct `ConnectorIntegration` implementation.

#### 6. `RequestContent::Empty`
*   **Observation**: For an empty request body, use `RequestContent::Empty`, not `RequestContent::None`. Ensure `common_utils::request::RequestContent` is correctly imported.
*   **Correction (21/05/2025)**: For Airwallex `AccessTokenAuth` (login), the API expects an empty JSON object `{}`. So, `RequestContent::Json(Box::new(AirwallexAuthUpdateRequest {}))` (where `AirwallexAuthUpdateRequest` is an empty struct deriving `Serialize`) is more appropriate than `RequestContent::Empty` if `Empty` means no body at all.

#### 7. Deriving `Serialize` and `Default`
*   **`Serialize`**: Connector-specific response structs (e.g., `AirwallexAuthUpdateResponse`) that are passed to `event_builder.set_response_body()` need to derive `serde::Serialize`.
*   **`Default`**: If a struct containing a status enum (e.g., `AirwallexPaymentsResponse` containing `AirwallexPaymentStatus`) derives `Default`, then the status enum itself must also derive `Default` or have a `#[default]` variant for `serde`.

#### 8. Request Data Accessor Traits
*   **Observation**: Methods like `get_amount()`, `get_currency()`, `get_browser_info()`, `get_router_return_url()` are provided by traits defined in `crate::utils` (e.g., `PaymentsAuthorizeRequestData`, `PaymentsPreProcessingRequestData`).
*   **Import**: These traits must be imported into the scope where the methods are used (typically `transformers.rs`).
    ```rust
    // Example in transformers.rs
    use crate::utils::{PaymentsAuthorizeRequestData, PaymentsPreProcessingRequestData /*, etc. */};
    ```

#### 9. `ByteSliceExt::parse_struct`
*   **Observation**: The `parse_struct` method is an extension trait method.
*   **Import**: `use common_utils::ext_traits::ByteSliceExt;` is required in the scope where `response.response.parse_struct(...)` is called (typically `airwallex.rs`).

#### 10. `ConnectorAuthType` and Bearer Tokens
*   **Observation**: `ConnectorAuthType` (from `hyperswitch_domain_models::router_data`) does *not* have an inherent `get_access_token()` method, nor a `TokenAuth` variant.
*   **Lesson**:
    *   After a successful `AccessTokenAuth` flow, the obtained `AccessToken` should be stored in `RouterData.access_token`.
    *   The `RouterData.connector_auth_type` should remain unchanged (e.g., as `BodyKey`), as it holds the initial credentials.
    *   For subsequent authenticated API calls, the `ConnectorCommonExt::build_headers` method should retrieve the Bearer token from `RouterData.access_token` (if the flow is not `AccessTokenAuth` itself).
    *   The `AccessTokenAuth::get_headers` method should use the initial credentials from `req.connector_auth_type` (e.g. API key, client ID) to make the login request.

#### 11. Populating `RedirectForm.form_fields`
*   **Observation**: The `form_fields` field is `HashMap<String, String>`. Values from `serde_json::Value` must be converted to `String`.
*   **Pattern**: Iterate over the JSON map and convert values, handling potential non-string values gracefully.
    ```rust
    // Example for RedirectForm construction
    // let data_map: serde_json::Map<String, serde_json::Value> = ...;
    // let form_fields: std::collections::HashMap<String, String> = data_map
    //     .into_iter()
    //     .filter_map(|(k, v)| v.as_str().map(|s| (k, s.to_string()))) // Or handle other Value types
    //     .collect();
    ```
    The `real-codebase/airwallex/transformers.rs` uses a simpler approach by directly creating HashMap entries, assuming specific fields.

#### 12. `MandateReference` in `PaymentsResponseData`
*   **Observation**: `MandateReference` struct (from `hyperswitch_domain_models::router_response_types`) is used to hold mandate details. For Airwallex, if `payment_consent_id` is available, it can be used. If not, or if full mandate details are not applicable, `Box::new(None)` can be used for the `mandate_reference` field in `PaymentsResponseData`. The `real-codebase` uses `Box::new(None)`.
*   **Lesson**: The type `MandateReferenceDetails` was causing import issues. Using `Box::new(None)` for `mandate_reference` in `PaymentsResponseData::TransactionResponse` for Airwallex is a simpler and valid approach if detailed mandate info isn't being mapped.

#### 13. `RouterData.response` Field
*   **Observation**: The `response` field in `hyperswitch_domain_models::router_data::RouterData` is `Result<ActualResponseType, ErrorResponse>`.
*   **Lesson**: When constructing `RouterData` in `handle_response` or `TryFrom` implementations, the connector's response should be wrapped in `Ok(...)`, e.g., `response: Ok(AccessToken { ... })`.

#### 14. `RouterData` Construction in `AccessTokenAuth::handle_response`
*   **Lesson**: The `AccessTokenAuth::handle_response` should return an owned `RouterData` (e.g. `RefreshTokenRouterData`). This `RouterData` should have its `response` field set to `Ok(AccessToken { ... })` and its `access_token` field set to `Some(AccessToken { ... })`. The `connector_auth_type` field should be copied from the input `data.connector_auth_type` and *not* changed to a non-existent `TokenAuth` variant. Other fields should be copied from the input `data` as appropriate for the `RefreshTokenRouterData` structure. Avoid populating fields not present in the generic `RouterData` or `RefreshTokenRouterData` definition.
*   **Correction (21/05/2025)**: When constructing `RefreshTokenRouterData`, ensure all fields are initialized, typically by using `..data.clone()` and then overriding only the necessary fields (`response`, `access_token`, `connector_http_status_code`). Removed the attempt to set `status: common_enums::AttemptStatus::Tokenized` as it's not a valid variant and status should be preserved or handled generically.

#### 15. `amount_captured` field in `RouterData`
*   The `RouterData` struct has `amount_captured: Option<i64>`.
*   The `PaymentsResponseData::TransactionResponse` does not have this field.
*   When constructing the final `RouterData` in `handle_response` or `TryFrom`, if the payment is successful (e.g., status is Charged), `amount_captured` on the `RouterData` should be updated with the actual amount captured (often `item.response.amount` from connector, converted to minor units if needed). My previous attempt to put it inside `PaymentsResponseData::TransactionResponse` was incorrect.
*   The `TryFrom<ResponseRouterData<F, AirwallexPaymentsResponse, T, PaymentsResponseData>> for RouterData<F, T, PaymentsResponseData>` in `transformers.rs` (real codebase) does not set `amount_captured` directly. It sets the `status`. The `amount_captured` on `RouterData` is likely updated in a subsequent step or based on the `status`. For now, I will focus on getting the `response` field correct.

#### 16. Connector Transaction ID Source (`RouterData.reference_id`)
*   **Observation**: For flows like Authorize, Capture, Void, CompleteAuthorize that operate on an existing payment intent (created during PreProcessing), the `connector_transaction_id` (which is the Airwallex `payment_intent_id`) is stored in `RouterData.reference_id`.
*   **Lesson**: In `get_url` methods for these flows, use `req.reference_id.clone().ok_or(...)` to get the payment intent ID.

#### 17. Connector Transaction ID Source for PSync (`PaymentsSyncData.connector_transaction_id`)
*   **Observation**: For `PSync`, the `PaymentsSyncData.connector_transaction_id` field holds an `Option<ConnectorTransactionId>`. The actual string ID is obtained by calling the inherent method `get_connector_transaction_id()` on the `ConnectorTransactionId` struct.
*   **Lesson**: The trait `PaymentsSyncRequestData::get_connector_transaction_id()` (implemented for `PaymentsSyncData`) returns `Result<String, _>`. Use `req.request.get_connector_transaction_id()?`.

#### 18. Amount Representation in `AirwallexIntentRequest`
*   **Observation**: The `real-codebase`'s `AirwallexIntentRequest.amount` is `String`. It converts the `i64` amount from `PaymentsPreProcessingData` to a minor unit string using `utils::to_currency_base_unit`.
*   **Lesson**: My `AirwallexIntentRequest.amount` was `StringMinorUnit`. Changing it to `String` and using `crate::utils::to_currency_base_unit` aligns with `real-codebase` and resolves type errors.

#### 19. Amount in `AirwallexPaymentsCaptureRequest`
*   **Observation**: `PaymentsCaptureData.amount_to_capture` is `i64`. The `AirwallexPaymentsCaptureRequest.amount` field in `real-codebase` is `Option<String>`, populated by converting `amount_to_capture` using `utils::to_currency_base_unit`.
*   **Lesson (Initial)**: My `AirwallexPaymentsCaptureRequest.amount` was `Option<StringMinorUnit>`. The conversion `Some(StringMinorUnit::from(item.request.amount_to_capture))` was thought to be correct.
*   **Correction (21/05/2025)**: `StringMinorUnit::from(i64)` is incorrect (type mismatch). `StringMinorUnit::new(String)` is private. The field `AirwallexPaymentsCaptureRequest.amount` should be `Option<String>`. The conversion should use `crate::utils::to_currency_base_unit(item.request.amount_to_capture, item.request.currency)` to get a `String` representing minor units.

#### 20. `Debug` for Trait Objects
*   **Observation**: Structs containing trait objects (e.g., `amount_converter: &'static (dyn AmountConvertor<...> + Sync)`) cannot automatically derive `Debug` if the trait object itself doesn't implement `Debug`.
*   **Lesson**: Remove `#[derive(Debug)]` from such structs or implement `Debug` manually if needed. (The `Airwallex` struct in `airwallex.rs` was deriving `Clone` but not `Debug`, so this was not an active issue but a general learning).

#### 21. Fully Qualified Enum Variants
*   **Observation**: Using `enums::AttemptStatus` or `enums::PaymentMethod` can lead to "unresolved module `enums`" if `enums` is not a recognized path.
*   **Lesson**: Use the full path from the crate, e.g., `common_enums::AttemptStatus` or `common_enums::PaymentMethod`.

#### 22. `ConnectorRedirectResponse` Trait Implementation (Airwallex - 21/05/2025)
*   **Problem**: `Airwallex` struct needs to implement `ConnectorRedirectResponse` to satisfy the `Connector` trait bound.
*   **Trait Definition**: Found in `hyperswitch_interfaces::api::ConnectorRedirectResponse`.
    ```rust
    pub trait ConnectorRedirectResponse {
        fn get_flow_type(
            &self,
            _query_params: &str,
            _json_payload: Option<serde_json::Value>,
            _action: common_enums::enums::PaymentAction, // Note: common_enums::enums path
        ) -> CustomResult<common_enums::enums::CallConnectorAction, errors::ConnectorError> { // Note: common_enums::enums path
            Ok(common_enums::enums::CallConnectorAction::Avoid)
        }
    }
    ```
*   **Key Learnings for Implementation**:
    *   The trait only has one method: `get_flow_type`. My previous placeholder incorrectly included `get_connector_redirect_response`.
    *   The `_action` parameter is of type `common_enums::enums::PaymentAction`.
    *   The return type is `CustomResult<common_enums::enums::CallConnectorAction, errors::ConnectorError>`.
    *   The default implementation returns `Ok(CallConnectorAction::Avoid)`. For Airwallex, if a redirect always implies a next step (like `CompleteAuthorize`), returning `Ok(CallConnectorAction::Trigger)` might be more appropriate.
*   **Imports**: Need `use common_enums::enums::{CallConnectorAction, PaymentAction};` in `airwallex.rs`.
## Bambora Connector (bambora.rs) Implementation Comparison

This document compares the implemented `crates/hyperswitch_connectors/src/connectors/bambora.rs` (My Implementation) with `real-codebase/bambora.rs` (Reference Implementation).

### Overall Structure & Imports
*   Both files have a similar overall structure, implementing the necessary traits.
*   The reference implementation has more specific imports from `hyperswitch_domain_models` and `hyperswitch_interfaces` due to its more detailed implementation of flows like 3DS and Void.

---

### Struct `Bambora` and `new()`

#### My Implementation:
```rust
#[derive(Clone)]
pub struct Bambora {
    amount_converter: &'static (dyn AmountConvertor<Output = StringMinorUnit> + Sync)
}

impl Bambora {
    pub fn new() -> &'static Self {
        &Self {
            amount_converter: &StringMinorUnitForConnector
        }
    }
}
```

#### Reference Implementation:
```rust
#[derive(Debug, Clone)]
pub struct Bambora;

// No explicit `new()` or `amount_converter` field.
// Amount conversion is handled within each flow's get_request_body using
// `bambora::BamboraRouterData::try_from((&self.get_currency_unit(), ...))`
```

#### Differences:
1.  **`amount_converter` field**: My implementation included an `amount_converter` field. The reference implementation does not store this in the struct but rather calls `self.get_currency_unit()` and passes it to the `BamboraRouterData::try_from` during request body generation.
2.  **`new()` constructor**: Present in mine to initialize `amount_converter`. Absent in reference.

#### Lessons Learned:
*   The reference approach of fetching `currency_unit` on-demand within `get_request_body` is cleaner and avoids storing it in the struct. The `BamboraRouterData` in the reference `transformers.rs` is designed to take `&api::CurrencyUnit` as input for its `TryFrom`.

---

### `ConnectorCommon::get_currency_unit()`

#### My Implementation:
```rust
fn get_currency_unit(&self) -> api::CurrencyUnit {
    api::CurrencyUnit::Minor
}
```

#### Reference Implementation:
```rust
fn get_currency_unit(&self) -> api::CurrencyUnit {
    api::CurrencyUnit::Base // This is a key difference!
}
```

#### Differences:
1.  **Returned Unit**: Mine returns `Minor`, reference returns `Base`.
    *   My reasoning for `Minor` was based on the Bambora doc stating "Currency Unit: Bambora uses minor units". However, the API examples show `100.0` for $100.
    *   The reference `transformers.rs` `BamboraRouterData::try_from` takes `(&api::CurrencyUnit, enums::Currency, i64, T)` and uses `utils::get_amount_as_f64(currency_unit, amount, currency)`. If `currency_unit` is `Base`, `get_amount_as_f64` would interpret the `i64` amount (which is Hyperswitch's internal minor unit amount) as a base unit if not careful.
    *   **Correction**: The reference `transformers.rs` `BamboraRouterData` expects the *output* amount to be `f64` (major units). The `utils::get_amount_as_f64` correctly converts the input `i64` (minor units) to `f64` (major units) *regardless* of whether `currency_unit` is Base or Minor, as long as the `currency_unit` correctly describes the input `i64 amount`.
    *   If Hyperswitch's internal `req.request.amount` (which is `i64`) is *always* in minor units, then `get_currency_unit()` should be `api::CurrencyUnit::Minor` to correctly describe this internal representation to `utils::get_amount_as_f64`. The reference `get_currency_unit()` returning `Base` seems contradictory if `req.request.amount` is minor. This needs careful checking of `utils::get_amount_as_f64`'s behavior.
    *   **Re-evaluation**: The Bambora documentation *text* says "minor units" but the *API example* shows `100.0`. The reference `transformers.rs` `BamboraPaymentsRequest` has `amount: f64`. The `utils::get_amount_as_f64` converts an `i64` (assumed minor unit from Hyperswitch core) to an `f64` (major unit for Bambora). So, `get_currency_unit()` should describe the unit *Bambora expects for its `amount` field if it were an integer*, or more accurately, it describes the unit of Hyperswitch's internal `amount` field. If Hyperswitch's `req.request.amount` is `i64` minor units, then `get_currency_unit()` should be `Minor`. The reference returning `Base` here is confusing and potentially an error if `req.request.amount` is minor. My choice of `Minor` seems more consistent with Hyperswitch's internal minor unit representation.

#### Lessons Learned:
*   The interpretation of `get_currency_unit()` is critical. It should reflect the unit of the amount *as it is stored in Hyperswitch's `RouterData`* before any connector-specific conversion. If Hyperswitch's `PaymentsAuthorizeData.request.amount` (an `i64`) is in minor units, then `get_currency_unit()` must return `Minor`. The conversion to Bambora's `f64` major unit is a separate step. The reference code's `Base` here is suspect.

---

### `ConnectorCommon::get_auth_header()`

#### My Implementation:
```rust
fn get_auth_header(&self, auth_type:&ConnectorAuthType)-> CustomResult<Vec<(String,masking::Maskable<String>)>,errors::ConnectorError> {
    let auth =  bambora::BamboraAuthType::try_from(auth_type)
        .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
    Ok(vec![(headers::AUTHORIZATION.to_string(), auth.api_key.expose().into_masked())])
}
// My transformers.rs expected ConnectorAuthType::HeaderKey
```

#### Reference Implementation:
```rust
fn get_auth_header(
    &self,
    auth_type: &ConnectorAuthType,
) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
    let auth = bambora::BamboraAuthType::try_from(auth_type)
        .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
    Ok(vec![(
        headers::AUTHORIZATION.to_string(),
        auth.api_key.into_masked(), // api_key already contains "Passcode <base64>"
    )])
}
// Reference transformers.rs expected ConnectorAuthType::BodyKey and constructed the full header.
```

#### Differences:
1.  The core difference stems from `transformers.rs`: my `BamboraAuthType::try_from` expected `HeaderKey` and assumed `api_key` was just the passcode. The reference `BamboraAuthType::try_from` expects `BodyKey` (with merchant ID in `key1` and passcode in `api_key`) and constructs the full `Passcode <base64_encoded_merchant:passcode>` string.
2.  My `expose().into_masked()` vs reference `into_masked()` is minor if `auth.api_key` is already the final header value.

#### Lessons Learned:
*   The reference implementation's handling of auth (constructing the full header string from components in `transformers.rs`) is more robust.

---

### `ConnectorCommon::build_error_response()`

#### My Implementation:
```rust
Ok(ErrorResponse {
    status_code: res.status_code,
    code: response.code, // From my simple BamboraErrorResponse
    message: response.message,
    reason: response.reason,
    // ...
})
```

#### Reference Implementation:
```rust
Ok(ErrorResponse {
    status_code: res.status_code,
    code: response.code.to_string(), // From detailed BamboraErrorResponse (code is i32)
    message: serde_json::to_string(&response.details) // Uses details for message
        .unwrap_or(hyperswitch_interfaces::consts::NO_ERROR_MESSAGE.to_string()),
    reason: Some(response.message), // Uses Bambora's top-level message as reason
    // ...
})
```

#### Differences:
1.  **Error Source**: Mine uses a simpler `BamboraErrorResponse`. Reference uses a detailed one matching Bambora's actual error structure.
2.  **Message/Reason Mapping**: Reference maps Bambora's `details` array to `message` and Bambora's `message` to `reason`. Mine had a more direct mapping.

#### Lessons Learned:
*   Mapping error fields more precisely to the connector's specific error response structure provides better error information.

---

### Flow: `Authorize`

#### My Implementation `get_url()`:
```rust
fn get_url(&self, _req: &PaymentsAuthorizeRouterData, connectors: &Connectors,) -> CustomResult<String,errors::ConnectorError> {
    Ok(format!("{}{}", self.base_url(connectors), "/payments"))
}
```
#### Reference Implementation `get_url()`:
```rust
fn get_url(
    &self,
    _req: &PaymentsAuthorizeRouterData,
    connectors: &Connectors,
) -> CustomResult<String, errors::ConnectorError> {
    Ok(format!("{}{}", self.base_url(connectors), "/v1/payments")) // Includes /v1/
}
```
#### My Implementation `get_request_body()`:
```rust
let amount_minor_unit = utils::convert_amount( // Uses self.amount_converter
    self.amount_converter,
    req.request.minor_amount,
    req.request.currency,
)?;
let connector_router_data = bambora::BamboraRouterData::try_from((
    amount_minor_unit, // StringMinorUnit
    req.request.currency,
    req,
))?;
// ...
```
#### Reference Implementation `get_request_body()`:
```rust
let connector_router_data = bambora::BamboraRouterData::try_from((
    &self.get_currency_unit(), // api::CurrencyUnit::Base in reference
    req.request.currency,
    req.request.amount, // This is i64 (minor units from Hyperswitch core)
    req,
))?;
// ...
```
#### My Implementation `handle_response()`:
Parses into `bambora::BamboraPaymentsResponse` (my simpler version).
#### Reference Implementation `handle_response()`:
Parses into `bambora::BamboraResponse` (enum for Normal vs 3DS).

#### Differences:
1.  **URL Path**: Reference includes `/v1/` in the path. Mine did not.
2.  **Amount Conversion in `get_request_body`**:
    *   Mine explicitly called `utils::convert_amount` (using `self.amount_converter`) to get `StringMinorUnit` then passed this to `BamboraRouterData::try_from`.
    *   Reference passes `&self.get_currency_unit()` (which it defines as `Base`), `req.request.currency`, and `req.request.amount` (the `i64` minor unit amount from `RouterData`) directly to its `BamboraRouterData::try_from`. The conversion to `f64` major units happens inside the reference `BamboraRouterData::try_from` using `utils::get_amount_as_f64`.
3.  **Response Handling**: Reference handles the `BamboraResponse` enum for 3DS.

#### Lessons Learned:
*   The `/v1/` prefix in API paths is common and should be included.
*   The reference's way of passing raw amount and currency unit to `BamboraRouterData::try_from` is cleaner, assuming `BamboraRouterData`'s transformer correctly handles it. The discrepancy in `get_currency_unit` (Base vs. Minor) is still a point of concern for correct amount conversion if `req.request.amount` is minor.
*   Full 3DS support requires handling the `BamboraResponse` enum.

---

### Flow: `CompleteAuthorize` (3DS Continue)

#### My Implementation:
Not implemented.

#### Reference Implementation:
Fully implemented.
*   `get_url()`: Constructs URL like `base_url/v1/payments/{three_d_session_data}/continue`.
*   `get_request_body()`: Uses `BamboraThreedsContinueRequest` from transformers.
*   `handle_response()`: Parses into `bambora::BamboraPaymentsResponse`.

#### Lessons Learned:
*   Proper 3DS flow requires a `CompleteAuthorize` implementation.
*   Connector metadata (`req.request.connector_meta`) is used to pass `three_d_session_data` from Authorize to CompleteAuthorize.

---

### Flow: `Capture`

#### My Implementation `get_url()`:
```rust
Ok(format!(
    "{}/payments/{}/complete", // Endpoint from my initial doc reading
    self.base_url(connectors),
    connector_payment_id
))
```
#### Reference Implementation `get_url()`:
```rust
Ok(format!(
    "{}/v1/payments/{}/completions", // /v1/ and /completions (plural)
    self.base_url(connectors),
    req.request.connector_transaction_id,
))
```
#### My Implementation `get_request_body()`:
Uses `BamboraCaptureRequest { amount: f64 }`.
#### Reference Implementation `get_request_body()`:
Uses `BamboraPaymentsCaptureRequest { amount: f64, payment_method: PaymentMethod::Card }`.

#### Differences:
1.  **URL Path**: Reference uses `/v1/` and `/completions` (plural). Mine used `/complete`.
2.  **Request Body**: Reference `BamboraPaymentsCaptureRequest` also includes `payment_method`.

#### Lessons Learned:
*   API endpoint paths need to be exact (`/v1/` and pluralization).
*   Capture request might also need `payment_method` field.

---

### Flow: `Void` (PaymentsCancel)

#### My Implementation:
Not implemented (empty struct `impl ConnectorIntegration<Void, ...> for Bambora {}`).

#### Reference Implementation:
Fully implemented.
*   `get_url()`: `base_url/v1/payments/{connector_payment_id}/void`.
*   `get_request_body()`: Uses `BamboraVoidRequest { amount: f64 }` from transformers.
*   `handle_response()`: Parses into `bambora::BamboraPaymentsResponse`.

#### Lessons Learned:
*   Bambora has a dedicated `/void` endpoint.
*   Void request also takes an `amount`.

---

### Flow: `RefundExecute`

#### My Implementation `get_url()`:
```rust
Ok(format!(
    "{}/payments/{}/returns",
    self.base_url(connectors),
    req.request.connector_transaction_id
))
```
#### Reference Implementation `get_url()`:
```rust
Ok(format!(
    "{}/v1/payments/{}/returns", // Includes /v1/
    self.base_url(connectors),
    connector_payment_id,
))
```
#### Differences:
1.  **URL Path**: Reference includes `/v1/`.

#### Lessons Learned:
*   Consistency with `/v1/` prefix.

---
### Flow: `PaymentMethodToken`

#### My Implementation:
Fully implemented using `/tokens` endpoint and `BamboraTokenizationRequest`/`Response`.

#### Reference Implementation:
Marked as `// Not Implemented (R)`.

#### Differences & Lessons Learned:
*   My implementation of the dedicated tokenization flow seems valid based on the API docs for `/tokens`. The reference codebase might handle tokenization differently or not prioritize this specific flow integration.

---

### `ConnectorSpecifications`

#### My Implementation:
```rust
// Manually constructed Vecs and Option<PaymentMethodDetails>
// ...
is_webhook_source_verification_supported: Some(true), // MD5 hash
```

#### Reference Implementation:
Uses `lazy_static!` to define `BAMBORA_SUPPORTED_PAYMENT_METHODS`, `BAMBORA_CONNECTOR_INFO`, etc.
More detailed and structured, using `SupportedPaymentMethodsExt` and `PaymentMethodDetails` with `FeatureStatus`.
```rust
// ...
fn get_supported_webhook_flows(&self) -> Option<&'static [enums::EventClass]> {
    Some(&*BAMBORA_SUPPORTED_WEBHOOK_FLOWS) // Empty Vec
}
// No explicit is_webhook_source_verification_supported in the static struct.
```

#### Differences:
1.  **Structure**: Reference uses `lazy_static` for cleaner definitions.
2.  **Detail**: Reference is more granular with `FeatureStatus`.
3.  **Webhook Verification**: My `get_payment_method_details` included `is_webhook_source_verification_supported`. Reference `ConnectorSpecifications` doesn't directly expose this boolean but has `get_supported_webhook_flows` (which is empty).

#### Lessons Learned:
*   Using `lazy_static` for static connector specification data is a good pattern.
*   The `FeatureStatus` enum provides a more structured way to define capabilities.

---

### General Observations:
*   The reference code is more robust in handling optional fields from `RouterData` (e.g., `get_optional_billing_country`).
*   The reference code consistently uses `/v1/` in API paths.
*   Error handling and context messages in `change_context()` are more specific in the reference code.

This comparison highlights several areas where my initial implementation can be improved to align with the more complete and robust patterns in the reference codebase, especially concerning 3DS, detailed API field mapping, and authentication.


## Common Pitfalls and Lessons Learned

Based on our experience with connector integrations (such as HiPay), here are key pitfalls to watch out for:

### 1. Authentication Mechanism Issues

- **Verify the auth type carefully**: Some connectors use `HeaderKey`, others use `BodyKey`, and some require complex multi-step authentication. Read the API docs thoroughly.
- **Check for Base64 encoding requirements**: Many Basic Auth implementations require Base64 encoding of credentials.
- **Use the correct credential handling**: For sensitive data, leverage `PeekInterface` rather than `expose()` when possible.

### 2. Amount Handling Variations

- **Major vs Minor Units**: Different payment processors expect amounts in different formats:
  - Major units (like 10.99 for dollars/euros) using `StringMajorUnit`
  - Minor units (like 1099 for cents) using `StringMinorUnit`
- Always check the API documentation carefully for amount format requirements.
- Verify by examining sample requests in the payment processor's documentation.

### 3. Request Format Requirements

- Some connectors expect `application/x-www-form-urlencoded` while others require `multipart/form-data`
<|> add supported types
- For FormData requests, you may need a custom serialization helper function
- Check content type requirements for each endpoint as they may vary within the same connector docs 

### 4. URL Construction Patterns

- Many connectors use different base URLs for different services (e.g., tokenization vs payment processing)
- Some connectors require path parameters, others use query parameters
- Ensure proper URL construction by testing each endpoint's format separately

### 5. Status Mapping Complexity

- Payment processors often use numeric or custom string codes for statuses
- Create comprehensive status mapping that covers all possible states
- Pay special attention to pending, partial success, and error states
- Document status mapping clearly for future reference

### 6. Structure Definition Completeness

- Ensure all required fields are included in request/response structures
- Handle optional fields appropriately with `Option<T>` and proper serde attributes
- Use `#[serde(rename = "field_name")]` when field names don't match Rust naming conventions
- Use `#[serde(skip_serializing_if = "Option::is_none")]` for optional fields

### 7. Testing All Flows

- Test the entire payment lifecycle: authorization, capture, refund, void
- Test both successful and error scenarios
- Verify 3DS flows if the connector supports them
- Test synchronization endpoints separately

By keeping these lessons in mind, you can avoid common pitfalls and accelerate connector integrations.

## Spreedly Integration Learnings (Session - 2025-05-26)

### Authentication
- **HTTP Basic Auth Pattern**: Spreedly uses standard HTTP Basic Auth with environment key as username and access secret as password.
  ```rust
  let auth_string = format!("{}:{}", auth.environment_key.expose(), auth.access_secret.expose());
  let encoded_auth = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, auth_string.as_bytes());
  Ok(vec![(headers::AUTHORIZATION.to_string(), format!("Basic {}", encoded_auth).into_masked())])
  ```
- **Key Learning**: Use the standard base64 engine for encoding. Format header as `Basic <encoded_credentials>`.

### Gateway Token Management
- **Connector Metadata Usage**: Spreedly requires a gateway token for routing transactions to the appropriate processor.
- **Extraction Pattern**:
  ```rust
  let gateway_token = req.connector_meta_data.as_ref()
      .and_then(|meta| meta.peek().as_object())
      .and_then(|obj| obj.get("gateway_token"))
      .and_then(|token| token.as_str())
      .ok_or(errors::ConnectorError::MissingRequiredField { field_name: "gateway_token in connector_meta_data" })?;
  ```
- **Key Learning**: Use `connector_meta_data` for merchant-specific configuration beyond standard auth credentials.

### Transaction Token Flow
- **Token Management**: Spreedly returns transaction tokens that must be tracked for subsequent operations:
  - Authorize → Returns `transaction.token`
  - Capture → Uses token in URL: `/v1/transactions/{transaction_token}/capture.json`
  - Refund → Uses token in URL: `/v1/transactions/{transaction_token}/credit.json`
  - Sync → Uses token in URL: `/v1/transactions/{transaction_token}.json`
- **Key Learning**: Store connector-specific transaction identifiers as `ConnectorTransactionId` and use them consistently in subsequent API calls.

### Request/Response Simplicity
- **Flat Structure Preference**: Unlike some connectors with deeply nested structures, Spreedly uses relatively flat request/response formats.
- **Example**:
  ```rust
  pub struct SpreedlyPaymentsRequest {
      transaction: SpreedlyTransaction,
  }
  pub struct SpreedlyTransaction {
      credit_card: SpreedlyCreditCard,
      amount: StringMinorUnit,
      currency_code: String,
  }
  ```
- **Key Learning**: Don't over-engineer. Keep structures as simple as the API allows.

### Name Splitting Requirements
- **Cardholder Name Parsing**: Spreedly requires separate first/last name fields.
- **Implementation Pattern**:
  ```rust
  first_name: name.peek().split_whitespace().collect::<Vec<_>>().first().map(|s| Secret::new(s.to_string())).unwrap_or_else(|| Secret::new("".to_string())),
  last_name: name.peek().split_whitespace().collect::<Vec<_>>().get(1..).map(|parts| Secret::new(parts.join(" "))).unwrap_or_else(|| Secret::new("".to_string())),
  ```
- **Key Learning**: Handle edge cases (single names, multiple spaces) gracefully with sensible defaults.

### Webhook Verification
- **HMAC-SHA256 Pattern**: Spreedly uses straightforward HMAC-SHA256 verification.
- **Implementation**:
  ```rust
  let expected_signature = crypto::HmacSha256::sign_message(
      webhook_secret.expose().as_bytes(),
      request.body.as_bytes(),
  ).change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;
  let expected_signature_str = hex::encode(expected_signature);
  ```
- **Key Learning**: Most webhook verifications involve computing HMAC over raw request body with shared secret and comparing against header value.

### Status Mapping Strategy
- **Multi-field Status Determination**: Spreedly uses transaction type + success flag rather than single status field.
- **Pattern**:
  ```rust
  match transaction_type.as_str() {
      "Authorize" => if succeeded { Authorized } else { Failure }
      "Capture" => if succeeded { Charged } else { CaptureFailed }
      _ => if succeeded { Pending } else { Failure }
  }
  ```
- **Key Learning**: Status mapping may require examining multiple response fields.

### Common Pitfalls Avoided

1. **Type Alias Issues**: 
   - Problem: `type SpreedlySyncResponse = SpreedlyPaymentsResponse` caused trait implementation conflicts.
   - Solution: Keep type aliases but don't implement duplicate traits for them.

2. **Import Management in Tests**:
   - Required imports: `cards`, `common_utils`, `api_models`, `std::str::FromStr`
   - Key Learning: Test files often need additional imports for type conversions.

3. **Optional Field Handling**:
   - Spreedly's transaction token is required for some operations but generic type system treats it as optional.
   - Solution: Proper error handling with meaningful error messages.

4. **Test Implementation Requirements**:
   - Mock gateway token in `connector_meta_data`
   - Test card: 4111111111111111
   - Complete address/email data for coverage
   - Some negative test expectations may not match actual behavior

### Architecture Insights

- **Separation of Concerns**: Clean separation between main logic (spreedly.rs) and data transformation (transformers.rs)
- **Minimal Boilerplate**: Standard operations require minimal custom code
- **Reusable Patterns**: Basic Auth and HMAC webhook verification patterns can be reused for similar connectors
- **Clear Token Management**: Transaction token pattern is clear and consistent

### Summary

The Spreedly integration demonstrates that effective connector implementations don't always require complex architectures. Key to success:
- Match the simplicity of the API with simple code structures
- Use established patterns (Basic Auth, HMAC verification) without reinventing
- Focus on clear error messages and proper error handling
- Test thoroughly with realistic data

This implementation serves as a good template for other payment orchestration platforms with similar authentication patterns and API structures.
