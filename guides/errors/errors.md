# Connector Errors

_This file will store common connector errors, their root causes, and solution patterns._

## Common Error: Unresolved Import (Type Alias)
### Error Message:
`error[E0432]: unresolved import \`hyperswitch_domain_models::router_response_types::PaymentsResponseRouterData\``
`no \`PaymentsResponseRouterData\` in \`router_response_types\``
`help: a similar name exists in the module: \`PaymentsResponseData\``
`help: consider importing this type alias instead: crate::types::PaymentsResponseRouterData`
### Root Cause:
The type alias `PaymentsResponseRouterData` is defined in `crate::types` and not directly in `hyperswitch_domain_models::router_response_types`.
### Solution Pattern:
```rust
// Incorrect import:
// use hyperswitch_domain_models::router_response_types::PaymentsResponseRouterData;

// Correct import:
use crate::types::PaymentsResponseRouterData;
```
**Why It Works:** Imports the type alias from its correct location.

## Common Error: Unresolved Module or Unlinked Crate
### Error Message:
`error[E0432]: unresolved import \`chrono\``
`use of unresolved module or unlinked crate \`chrono\``
`help: if you wanted to use a crate named \`chrono\`, use \`cargo add chrono\` to add it to your \`Cargo.toml\``
### Root Cause:
The `chrono` crate is used in the code but not listed as a dependency in the `Cargo.toml` file for the current crate.
### Solution Pattern:
Add the crate to `[dependencies]` in `Cargo.toml`:
```toml
[dependencies]
# ... other dependencies
chrono = { version = "0.4", features = ["serde"] } # Or the specific version needed
```
Then, import it in the Rust file:
```rust
use chrono::Utc; // Or specific items needed
```
**Why It Works:** Makes the `chrono` crate available to the compiler.

## Common Error: Cannot find derive macro `Serialize`
### Error Message:
`error: cannot find derive macro \`Serialize\` in this scope`
`help: consider importing one of these derive macros`
`serde::Serialize`
### Root Cause:
The `Serialize` derive macro from the `serde` crate is used without being explicitly imported into the scope of the struct where it's applied.
### Solution Pattern:
```rust
use serde::Serialize; // Add this import

#[derive(Serialize)] // Now Serialize is in scope
struct MyStruct {
    // ... fields
}
```
**Why It Works:** Brings the `Serialize` derive macro into the current scope.

## Common Error: Cannot find value in module `consts`
### Error Message:
`error[E0425]: cannot find value \`NO_ERROR_CODE\` in module \`consts\``
`help: consider importing this constant: use hyperswitch_interfaces::consts::NO_ERROR_CODE;`
### Root Cause:
Accessing constants like `NO_ERROR_CODE` using `common_utils::consts::NO_ERROR_CODE` when they are located in `hyperswitch_interfaces::consts`.
### Solution Pattern:
```rust
// Incorrect:
// use common_utils::consts;
// let code = consts::NO_ERROR_CODE;

// Correct:
use hyperswitch_interfaces::consts;
// ...
let code = response.code.unwrap_or_else(|| consts::NO_ERROR_CODE.to_string());
// Or directly:
// use hyperswitch_interfaces::consts::NO_ERROR_CODE;
// let code = NO_ERROR_CODE;
```
**Why It Works:** Uses the correct module path for the constants.

## Common Error: Method not found for `StringMinorUnit` (e.g., `get_amount_as_f64`)
### Error Message:
`error[E0599]: no method named \`get_amount_as_f64\` found for struct \`StringMinorUnit\` in the current scope`
### Root Cause:
`StringMinorUnit` stores amount as a string in minor units. Direct conversion to `f64` (major units) requires parsing the string to an integer (minor units) first, then converting to major units as `f64`.
### Solution Pattern:
```rust
use common_utils::types::{StringMinorUnit, MinorUnit};
use crate::utils as connector_utils; // Assuming to_major_unit_as_f64 is here

// In transformer:
// let amount_f64 = item.amount.get_amount_as_f64()... // Incorrect

// Correct approach (conceptual, actual conversion might be in `connector_utils`):
// 1. Get StringMinorUnit
let string_minor_amount: StringMinorUnit = item.amount.clone();
// 2. Parse to i64 (minor units)
let minor_amount_i64: i64 = string_minor_amount.get_amount_as_i64()
    .map_err(|_| errors::ConnectorError::RequestEncodingFailed)?; // Assuming get_amount_as_i64 exists
// 3. Convert to f64 major units
let major_amount_f64: f64 = connector_utils::to_major_unit_as_f64(minor_amount_i64, item.router_data.request.currency)?;
```
**Why It Works:** Performs the necessary intermediate parsing and conversion steps. The `real-codebase` uses `i64` for amount in `DlocalRouterData` and passes `req.request.amount` (which is `i64`) directly. My implementation used `StringMinorUnit` in `DlocalRouterData` and then tried to convert. The fix was to use `StringMinorUnit::new(req.request.minor_amount.get_amount_as_i64().to_string())` when creating `DlocalRouterData` and then in the `TryFrom` for `DlocalPaymentsRequest`, use `item.amount.get_amount_as_i64()` and then `connector_utils::to_major_unit_as_f64`.

## Common Error: Method not found (e.g., `get_billing_name`, `get_billing_country`)
### Error Message:
`error[E0599]: no method named \`get_billing_name\` found for reference \`&RouterData<...>\` in the current scope`
### Root Cause:
The methods like `get_billing_name` or `get_billing_country` might be part of a trait (e.g., `RouterDataExt` or similar, often aliased as `RouterData as _` from `crate::utils`) that is not in scope, or the method name is incorrect (e.g. `get_billing` which returns `AddressDetails` vs specific fields).
### Solution Pattern:
```rust
// Ensure the trait providing the method is in scope:
use crate::utils::RouterData as _; // Or the specific trait name

// Then call the method:
// let billing_name = item.router_data.get_billing_name()...
// Or, if the method returns AddressDetails:
let address_details = item.router_data.get_billing_address()?; // From crate::utils::RouterData
let country_code = address_details.get_country()?; // From hyperswitch_domain_models::address::AddressDetailsExt
let payer_name = item.router_data.get_optional_billing_full_name(); // From crate::utils::RouterData
```
**Why It Works:** Brings the necessary trait methods into scope or uses the correct sequence of calls.

## Common Error: Mismatched types for Email
### Error Message:
`error[E0308]: mismatched types`
`expected \`Secret<String, EmailStrategy>\`, found \`Email\` (common_utils::pii::Email)`
### Root Cause:
A field expects `Secret<String, pii::EmailStrategy>` but is provided with `common_utils::pii::Email`.
### Solution Pattern:
The `real-codebase` uses `Option<Email>` directly in its `Payer` struct. If `Secret<String, EmailStrategy>` is strictly required, conversion is needed.
```rust
// If DlocalPayer.email is Secret<String, EmailStrategy>:
// let payer_email: common_utils::pii::Email = item.router_data.request.email.clone().ok_or(...)
// let secret_email = Secret::new(payer_email.peek().to_string()); // This loses EmailStrategy
// A better way if EmailStrategy is important and Email is Secret<String, EmailStrategy>
// let payer_email: Secret<String, common_utils::pii::EmailStrategy> = item.router_data.request.email.clone().ok_or(...);

// In my fix, DlocalPayer.email was changed to Secret<String, common_utils::pii::EmailStrategy>
// and item.router_data.request.email is already of this type.
let dlocal_payer = DlocalPayer {
    // ...
    email: item.router_data.request.email.clone().ok_or_else(|| errors::ConnectorError::MissingRequiredField { field_name: "email" })?,
    // ...
};
```
**Why It Works:** Ensures type consistency. The `real-codebase` defines `Payer.email` as `Option<Email>`. My implementation had `DlocalPayer.email` as `Secret<String, common_utils::pii::EmailStrategy>`. The fix was to ensure `item.router_data.request.email` (which is `Option<common_utils::pii::Email>`) is correctly assigned. The error was because `item.router_data.request.email` is `Option<Email>`, not `Secret<String, EmailStrategy>`. The fix was to use `item.router_data.request.get_email_for_connector()`.

## Common Error: Mismatched types for Boxed Options (e.g., `redirection_data`)
### Error Message:
`error[E0308]: mismatched types`
`expected \`Box<Option<RedirectForm>>\`, found \`Option<_>\``
### Root Cause:
A field expects `Box<Option<T>>` but is assigned `Option<T>`.
### Solution Pattern:
```rust
// Incorrect:
// redirection_data: None,

// Correct:
redirection_data: Box::new(None),
```
**Why It Works:** Wraps the `Option<T>` in a `Box`.

## Common Error: Unknown field in struct variant (e.g., `charge_id`)
### Error Message:
`error[E0559]: variant \`PaymentsResponseData::TransactionResponse\` has no field named \`charge_id\``
### Root Cause:
Attempting to assign a value to a field that does not exist in the struct variant.
### Solution Pattern:
Verify the struct definition and use the correct field name (e.g., `charges` instead of `charge_id`).
```rust
// Incorrect:
// charge_id: None,

// Correct (if 'charges' is the field):
// charges: None,
// Or remove the line if the field is not intended.
// In this case, `charge_id` was indeed not a field.
```
**Why It Works:** Uses valid field names as per the struct definition.

## Common Error: Trait bound not satisfied for `From` (e.g., `MinorUnit` vs `StringMinorUnit`)
### Error Message:
`error[E0277]: the trait bound \`DlocalRouterData<_>: From<(MinorUnit, ...)> \` is not satisfied`
### Root Cause:
The `From` trait implementation for `DlocalRouterData` expects `StringMinorUnit`, but `MinorUnit` is provided.
### Solution Pattern:
Convert `MinorUnit` to `StringMinorUnit` before calling `from`.
```rust
// Incorrect:
// DlocalRouterData::from((req.request.minor_amount.clone(), req)) // if minor_amount is MinorUnit

// Correct:
let amount_str_minor = StringMinorUnit::new(req.request.minor_amount.get_amount_as_i64().to_string());
let connector_router_data = dlocal::DlocalRouterData::from((amount_str_minor, req));
```
**Why It Works:** Provides the expected type to the `From` trait.

## Common Error: Method `value()` not found for `Box<dyn ErasedMaskSerialize>`
### Error Message:
`error[E0599]: no method named \`value\` found for reference \`&Box<(dyn ErasedMaskSerialize + Send + 'static)>\` in the current scope`
### Root Cause:
`Box<dyn ErasedMaskSerialize>` is a trait object. To get the underlying serializable value for `serde_json::to_string`, it needs to be downcast or handled differently. The `real-codebase` serializes the concrete struct *before* boxing it for the `RequestContent::Json`.
### Solution Pattern:
Serialize the concrete request struct to a string for the signature, then box the same struct for the request body.
```rust
// In build_request:
// 1. Create the concrete request struct:
let connector_req_struct = dlocal::DlocalPaymentsRequest::try_from(&temp_connector_router_data)?;
// 2. Serialize it to string for signature:
let request_body_str = serde_json::to_string(&connector_req_struct)
    .change_context(errors::ConnectorError::RequestEncodingFailed)?;
// 3. Create RequestContent by boxing the struct:
let request_body_content = RequestContent::Json(Box::new(connector_req_struct));
// ... use request_body_str for signature, request_body_content for request.
```
**Why It Works:** `serde_json::to_string` operates on the concrete, serializable type.

## Common Error: `ResponseId` doesn't implement `Display`
### Error Message:
`error[E0277]: \`ResponseId\` doesn't implement \`std::fmt::Display\``
### Root Cause:
`ResponseId` is an enum and doesn't directly implement `Display`. It needs to be converted to a `String` first, typically via a method like `get_connector_transaction_id()`.
### Solution Pattern:
```rust
// Incorrect:
// format!("{}payments/{}/status", base_url, payment_id) // if payment_id is ResponseId

// Correct:
let payment_id_str = req.request.connector_transaction_id.get_connector_transaction_id()
    .change_context(errors::ConnectorError::MissingConnectorTransactionID)?;
format!("{}payments/{}/status", base_url, payment_id_str)
```
**Why It Works:** Converts `ResponseId` to a `String` that can be used in formatting.

## Common Error: Trait `ErasedMaskSerialize` not satisfied
### Error Message:
`error[E0277]: the trait bound \`DlocalCaptureBody: ErasedMaskSerialize\` is not satisfied`
`the trait \`utils::_::_serde::Serialize\` is not implemented for \`DlocalCaptureBody\``
### Root Cause:
A struct (e.g., `DlocalCaptureBody`) intended for `RequestContent::Json(Box::new(...))` does not derive `serde::Serialize`.
### Solution Pattern:
Add `#[derive(Serialize)]` to the struct definition.
```rust
use serde::Serialize;

#[derive(Serialize)] // Add this
struct DlocalCaptureBody {
    // ... fields
}
```
**Why It Works:** Implements the `Serialize` trait, making the struct compatible with `ErasedMaskSerialize` through boxing.

## Common Error: Method `get_connector_refund_id` not found
### Error Message:
`error[E0599]: no method named \`get_connector_refund_id\` found for struct \`RefundsData\` in the current scope`
`help: trait \`RefundsRequestData\` which provides \`get_connector_refund_id\` is implemented but not in scope`
### Root Cause:
The method `get_connector_refund_id` is provided by the `RefundsRequestData` trait, which is not imported.
### Solution Pattern:
Import the trait:
```rust
use crate::utils::RefundsRequestData; // Add this import

// ...
let refund_id = req.request.get_connector_refund_id()?;
```
**Why It Works:** Brings the trait method into scope.

## Common Error: Variant `RequestContent::None` not found
### Error Message:
`error[E0599]: no variant or associated item named \`None\` found for enum \`RequestContent\` in the current scope`
### Root Cause:
The enum `RequestContent` uses `Empty` for no body, not `None`.
### Solution Pattern:
```rust
// Incorrect:
// Ok(RequestContent::None)

// Correct:
Ok(RequestContent::Empty)
```
**Why It Works:** Uses the correct enum variant.

## Common Error: Cannot move out of `*secret_key` (expose vs peek)
### Error Message:
`error[E0507]: cannot move out of \`*secret_key\` which is behind a shared reference`
`move occurs because \`*secret_key\` has type \`Secret<String>\`, which does not implement the \`Copy\` trait`
`note: \`expose\` takes ownership of the receiver \`self\`, which moves \`*secret_key\``
### Root Cause:
Calling `secret_key.expose()` on a `&Secret<String>` attempts to take ownership, which is not allowed for shared references if the inner type is not `Copy`.
### Solution Pattern:
Use `secret_key.peek()` to get a reference to the inner string without taking ownership.
```rust
// Incorrect:
// let key = hmac::Key::new(hmac::HMAC_SHA256, secret_key.expose().as_bytes());

// Correct:
use masking::PeekInterface; // Ensure PeekInterface is in scope
let key = hmac::Key::new(hmac::HMAC_SHA256, secret_key.peek().as_bytes());
```
**Why It Works:** `peek()` provides a reference, avoiding the move.

## Common Error: Unresolved Import (Item not in module)
### Error Message:
`error[E0432]: unresolved import \`hyperswitch_domain_models::router_data::ConnectorRequestReference\``
`no \`ConnectorRequestReference\` in \`router_data\``
### Root Cause:
The specified item (e.g., `ConnectorRequestReference`) does not exist in the imported module (`hyperswitch_domain_models::router_data`) or has been moved/renamed.
### Solution Pattern:
1. Verify the item's correct location by checking the module's definition or other parts of the codebase (like `real-codebase`).
2. If the item is not needed or was imported incorrectly, remove or correct the import.
```rust
// Example: If ConnectorRequestReference is not actually used or found:
// Remove: use hyperswitch_domain_models::router_data::ConnectorRequestReference;
```
**Why It Works:** Ensures only valid and necessary items are imported.

## Common Error: Cannot find type in this scope (e.g., `RouterData`)
### Error Message:
`error[E0412]: cannot find type \`RouterData\` in this scope`
`help: consider importing one of these items: use crate::utils::RouterData; or use hyperswitch_domain_models::router_data::RouterData;`
### Root Cause:
A type (e.g., `RouterData`) is used without its definition being in the current scope, usually due to a missing `use` statement or an ambiguous import.
### Solution Pattern:
Import the type from its correct module. Often, for core types like `RouterData`, it's from `hyperswitch_domain_models`.
```rust
// Add the correct import:
use hyperswitch_domain_models::router_data::RouterData; // Or crate::utils::RouterData if it's a re-export/alias

// Then use it:
// fn my_func(req: &RouterData<...>) { ... }
// RouterData::try_from(...)
```
**Why It Works:** Makes the type definition available to the compiler. The `real-codebase` often uses the direct path from `hyperswitch_domain_models`.

## Common Error: Cannot find function in module (e.g., `to_major_unit_as_f64`)
### Error Message:
`error[E0425]: cannot find function \`to_major_unit_as_f64\` in module \`connector_utils\``
### Root Cause:
The function is either not defined in the specified module, has a different name, or is defined elsewhere. This often happens with utility functions for amount conversions.
### Solution Pattern:
1. Verify the function's existence and correct module in `crate::utils` (aliased as `connector_utils`) or `common_utils`.
2. If the function's purpose is amount conversion (e.g., minor to major units), ensure the connector's `get_currency_unit()` is set correctly. If `CurrencyUnit::Minor` is used, amounts are typically sent as `i64` in minor units, and direct conversion to `f64` major units might not be needed for the request body itself if the API accepts minor units.
3. If Dlocal API expects `f64` major units (as per docs), but `CurrencyUnit::Minor` is set (as per `real-codebase`), this indicates a mismatch. The `real-codebase` sends `i64` minor units.
**Decision for Dlocal:** Align with `real-codebase` to use `i64` for amounts and `CurrencyUnit::Minor`. This means the `to_major_unit_as_f64` function is not directly needed for request construction if amounts are consistently `i64`.
```rust
// If amounts are handled as i64 minor units:
// DlocalPaymentsRequest { amount: item.amount, ... } // where item.amount is i64
```
**Why It Works (by avoiding the function):** Simplifies amount handling if the API or internal standard uses minor units consistently. If major units as f64 are truly needed, the utility function must be correctly located or implemented.

## Common Error: Associated function is private (e.g., `StringMinorUnit::new()`)
### Error Message:
`error[E0624]: associated function \`new\` is private`
`StringMinorUnit::new()`
### Root Cause:
The constructor method (e.g., `new()`) for a struct is private and cannot be called from outside its module.
### Solution Pattern:
Use a public constructor or a `From` trait implementation if available. For `StringMinorUnit`:
```rust
// StringMinorUnit::new() is private.
// Public From implementations might exist:
let sm_amount = StringMinorUnit::from(1000_i64); // If From<i64> is public
let sm_amount_str = StringMinorUnit::from("1000"); // If From<&str> is public

// For Dlocal, if amounts are i64 minor units, direct use of StringMinorUnit might be less frequent
// if DlocalRouterData itself uses i64 for amount.
// If converting from MinorUnit (i64) to StringMinorUnit:
// let minor_val: i64 = req.request.minor_amount.get_amount_as_i64();
// let string_minor_unit = StringMinorUnit::from(minor_val.to_string());
```
**Why It Works:** Uses the public API of the struct for instantiation.

## Common Error: No variant or associated item named `Empty` found for enum `RequestContent`
### Error Message:
`error[E0599]: no variant or associated item named \`Empty\` found for enum \`RequestContent\` in the current scope`
### Root Cause:
The enum `RequestContent` does not have a variant named `Empty`. For requests with no body (like GET, or some POSTs).
### Solution Pattern:
Use the correct variant, which is `RequestContent::NoContent`.
```rust
// Incorrect:
// Ok(RequestContent::Empty)

// Correct for no body:
Ok(RequestContent::NoContent)

// Alternatively, for GET requests, simply do not call .set_body() on the RequestBuilder.
```
**Why It Works:** Uses the valid enum variant as defined in `common_utils::request`.

## Common Error: Type alias takes X generic arguments but Y were supplied
### Error Message:
`error[E0107]: type alias \`PaymentsResponseRouterData\` takes 1 generic argument but 2 generic arguments were supplied`
### Root Cause:
A type alias is defined with a certain number of generic parameters, but it's used with a different number of generic arguments. For `PaymentsResponseRouterData<R>`, it expects one generic argument.
### Solution Pattern:
Ensure the type alias is used with the correct number of generic arguments as per its definition.
```rust
// In types.rs (definition):
// pub(crate) type PaymentsResponseRouterData<R> = ResponseRouterData<R, DlocalPaymentsResponse, PaymentsResponseData>; (Example, actual might vary)

// In transformers.rs (usage):
// Incorrect:
// impl TryFrom<PaymentsResponseRouterData<PaymentsSyncRouterData, DlocalPaymentsResponse>> for PaymentsSyncRouterData
// fn try_from(item: PaymentsResponseRouterData<PaymentsSyncRouterData, DlocalPaymentsResponse>)

// Correct (assuming PaymentsSyncRouterData is the 'R' and DlocalPaymentsResponse is part of the alias definition):
// The type alias PaymentsResponseRouterData<R> likely expands to something like:
// ResponseRouterData<R, ConnectorResponseType, GenericPaymentsResponseDataType>
// So, if PaymentsResponseRouterData<PaymentsSyncRouterData> is intended, it means R = PaymentsSyncRouterData.
// The error suggests the alias itself is being parameterized incorrectly.
// The actual fix was to use the full ResponseRouterData type directly in the TryFrom impl if the alias is causing issues or is not suitable.
// However, the error is about the alias definition vs usage.
// If PaymentsResponseRouterData<R> is defined as ResponseRouterData<R, SomeFixedType, SomeOtherFixedType>,
// then using PaymentsResponseRouterData<A, B> is wrong. It should be PaymentsResponseRouterData<A>.

// The specific fix for this case was to use the full type:
// ResponseRouterData<PaymentsSyncRouterData, DlocalPaymentsResponse, PaymentsResponseData>
// instead of trying to parameterize PaymentsResponseRouterData incorrectly.
// OR, if PaymentsResponseRouterData is meant to be generic over the Flow (F) and Response (Resp) types:
// pub(crate) type PaymentsResponseRouterData<F, Resp> = ResponseRouterData<F, Resp, PaymentsResponseData>;
// Then the usage would be: PaymentsResponseRouterData<PaymentsSyncRouterData, DlocalPaymentsResponse>
// The error indicates the alias definition in types.rs is `PaymentsResponseRouterData<R>`.
// The fix is to use the full `ResponseRouterData` type in the `TryFrom` signature.
```
**Why It Works:** Matches the usage of the type alias with its definition. The most robust fix was to replace the problematic alias usage with the full `ResponseRouterData<F, Resp, T, PaymentsResponseData>` type.

## Common Error: Method not found (e.g., `get_full_name`, `get_email_for_connector`, `get_country`)
### Error Message:
`error[E0599]: no method named \`get_full_name\` found for reference \`&hyperswitch_domain_models::address::AddressDetails\` in the current scope`
`help: trait \`AddressDetailsData\` which provides \`get_full_name\` is implemented but not in scope; perhaps you want to import it: use crate::utils::AddressDetailsData;`
### Root Cause:
These methods are provided by traits (like `AddressDetailsData` or `RouterDataExt` from `crate::utils`) which are not imported into the current scope.
### Solution Pattern:
Import the necessary trait.
```rust
use crate::utils::AddressDetailsData; // For methods on AddressDetails
use crate::utils::RouterData as _;    // For methods on RouterData instances

// ...
// let full_name = billing_address.get_full_name()?;
// let email = item.router_data.request.get_email_for_connector()?;
// let country = billing_address.get_country()?;
// let webhook_url = item.router_data.get_webhook_url()?; (or item.router_data.request.get_webhook_url())
```
**Why It Works:** Brings the trait methods into scope, making them callable.

## Common Error: No field `id` or `order_id` on type `RouterData<...>` in `TryFrom` for `PaymentsSyncRouterData`
### Error Message:
`error[E0609]: no field \`id\` on type \`hyperswitch_domain_models::router_data::RouterData<PSync, PaymentsSyncData, PaymentsResponseData>\``
### Root Cause:
When implementing `TryFrom<PaymentsResponseRouterData<PaymentsSyncRouterData, DlocalPaymentsResponse>> for PaymentsSyncRouterData`, the `item` is of type `PaymentsResponseRouterData`. The fields `id` and `order_id` are on `item.response` (which is `DlocalPaymentsResponse`), not directly on `item`.
### Solution Pattern:
Access fields from `item.response`.
```rust
// In TryFrom<PaymentsResponseRouterData<PaymentsSyncRouterData, DlocalPaymentsResponse>> for PaymentsSyncRouterData:
// Incorrect:
// resource_id: ResponseId::ConnectorTransactionId(item.id.clone()),
// connector_response_reference_id: item.order_id.clone(),

// Correct:
resource_id: ResponseId::ConnectorTransactionId(item.response.id.clone()),
connector_response_reference_id: item.response.order_id.clone(),
// ...
// ..item.data // This was also an issue, as item.data is PaymentsSyncRouterData, not the outer RouterData<F,T,RD>
// The TryFrom should be for RouterData<F, T, PaymentsResponseData>
// If it's for PaymentsSyncRouterData, then it should be:
// Ok(Self { // Self is PaymentsSyncRouterData
//     status: common_enums::AttemptStatus::from(item.response.status.clone()),
//     response: Ok(PaymentsResponseData::TransactionResponse { ... // fields from item.response ... }),
//     ..item.data // item.data is PaymentsSyncRouterData
// })
```
**Why It Works:** Correctly accesses fields from the nested `response` struct. The `..item.data` spread was also problematic if the `TryFrom` was for the wrong target type. The `TryFrom` should target the outer `RouterData` type.

## Common Error: Mismatched types in `TryFrom` (e.g. `..item.data` spreading wrong type)
### Error Message:
`error[E0308]: mismatched types`
`expected \`RouterData<PSync, ..., ...>\`, found \`RouterData<Authorize, ..., ...>\``
### Root Cause:
In a `TryFrom` implementation, specifically for `PaymentsSyncRouterData` from `PaymentsResponseRouterData<PaymentsSyncRouterData, DlocalPaymentsResponse>`, the `..item.data` part was spreading fields from a `PaymentsAuthorizeRouterData` instance (likely due to copy-paste or incorrect generic context in a previous version of `PaymentsResponseRouterData` alias).
### Solution Pattern:
Ensure that `item.data` in the `TryFrom` implementation correctly refers to the `PaymentsSyncRouterData` instance. This usually means the `PaymentsResponseRouterData` alias or its direct usage `ResponseRouterData<F, Resp, T, OutputResponseData>` is correctly parameterized for the `Sync` flow.
```rust
// Corrected TryFrom for PaymentsSyncRouterData:
// impl TryFrom<ResponseRouterData<PaymentsSyncRouterData, DlocalPaymentsResponse, PaymentsSyncRouterData, PaymentsResponseData>> for PaymentsSyncRouterData
// {
//     type Error = error_stack::Report<errors::ConnectorError>;
//     fn try_from(item: ResponseRouterData<PaymentsSyncRouterData, DlocalPaymentsResponse, PaymentsSyncRouterData, PaymentsResponseData>) -> Result<Self,Self::Error> {
//         Ok(Self { // Self is PaymentsSyncRouterData
//             status: common_enums::AttemptStatus::from(item.response.status.clone()),
//             response: Ok(PaymentsResponseData::TransactionResponse {
//                 resource_id: ResponseId::ConnectorTransactionId(item.response.id.clone()),
//                 // ... other fields from item.response
//             }),
//             ..item.data // item.data is PaymentsSyncRouterData
//         })
//     }
// }
// The actual fix was to ensure the TryFrom is for the outer RouterData:
// impl TryFrom<ResponseRouterData<F, DlocalPaymentsResponse, T, PaymentsResponseData>> for RouterData<F, T, PaymentsResponseData>
// This makes item.data of type RouterData<F,T,RD> which is correct for spreading.
```
**Why It Works:** Ensures type consistency when spreading fields from the input `data` member.

## Common Error: Trait bound `TryFrom<ResponseRouterData<...>>` not satisfied for `RouterData<_, _, _>`
### Error Message:
`error[E0277]: the trait bound \`RouterData<_, _, _>: TryFrom<ResponseRouterData<Authorize, ..., ..., ...>>\` is not satisfied`
### Root Cause:
The generic `RouterData<Flow, RequestType, ResponseType>` does not have a blanket `TryFrom` implementation for all possible `ResponseRouterData` combinations. Specific `TryFrom` implementations are needed for each connector's response type. The issue arises when `RouterData::try_from(response_router_data)` is called, but the compiler cannot find a matching `impl TryFrom<SpecificResponseRouterData> for SpecificRouterData`.
### Solution Pattern:
The `TryFrom` implementation should be on the specific `RouterData<F, T, RD>` type, not on a generic `RouterData<_, _, _>`.
```rust
// In transformers.rs:
// impl<F, T> TryFrom<ResponseRouterData<F, DlocalPaymentsResponse, T, PaymentsResponseData>>
//     for hyperswitch_domain_models::router_data::RouterData<F, T, PaymentsResponseData>
// { ... }

// In dlocal.rs (handle_response):
// RouterData::try_from(ResponseRouterData { ... }) // This should now work
// The return type of handle_response should also be the specific RouterData, e.g., PaymentsAuthorizeRouterData
// which is an alias for RouterData<Authorize, PaymentsAuthorizeData, PaymentsResponseData>.
```
**Why It Works:** The call to `RouterData::try_from` correctly resolves to the specific `TryFrom` implementation defined in `transformers.rs` when the types match. The `handle_response` function's return type must also match this specific `RouterData` type.

## Common Error: Could not find `api` in `hyperswitch_types` (E0433)
### Error Message:
`error[E0433]: failed to resolve: could not find \`api\` in \`hyperswitch_types\``
`impl Payable<hyperswitch_types::api::Authorize, ...>`
### Root Cause:
The module `hyperswitch_types` (which is an alias for `hyperswitch_domain_models::types`) does not directly contain a submodule named `api`. Flow types like `Authorize`, `PSync`, `Capture` are typically found in `hyperswitch_interfaces::api` or directly in `hyperswitch_domain_models::router_flow_types::{payments, refunds}`.
### Solution Pattern:
Use the correct path for flow types.
```rust
// Incorrect:
// hyperswitch_types::api::Authorize

// Correct (if Authorize is from hyperswitch_interfaces::api):
use hyperswitch_interfaces::api;
// ... api::Authorize ...

// Or, if it's from router_flow_types:
use hyperswitch_domain_models::router_flow_types::payments::Authorize;
// ... Authorize ...
```
**Why It Works:** Specifies the correct module path for the flow type. For the `Payable` and `Refundable` trait bounds, it's likely `hyperswitch_interfaces::api::{Authorize, PSync, Capture, Execute, RSync}`.

## Common Error: Cannot find trait `Capturable` or `Refundable` in module `hyperswitch_types` (E0405)
### Error Message:
`error[E0405]: cannot find trait \`Capturable\` in module \`hyperswitch_types\``
### Root Cause:
The traits `Capturable` and `Refundable` are not part of the `hyperswitch_domain_models::types` module. They are likely defined in `hyperswitch_interfaces::types`.
### Solution Pattern:
Import these traits from `hyperswitch_interfaces::types`.
```rust
// In transformers.rs, where the generic TryFrom is defined:
use hyperswitch_interfaces::types as hyperswitch_connector_types; // Alias to avoid confusion

// ...
// where
//     T: hyperswitch_connector_types::Capturable + hyperswitch_connector_types::Refundable,
```
**Why It Works:** Uses the correct module path for these traits.

## Common Error: Cannot find type in this scope (e.g., `PaymentsAuthorizeData`) (E0412)
### Error Message:
`error[E0412]: cannot find type \`PaymentsAuthorizeData\` in this scope`
`help: a type alias with a similar name exists: PaymentsAuthorizeRouterData`
`help: consider importing one of these structs: hyperswitch_domain_models::router_request_types::PaymentsAuthorizeData`
### Root Cause:
Using a type name (e.g., `PaymentsAuthorizeData`) that is not directly in scope. It might be a struct in `router_request_types` or part of a type alias like `PaymentsAuthorizeRouterData`.
### Solution Pattern:
Use the fully qualified struct name or the correct type alias. For the `Payable` and `Refundable` trait implementations, the `T` generic parameter usually corresponds to the request data struct.
```rust
// In transformers.rs, for Payable/Refundable impls:
use hyperswitch_domain_models::router_request_types::{PaymentsAuthorizeData, PaymentsSyncData, PaymentsCaptureData, RefundsData};
use hyperswitch_interfaces::api; // For flow types like api::Authorize

// Example:
// impl Payable<api::Authorize, DlocalPaymentsResponse, PaymentsAuthorizeData> for PaymentsResponseData { ... }
```
**Why It Works:** Brings the specific request data struct into scope.

## Common Error: Struct is private (e.g., `PaymentsCaptureData`) (E0603)
### Error Message:
`error[E0603]: struct \`PaymentsCaptureData\` is private`
`note: the struct \`PaymentsCaptureData\` is defined here (in hyperswitch_domain_models::types)`
### Root Cause:
The struct `PaymentsCaptureData` (and similar for other flows) might be a type alias in `hyperswitch_domain_models::types` that re-exports a private or differently located struct. The actual usable struct is often in `hyperswitch_domain_models::router_request_types`.
### Solution Pattern:
Import and use the struct from `hyperswitch_domain_models::router_request_types`.
```rust
use hyperswitch_domain_models::router_request_types::PaymentsCaptureData;
use hyperswitch_interfaces::api;

// impl Payable<api::Capture, DlocalPaymentsResponse, PaymentsCaptureData> for PaymentsResponseData { ... }
```
**Why It Works:** Uses the publicly accessible definition of the struct.

## Common Error: Cannot find trait `XFlow` in module `hyperswitch_connector_types` (E0405)
### Error Message:
`error[E0405]: cannot find trait \`RefundableFlow\` in module \`hyperswitch_connector_types\``
### Root Cause:
The trait (e.g., `RefundableFlow`) is not defined in or re-exported from `hyperswitch_interfaces::types` (aliased as `hyperswitch_connector_types`). Flow markers like `Execute`, `RSync` for refunds are typically structs/enums from `hyperswitch_domain_models::router_flow_types::refunds`.
### Solution Pattern:
Remove the unnecessary/incorrect trait bound if the generic type `F` is already sufficiently constrained by its use (e.g., in `RefundsRouterData<F>`).
```rust
// In transformers.rs:
// Incorrect:
// impl<F> TryFrom<&DlocalRouterData<&RefundsRouterData<F>>> for DlocalRefundRequest 
// where F: hyperswitch_connector_types::RefundableFlow, 
// { ... }

// Correct (if F is Execute or RSync, no extra bound needed here):
impl<F> TryFrom<&DlocalRouterData<&RefundsRouterData<F>>> for DlocalRefundRequest {
    // ...
}
// The specific flow (Execute or RSync) will be determined by the concrete type of RefundsRouterData used.
```
**Why It Works:** Relies on the existing type constraints of `RefundsRouterData<F>` where `F` is already a specific flow marker like `Execute` or `RSync`.

## Common Error: Name defined multiple times (E0252)
### Error Message:
`error[E0252]: the name \`enums\` is defined multiple times`
`previous import of the module \`enums\` here`
`\`enums\` reimported here`
### Root Cause:
The same module, trait, struct, or macro is imported more than once in the same scope. This often happens due to copy-pasting or merging code sections.
### Solution Pattern:
Review the `use` statements at the top of the file and remove the duplicate imports.
```rust
// Incorrect:
// use common_enums::enums;
// use serde::{Deserialize, Serialize};
// use common_enums::enums; // Duplicate

// Correct:
use common_enums::enums;
use serde::{Deserialize, Serialize};
```
**Why It Works:** Ensures each item is imported only once per module.

## Common Error: Type annotations needed (E0283)
### Error Message:
`error[E0283]: type annotations needed`
`cannot satisfy \`dlocal::Dlocal: hyperswitch_interfaces::api::ConnectorIntegration<_, _, _>\``
`help: try using a fully qualified path to specify the expected types: <dlocal::Dlocal as hyperswitch_interfaces::api::ConnectorIntegration<T, Req, Resp>>::get_content_type(self)`
### Root Cause:
The compiler cannot infer the generic types `T, Req, Resp` for the trait `ConnectorIntegration` when calling a method like `get_content_type` that is defined in `ConnectorCommonExt` (which has a `where Self: ConnectorIntegration<Flow, Request, Response>`). This usually happens when `get_content_type` is called in a context where these generics are ambiguous.
### Solution Pattern:
The `get_content_type` method in `ConnectorCommonExt` calls `self.common_get_content_type()`. The issue might be that `self.get_content_type()` is being called from within `build_headers` in `ConnectorCommonExt` itself, where the specific flow generics aren't fixed.
The `Dlocal` struct directly implements `ConnectorCommon` which has `common_get_content_type`.
The `build_headers` in `ConnectorCommonExt` should ideally use `self.common_get_content_type()` if it needs a content type without specific flow context.
However, for Dlocal, the `build_headers` in `ConnectorCommonExt` is mostly a placeholder as headers are built in flow-specific `build_request`.
The error might be from a call like `self.get_content_type()` inside `build_request` where it should be `types::FlowSpecificType::get_content_type(self, req, connectors)`.
The actual fix for the Dlocal connector was that `self.get_content_type()` was called inside `build_request` where it should have been `self.common_get_content_type()` because the `get_headers` for each flow was returning an empty vec, and `build_request` was constructing all headers.
```rust
// In build_request:
// Incorrect if get_headers is empty:
// headers.push((headers::CONTENT_TYPE.to_string(), self.get_content_type().to_string().into_masked()));

// Correct if using common_get_content_type:
headers.push((headers::CONTENT_TYPE.to_string(), self.common_get_content_type().to_string().into_masked()));
```
**Why It Works:** `common_get_content_type` is non-generic and directly available.

## Common Error: Method not found (e.g., `get_email_for_connector`) - Revisited
### Error Message:
`error[E0599]: no method named \`get_email_for_connector\` found for struct \`PaymentsAuthorizeData\` in the current scope`
### Root Cause:
The method `get_email_for_connector` is provided by the `PaymentsAuthorizeRequestData` trait (from `crate::utils`), which needs to be implemented for or accessed via `RouterData<_, PaymentsAuthorizeData, _>.request`.
### Solution Pattern:
Ensure the trait `PaymentsAuthorizeRequestData` (and similar for other flows like `RefundsRequestData`) is imported and used correctly. The method is on the `request` field of `RouterData`.
```rust
use crate::utils::PaymentsAuthorizeRequestData; // Import the trait

// In transformers.rs, inside TryFrom for DlocalPaymentsRequest:
// let payer_email = item.router_data.request.get_email_for_connector()?;
// This assumes PaymentsAuthorizeRequestData is implemented for PaymentsAuthorizeData struct.
```
**Why It Works:** The trait provides the method. The `real-codebase` pattern often involves `item.router_data.request.email.clone()` if `email` is a direct field on `PaymentsAuthorizeData`. If `get_email_for_connector` is a helper, ensure it's correctly defined and used. The error indicates it's not found directly on `PaymentsAuthorizeData`. The fix is to use `item.router_data.get_billing_email()` or `item.router_data.request.email` if available. The `real-codebase` uses `item.router_data.request.email.clone()`. My implementation was trying to call it on `request` which is `PaymentsAuthorizeData`. The method `get_email_for_connector` is on `RouterData` itself (from `crate::utils::RouterData as _`).
**Corrected Pattern:**
```rust
use crate::utils::RouterData as _; // For RouterData methods

// ...
let payer_email = item.router_data.get_email_for_connector()?;
// OR if directly on request:
// let payer_email = item.router_data.request.email.clone().ok_or_else(...)?;
```
The error was in `transformers.rs`, where `item.router_data.request.get_email_for_connector()?` was used. The method `get_email_for_connector` is on `RouterData` itself, not on the `request` field.
**Final Fix:** `let payer_email = item.router_data.get_email_for_connector()?;` (after importing `crate::utils::RouterData as _`).

## Common Error: No variant `NoContent` or `Empty` for `RequestContent` (E0599) - Revisited
### Error Message:
`error[E0599]: no variant or associated item named \`NoContent\` found for enum \`RequestContent\` in the current scope`
`error[E0599]: no variant or associated item named \`Empty\` found for enum \`RequestContent\` in the current scope`
### Root Cause:
The specific variants `NoContent` or `Empty` might not be available or are named differently in the version of `common_utils::request::RequestContent` being used. The `real-codebase` pattern for `ConnectorCommonExt::build_headers` involves calling `request_content.get_inner_value().peek().to_owned()` to get a string representation of the body. For truly empty bodies (like GET requests or body-less POSTs), `get_request_body` should return a `RequestContent` variant that `get_inner_value().peek().to_owned()` correctly interprets as an empty string.
### Solution Pattern:
For GET requests or POST/PUT requests that should have no actual body content for signature purposes:
```rust
// In flow-specific get_request_body:
Ok(RequestContent::Json(serde_json::Value::Null))
// Or:
// Ok(RequestContent::FormUrlEncoded(String::new()))
```
This ensures that `request_content.get_inner_value().peek().to_owned()` in `build_headers` results in an empty string.
If a `Content-Length: 0` header is strictly needed for some POST/PUT without a body, and `RequestContent::Json(serde_json::Value::Null)` doesn't achieve that, further investigation into `RequestBuilder` behavior is needed. However, for signature calculation, an empty string is the goal.
**Why It Works:** Provides a valid `RequestContent` that results in an empty string when processed by `get_inner_value().peek().to_owned()`, aligning with how `build_headers` calculates the signature.

## Common Error: Unexpected token / Expected identifier (Syntax Error from Tooling)
### Error Message:
`error: expected identifier, found \`<\``
`--> crates/hyperswitch_connectors/src/connectors/dlocal.rs:802:1`
`802 | </final_file_content>`
`    | ^ expected identifier`
### Root Cause:
This specific error was caused by the AI agent (myself) incorrectly including XML-like tags (e.g., `</final_file_content>`) within the actual file content during a `write_to_file` operation. These tags are part of the tool's response format, not valid Rust code.
### Solution Pattern:
Ensure that only valid code is written to files. When using `write_to_file` or `replace_in_file`, the `content` or `diff` sections must contain only the code itself, without any surrounding metadata tags or informational comments from the tool interaction.
### Prevention:
Carefully review the content being written to files to ensure no extraneous characters or tool-specific formatting is included. This includes XML-like tags (e.g., `<final_file_content>`) and informational comments (e.g., "IMPORTANT: For any future changes...").

**Variation: Informational Comment from Tooling**
### Error Message:
`error: expected one of \`!\` or \`::\`, found \`:\``
`--> crates/hyperswitch_connectors/src/connectors/dlocal.rs:802:10`
`802 | IMPORTANT: For any future changes to this file...`
`    |          ^ expected one of \`!\` or \`::\``
### Root Cause:
An informational comment, typically provided by the AI tooling system after a file operation (like "IMPORTANT: For any future changes..."), was incorrectly included as part of the file content itself. This is not valid Rust syntax.
### Solution:
Remove these non-code comment lines from the source file.


## Common Error: Test Panics with "Connector authentication file path not set: NotPresent"

### Error Message:
```
thread '...' panicked at crates/test_utils/src/connector_auth.rs:XXX:YY:
Connector authentication file path not set: NotPresent
```

### Root Cause:
This error occurs during integration testing (`cargo test --package router --test connectors -- <connector_name>`) when the test framework cannot find or access the connector authentication credentials. This is typically due to one of the following:
1.  The `CONNECTOR_AUTH_FILE_PATH` environment variable is not set before running the tests. This variable should point to a TOML file containing the authentication keys (e.g., `crates/router/tests/connectors/sample_auth.toml` or a local `auth.toml` override).
2.  The specified authentication file does not exist at the path provided by `CONNECTOR_AUTH_FILE_PATH`.
3.  The authentication file exists but does not contain the necessary (valid) credentials for the connector under test (e.g., missing `[bambora]` section or placeholder values).

### Solution Pattern:
1.  **Ensure `CONNECTOR_AUTH_FILE_PATH` is set**: Before running tests, export the environment variable:
    ```bash
    export CONNECTOR_AUTH_FILE_PATH="/path/to/your/hyperswitch/crates/router/tests/connectors/sample_auth.toml"
    # Or point to your local auth.toml if you use an override
    # export CONNECTOR_AUTH_FILE_PATH="/path/to/your/hyperswitch/crates/router/tests/connectors/auth.toml"
    ```
2.  **Verify Auth File**:
    *   Ensure the file specified by `CONNECTOR_AUTH_FILE_PATH` exists.
    *   Ensure the file contains a section for the connector being tested (e.g., `[bambora]`) with the correct, non-placeholder API keys and other required authentication parameters (e.g., `api_key = "your_actual_api_key"`, `key1 = "your_actual_merchant_id"`). Refer to `crates/router/tests/connectors/sample_auth.toml` for the expected format for each connector.
    *   **Security Note**: Do not commit real secret keys to `sample_auth.toml` if it's tracked by Git. Use a local, git-ignored `auth.toml` file for actual secrets and point `CONNECTOR_AUTH_FILE_PATH` to it during local testing.

### Why It Works:
The test utility `connector_auth.rs` relies on this environment variable to locate and load the necessary credentials to interact with the live connector APIs during integration tests. Without valid credentials, the tests cannot proceed.

## Common Error: Compilation Fails with "cannot find trait `RouterRequest`"

### Error Message:
```
error[E0405]: cannot find trait `RouterRequest` in module `...`
 --> crates/hyperswitch_connectors/src/connectors/your_connector/transformers.rs:XXX:YY
  |
XXX |     Req: some::path::RouterRequest,
  |                      ^^^^^^^^^^^^^ not found in `...`
```

### Root Cause:
This error occurs when the `RouterRequest` trait, which is a generic bound for request types in some `TryFrom` implementations (especially for converting `ResponseRouterData<F, ConnectorResponseType, Req, PaymentsResponseData>` back to `RouterData<F, Req, PaymentsResponseData>`), is not correctly pathed. The `RouterRequest` trait is defined in `hyperswitch_interfaces::api`.

### Solution Pattern:
Ensure the `RouterRequest` trait is correctly referenced in the `where` clause of your `TryFrom` implementation. If `hyperswitch_interfaces::api` is imported as `api`, the correct usage is `Req: api::RouterRequest`.

Example:
```rust
use hyperswitch_interfaces::{api, errors}; // Ensure api is in scope

// ... other structs ...

// In a TryFrom implementation, for example:
impl<F, Req, Resp> TryFrom<ResponseRouterData<F, YourConnectorResponseType, Req, Resp>>
    for RouterData<F, Req, Resp>
where
    Req: api::RouterRequest, // Correct path to RouterRequest
    // ... other bounds ...
{
    // ... implementation ...
}
```

### Why It Works:
The compiler needs the correct path to locate the `RouterRequest` trait definition. Using `api::RouterRequest` (assuming `use hyperswitch_interfaces::api;`) provides this correct path.

## Common Error: Unresolved Imports for RouterData Wrappers or Specific Types

### Error Message:
```
error[E0432]: unresolved import `crate::types::ResponseRouterDataCommon`
  --> crates/hyperswitch_connectors/src/connectors/your_connector/transformers.rs:XXX:YY
   |
XXX | use crate::types::ResponseRouterDataCommon;
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ no `ResponseRouterDataCommon` in `types`

error[E0432]: unresolved import `hyperswitch_types::AccessTokenResponseRouterData`
  --> crates/hyperswitch_connectors/src/connectors/your_connector/transformers.rs:XXX:YY
   |
XXX | use hyperswitch_types::AccessTokenResponseRouterData;
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ no `AccessTokenResponseRouterData` in `types` (referring to `hyperswitch_domain_models::types`)
```
Or similar errors for `PaymentsAuthorizeRouterData`, `AccessTokenAuthRouterData`, `ConnectorTransactionIdType`, etc.

### Root Cause:
These errors typically arise from incorrect import paths for types that might be defined in one crate (e.g., `hyperswitch_domain_models`) and re-exported or aliased in another (e.g., `crate::types` which is `hyperswitch_interfaces::types`, or `hyperswitch_types` which is an alias for `hyperswitch_domain_models::types`). The exact location of these types can sometimes be confusing.

Common specific causes:
1.  `ResponseRouterDataCommon`: This trait is usually found in `hyperswitch_interfaces::types`.
2.  `AccessTokenResponseRouterData`: This specific router data wrapper is often in `hyperswitch_domain_models::types` (aliased as `hyperswitch_types`).
3.  `Payments...RouterData` (e.g., `PaymentsAuthorizeRouterData`): These are often type aliases in `hyperswitch_domain_models::types` for `RouterData<Flow, RequestType, ResponseType>`.
4.  `AccessTokenAuthRouterData`: Similar to above, a type alias in `hyperswitch_domain_models::types`.
5.  `ConnectorTransactionIdType`: This enum is in `hyperswitch_domain_models::types`.
6.  `AccessTokenAuthType` (trait): This trait is in `hyperswitch_interfaces::api::payments`.

### Solution Pattern:
1.  **Verify Re-exports**: Check if `crate::types` (i.e., `hyperswitch_interfaces::types`) re-exports the required type. If it does, `use crate::types::TypeName;` should work.
2.  **Use Direct Paths**: If re-exports are unclear or not present, use the direct path from the defining crate.
    *   For `ResponseRouterDataCommon`: `use hyperswitch_interfaces::types::ResponseRouterDataCommon;`
    *   For `AccessTokenResponseRouterData` and other `...RouterData` wrappers: `use hyperswitch_domain_models::types::AccessTokenResponseRouterData;` (or `use hyperswitch_types::AccessTokenResponseRouterData;` if `hyperswitch_types` is an alias for `hyperswitch_domain_models::types`).
    *   For `ConnectorTransactionIdType`: `use hyperswitch_domain_models::types::ConnectorTransactionIdType;`
    *   For `AccessTokenAuthType` trait: `use hyperswitch_interfaces::api::payments::AccessTokenAuthType;`
3.  **Check Aliases**: Be mindful of aliases like `hyperswitch_types` for `hyperswitch_domain_models::types`.
4.  **Compiler Suggestions**: Pay close attention to the compiler's "help: a similar name exists in the module" or "help: consider importing one of these items instead" suggestions, as they often point to the correct location.

Example for `AccessTokenResponseRouterData` in `transformers.rs`:
```rust
// In transformers.rs
use hyperswitch_domain_models::types::AccessTokenResponseRouterData; // Or use hyperswitch_types alias

// ... rest of the code
```

Example for `AccessTokenAuthType` trait in `connector.rs`:
```rust
// In connector.rs (e.g., airwallex.rs)
use hyperswitch_interfaces::api::payments::AccessTokenAuthType as AccessTokenAuthTypeTrait; // Alias if needed

// ...
// impl ConnectorIntegration<AccessTokenAuth, AccessTokenRequestData, AccessToken> for MyConnector {
//    fn build_request(...) -> {
//        RequestBuilder::new().url(&AccessTokenAuthTypeTrait::get_url(self, req, connectors)?)
//    }
// }
```

### Why It Works:
Rust's module system requires precise paths for types and traits. Errors occur if the compiler cannot find the definition at the specified path. Using the correct, fully qualified path or a valid re-export resolves the ambiguity.

## Common Error: E0117 - Orphan Rule Violation for `TryFrom`

### Error Message:
```
error[E0117]: only traits defined in the current crate can be implemented for types defined outside of the crate
  --> crates/hyperswitch_connectors/src/connectors/your_connector/transformers.rs:XXX:Y
   |
XXX | impl TryFrom<hyperswitch_domain_models::router_data::RouterData<...>>
   | ^    ----------------------------------------------------------------- `RouterData` is not defined in the current crate
   | |
YYY |     for hyperswitch_domain_models::router_data::RouterData<...>
   |     ----------------------------------------------------------- `RouterData` is not defined in the current crate
   |
   = note: impl doesn't have any local type before any uncovered type parameters
```

### Root Cause:
This error occurs when attempting to implement a foreign trait (like `std::convert::TryFrom`) for a foreign type (like `hyperswitch_domain_models::router_data::RouterData`). Rust's orphan rule prevents this to avoid conflicting implementations.

### Solution Pattern:
Implement the `TryFrom` trait using a local wrapper type, typically `crate::types::ResponseRouterData`, as the input type. `crate::types::ResponseRouterData` is defined within the `hyperswitch_connectors` crate, making it a local type for the implementation.

```rust
// In your_connector/transformers.rs
use crate::types::ResponseRouterData; // Local wrapper type
use hyperswitch_domain_models::router_data::{AccessToken, RouterData as HyperswitchRouterData};
use hyperswitch_domain_models::router_flow_types::access_token_auth::AccessTokenAuth;
use hyperswitch_domain_models::router_request_types::AccessTokenRequestData;
// ... other imports ...

// Example: Converting a connector-specific auth response
pub struct YourConnectorAuthResponse { /* ... fields ... */ }

impl TryFrom<ResponseRouterData<AccessTokenAuth, YourConnectorAuthResponse, AccessTokenRequestData, AccessToken>>
    for HyperswitchRouterData<AccessTokenAuth, AccessTokenRequestData, AccessToken>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<AccessTokenAuth, YourConnectorAuthResponse, AccessTokenRequestData, AccessToken>,
    ) -> Result<Self, Self::Error> {
        // item.response is YourConnectorAuthResponse
        // item.data is the original HyperswitchRouterData
        Ok(Self {
            response: Ok(AccessToken {
                token: item.response.token_field, // Access field from YourConnectorAuthResponse
                expires: item.response.expires_field,
            }),
            ..item.data // Spread fields from original RouterData
        })
    }
}
```

### Why It Works:
By using `crate::types::ResponseRouterData` as the input type in `TryFrom<crate::types::ResponseRouterData<...>>`, at least one of the types involved in the `impl` (the input type) is local to the current crate (`hyperswitch_connectors`), satisfying the orphan rule.

## Common Error: E0432/E0433 - Unresolved Import for `api_models` or `id_type`

### Error Message:
```
error[E0433]: failed to resolve: could not find `api_models` in `hyperswitch_interfaces`
  --> crates/hyperswitch_connectors/src/connectors/your_connector.rs:XX:Y
   |
XX | use hyperswitch_interfaces::api_models::payments::PaymentIdType;
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

error[E0432]: unresolved import `hyperswitch_domain_models::id_type`
  --> crates/hyperswitch_connectors/src/connectors/your_connector.rs:XX:Y
   |
XX | use hyperswitch_domain_models::id_type;
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ no `id_type` in the root
```

### Root Cause:
1.  `api_models` is a separate crate, not a submodule of `hyperswitch_interfaces`.
2.  `id_type` is a module within `common_utils`, not directly under `hyperswitch_domain_models`.

### Solution Pattern:
1.  Import `PaymentIdType` (and other types from `api_models`) directly:
    ```rust
    use api_models::payments::PaymentIdType;
    ```
    Ensure `api_models` is listed as a dependency in the `hyperswitch_connectors/Cargo.toml`.
2.  Import `id_type` from `common_utils`:
    ```rust
    use common_utils::id_type;
    ```

## Common Error: E0599 - Method Not Found (e.g., `get_amount`, `parse_struct`)

### Error Message:
```
error[E0599]: no method named `get_amount` found for struct `PaymentsPreProcessingData`
error[E0599]: no method named `parse_struct` found for reference `&[u8]`
```

### Root Cause:
These methods are often provided by traits that are not in scope.
*   `get_amount`, `get_currency`, `get_browser_info`, etc., on request data structs are typically from traits defined in `crate::utils` (e.g., `PaymentsAuthorizeRequestData`, `PaymentsPreProcessingRequestData`).
*   `parse_struct` on byte slices (`&[u8]`) is from the `ByteSliceExt` trait in `common_utils::ext_traits`.

### Solution Pattern:
Import the necessary traits:
```rust
// For request data accessors (in transformers.rs or where request data is handled)
use crate::utils::{PaymentsAuthorizeRequestData, PaymentsPreProcessingRequestData /*, etc. */};

// For parse_struct (usually in connector.rs where responses are parsed)
use common_utils::ext_traits::ByteSliceExt;
```

## Common Error: E0599 - No Variant `RequestContent::None`

### Error Message:
```
error[E0599]: no variant or associated item named `None` found for enum `RequestContent`
  --> crates/hyperswitch_connectors/src/connectors/your_connector.rs:XXX:YY
   |
XXX |         Ok(RequestContent::None)
   |                            ^^^^ variant or associated item not found
```

### Root Cause:
The variant for an empty request body in `common_utils::request::RequestContent` is `Empty`.

### Solution Pattern:
Use `RequestContent::Empty`.
```rust
use common_utils::request::RequestContent;
// ...
Ok(RequestContent::Empty)
```

## Common Error: E0277 - `Serialize` or `Default` Trait Not Implemented

### Error Message:
```
error[E0277]: the trait `Serialize` is not implemented for `YourConnectorResponse`
error[E0277]: the trait `Default` is not implemented for `YourConnectorStatusEnum`
```

### Root Cause:
1.  Connector-specific response structs passed to `event_builder.set_response_body()` require `serde::Serialize`.
2.  If a struct derives `Default` and contains an enum field, that enum must also derive `Default` or have a `#[default]` variant for `serde` if the enum itself is also deserialized with a default.

### Solution Pattern:
1.  Add `#[derive(serde::Serialize)]` to your connector-specific response structs.
    ```rust
    #[derive(serde::Serialize, serde::Deserialize, Debug)]
    pub struct YourConnectorAuthResponse { /* ... */ }
    ```
2.  Add `#[derive(Default)]` or `#[serde(default)]` with a `#[default]` variant to enums used in structs that derive `Default`.
    ```rust
    #[derive(Debug, Clone, Default, Deserialize, PartialEq, Serialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum YourConnectorStatus {
        Succeeded,
        Failed,
        #[default] // Important if this enum is part of a struct deriving Default
        Pending,
    }
    ```

## Common Error: E0609/E0560 - Field Access on `Result` or Incorrect Field Name

### Error Message:
```
error[E0609]: no field `token` on type `Result<YourConnectorAuthResponse, ErrorResponse>`
error[E0560]: struct `RouterData<...>` has no field named `data`
```

### Root Cause:
1.  Accessing fields on a `Result` type without unwrapping (e.g., `item.response.token` when `item.response` is `Result<_,_>`).
2.  Trying to spread `..item.data` when `item` is already the `RouterData` struct (which doesn't have a sub-field named `data` for this purpose). This usually happens after incorrectly changing a `TryFrom` signature.

### Solution Pattern:
1.  Properly handle the `Result` before accessing fields, e.g., using `?`, `match`, or `as_ref().ok()?`:
    ```rust
    // If item.response is Result<ActualResponse, ErrorResponse>
    // let token = item.response.as_ref().ok()?.token_field;
    // Or if item.response is ActualResponse directly (preferred in TryFrom<crate::types::ResponseRouterData<...>>)
    // let token = item.response.token_field;
    ```
2.  When constructing `RouterData` and copying fields from an existing `RouterData` (e.g., `original_router_data`), use `..original_router_data` if you are in a `TryFrom` that outputs `RouterData` and takes `crate::types::ResponseRouterData` (where `original_router_data` is `item.data`).
    If constructing `RouterData` manually in `handle_response`, copy fields individually or ensure the source of spread `..` is appropriate.

## Common Error: E0308 - Mismatched Types for `RedirectForm.form_fields`

### Error Message:
```
error[E0308]: mismatched types
  --> .../transformers.rs:XXX:YY
   |
XXX|                       form_fields: body.map_or_else(...)
   |                                    ^^^^^^^^^^^^^^^^^^^^ expected `HashMap<String, String>`, found `Map<String, Value>`
```

### Root Cause:
The `form_fields` in `RedirectForm` expects `HashMap<String, String>`, but `serde_json::from_value` on a JSON object often yields `serde_json::Map<String, serde_json::Value>`.

### Solution Pattern:
Convert the `serde_json::Map<String, serde_json::Value>` to `HashMap<String, String>`, ensuring each `serde_json::Value` is converted to a `String`.
```rust
use serde_json::Value;
use std::collections::HashMap;
// ...
// let json_map: serde_json::Map<String, Value> = ...;
let form_fields: HashMap<String, String> = json_map
    .into_iter()
    .filter_map(|(k, v)| {
        v.as_str().map(|s| (k, s.to_string())) // Simplest: only take string values
        // More robust: handle numbers, booleans by converting to string
        // match v {
        //    Value::String(s) => Some((k, s)),
        //    Value::Number(n) => Some((k, n.to_string())),
        //    Value::Bool(b) => Some((k, b.to_string())),
        //    _ => None, // Or skip, or error
        // }
    })
    .collect();
```
The `real-codebase/airwallex/transformers.rs` shows a pattern of directly constructing the HashMap with known string key-value pairs if the structure is fixed.

## Common Error: E0063 - Missing Fields in Struct Initializer

### Error Message:
```
error[E0063]: missing fields `connector_mandate_request_reference_id` and `mandate_metadata` in initializer of `MandateReference`
```

### Root Cause:
The `MandateReference` struct (or any struct) requires all its non-Option fields to be specified during initialization, or if they are `Option`, they can be `None`.

### Solution Pattern:
Ensure all required fields are provided. For optional fields that are not available, explicitly set them to `None`.
```rust
use hyperswitch_domain_models::router_response_types::MandateReference;
// ...
MandateReference {
    connector_mandate_id: Some("some_id".to_string()),
    payment_method_id: None,
    connector_mandate_request_reference_id: None, // Provide if missing
    mandate_metadata: None,                      // Provide if missing
}
