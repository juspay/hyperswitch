# Connector Patterns

_This file will store common connector implementation patterns and variations._

## Pattern Variation: HMAC Signature Generation and Header Construction for Dlocal (Aligned with `real-codebase`)
### Standard Implementation:
Typically, HMAC signatures and header construction might be handled in flow-specific `build_request` methods or with more granular helpers.
### Dlocal Variation (Aligned with `real-codebase`):
The `real-codebase` pattern for Dlocal centralizes all header generation, including the dynamic Dlocal authentication headers and HMAC signature, within the `ConnectorCommonExt::build_headers` method.

- **Centralized Logic:** `ConnectorCommonExt::build_headers` is responsible for:
    - Retrieving the request body string (using `self.get_request_body()` and then serializing or extracting the string content).
    - Generating the `X-Date` timestamp.
    - Extracting authentication details (`X-Login`, `X-Trans-Key`, `SecretKey`) from `req.connector_auth_type`.
    - Constructing the signature payload: `X-Login + X-Date + RequestBodyString`.
        - **Note on GET Requests:** For GET requests, `RequestBodyString` is empty. Thus, the signature payload becomes `X-Login + X-Date`. This differs from Dlocal's documentation which specifies `X-Login + X-Date + RequestPathAndQuery` for GET requests. This approach prioritizes consistency with the `real-codebase`.
    - Generating the HMAC SHA256 signature.
    - Assembling all required headers: `Authorization` (with the signature), `X-Login`, `X-Trans-Key`, `X-Date`, `X-Version`, and `Content-Type` (if the request body is not empty).
- **Flow-specific `get_headers`:** These methods in each `ConnectorIntegration` implementation simply call `self.build_headers(req, connectors)`.
- **Flow-specific `build_request`:** These methods use `RequestBuilder::new().attach_default_headers().headers(self.get_headers(...))...` to include the centrally generated headers.
- **Removal of Custom Helpers:** Any custom HMAC generation helpers (like a standalone `generate_dlocal_hmac_signature` function) are removed, and their logic is integrated into `ConnectorCommonExt::build_headers`.

```rust
// In ConnectorCommonExt<Flow, Req, Resp> for Dlocal:
fn build_headers(
    &self,
    req: &RouterData<Flow, Req, Resp>,
    connectors: &Connectors,
) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
    let request_content = self.get_request_body(req, connectors)?;
    let request_body_str = match request_content {
        RequestContent::Json(body) => serde_json::to_string(&body) // Assuming body is Serialize
            .change_context(errors::ConnectorError::RequestEncodingFailed)?,
        RequestContent::FormUrlEncoded(body_str) => body_str, // If it's already a string
        RequestContent::Empty | RequestContent::NoContent => String::new(),
    };

    let date = date_time::date_as_yyyymmddthhmmssmmmz()
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
    let auth = dlocal::DlocalAuthType::try_from(&req.connector_auth_type) // From transformers
        .change_context(errors::ConnectorError::FailedToObtainAuthType)?;

    let sign_payload = format!("{}{}{}", auth.x_login.peek(), date, request_body_str);

    let signature = crypto::HmacSha256::sign_message(
        &crypto::HmacSha256,
        auth.secret_key.peek().as_bytes(), // DlocalAuthType needs a secret_key field
        sign_payload.as_bytes(),
    )
    .change_context(errors::ConnectorError::RequestEncodingFailed)
    .attach_printable("Failed to sign the message")?;
    let auth_string = format!("V2-HMAC-SHA256, Signature: {}", hex::encode(signature));

    let mut headers_vec = vec![
        (headers::AUTHORIZATION.to_string(), auth_string.into_masked()),
        (headers::X_LOGIN.to_string(), auth.x_login.clone().into_masked()),
        (headers::X_TRANS_KEY.to_string(), auth.x_trans_key.clone().into_masked()),
        (headers::X_VERSION.to_string(), "2.1".to_string().into()), // Dlocal API version
        (headers::X_DATE.to_string(), date.into_masked()),
    ];

    if !request_body_str.is_empty() {
        headers_vec.push((
            headers::CONTENT_TYPE.to_string(),
            self.get_content_type().to_string().into(), // Or self.common_get_content_type()
        ));
    }
    Ok(headers_vec)
}
```
**Reason for Variation & Alignment:** This pattern centralizes header and signature logic, promoting consistency and reducing redundancy across different payment flows. It aligns with the `real-codebase`'s established structure for Dlocal. The deviation in GET request signature content (path not included) is a deliberate choice to match `real-codebase`, and should be monitored for API compatibility.
The `RequestContent` needs to be handled appropriately to get the string for the signature. If it's `Json(Box<dyn ErasedMaskSerialize>)`, `serde_json::to_string` would need the concrete type. The `real-codebase` pattern implies `get_request_body` returns a type that can be stringified or `peek()`-ed.
My implementation now uses `serde_json::to_string(&body)` where `body` is `Box<dyn ErasedMaskSerialize>`, which might require the actual concrete type. The `real-codebase` uses `dlocal_req_content.get_inner_value().peek().to_owned()`. This detail is important for correct serialization. The updated code uses `serde_json::to_string(&body)` on the `Box<dyn ErasedMaskSerialize>`, which will work if the underlying type is `Serialize`.
The `DlocalAuthType` in `transformers.rs` must expose `x_login`, `x_trans_key`, and `secret_key`.

## Pattern Variation: Amount Conversion (StringMinorUnit to f64 Major Unit)
### Standard Implementation:
Some connectors might accept amounts in minor units (i64) or have utility functions that directly convert.
### Dlocal Variation:
Dlocal's API expects amounts as `f64` in major units (e.g., 100.00 for $100.00 USD). If the internal representation is `StringMinorUnit` or `MinorUnit (i64)`, a two-step conversion is needed:
1. Parse `StringMinorUnit` to `i64` (minor units).
2. Convert `i64` (minor units) to `f64` (major units) using currency-aware utility.
```rust
use common_utils::types::{StringMinorUnit, MinorUnit}; // Assuming MinorUnit has get_amount_as_i64
use hyperswitch_interfaces::errors;
use crate::utils as connector_utils; // For to_major_unit_as_f64

// Example for DlocalPaymentsRequest TryFrom:
// item.amount is StringMinorUnit
let minor_amount_i64 = item.amount.get_amount_as_i64()
    .map_err(|_| errors::ConnectorError::RequestEncodingFailed)?;
let major_amount_f64 = connector_utils::to_major_unit_as_f64(minor_amount_i64, item.router_data.request.currency)?;
// ...
// amount: major_amount_f64,
```
**Reason for Variation:** Dlocal's specific requirement for amount format in API requests.

## Pattern Variation: Request Body Serialization for Signature
### Standard Implementation:
Some connectors might not require the request body in the signature, or the `RequestContent` can be directly serialized.
### Dlocal Variation:
The Dlocal HMAC signature requires the JSON string of the request body. However, `RequestContent::Json(Box<dyn ErasedMaskSerialize>)` cannot be directly serialized with `serde_json::to_string()` using `.value()`.
The pattern is to:
1. Construct the concrete request struct.
2. Serialize this struct to a JSON string for the signature calculation.
3. Box the same concrete struct instance into `RequestContent::Json` for the actual request body.
```rust
// In build_request for POST/PUT flows:
// 1. Create the concrete request struct (e.g., DlocalPaymentsRequest)
let temp_connector_router_data = dlocal::DlocalRouterData::from((
    StringMinorUnit::new(req.request.minor_amount.get_amount_as_i64().to_string()),
    req,
));
let connector_req_struct = dlocal::DlocalPaymentsRequest::try_from(&temp_connector_router_data)?;

// 2. Serialize it to string for signature:
let request_body_str = serde_json::to_string(&connector_req_struct)
    .change_context(errors::ConnectorError::RequestEncodingFailed)?;

// 3. Create RequestContent by boxing the struct:
let request_body_content = RequestContent::Json(Box::new(connector_req_struct));

// ... then use request_body_str in generate_dlocal_hmac_signature
// ... and request_body_content in RequestBuilder::new().set_body()
```
**Reason for Variation:** Ensures the exact string representation of the request body is used for the signature, matching what the server will receive, while still using the `RequestContent` abstraction.
