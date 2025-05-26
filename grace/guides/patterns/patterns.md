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

## Common Error: [Error Name]

### Error Message:
[Exact error message]

### Root Cause:
[Explanation of what causes this error]

### Solution Pattern:
```rust
// Code showing correct implementation
```
Why It Works:
[Explanation of why this solution resolves the issue]

### Connector Pattern Variation

## Pattern Variation: [Pattern Name]

### Standard Implementation:
```rust
// Standard code implementation
```
[Connector] Variation:
```rust
// Connector-specific implementation
```
Reason for Variation:
[Explanation of why this connector requires a different approach]

## Pattern: Correct `TryFrom` for Connector Auth Response to `RouterData<AccessTokenAuth, ...>`

### Context:
Implementing `TryFrom` to convert a connector-specific authentication response (e.g., for access tokens) into the Hyperswitch `RouterData` struct, specifically for the `AccessTokenAuth` flow. This pattern addresses Rust's orphan rule (E0117) by using the local `crate::types::ResponseRouterData` wrapper.

### Standard Implementation:
```rust
// In your_connector/transformers.rs
use crate::types::ResponseRouterData; // Local wrapper: hyperswitch_connectors::types::ResponseRouterData
use hyperswitch_domain_models::{
    router_data::{AccessToken, RouterData as HyperswitchRouterData}, // Core RouterData
    router_flow_types::access_token_auth::AccessTokenAuth,      // Flow marker
    router_request_types::AccessTokenRequestData,               // Request data type for this flow
};
use hyperswitch_interfaces::errors; // For error handling
// Potentially: use common_utils::date_time;

// Define your connector's specific authentication response structure
#[derive(Debug, serde::Deserialize, serde::Serialize)] // Ensure Serialize for event_builder
pub struct YourConnectorAuthResponse {
    pub token_field: masking::Secret<String>,
    pub expires_field: i64, // Or PrimitiveDateTime, etc.
    // ... other fields
}

// Implement TryFrom using the local ResponseRouterData wrapper as input
impl TryFrom<ResponseRouterData<AccessTokenAuth, YourConnectorAuthResponse, AccessTokenRequestData, AccessToken>>
    for HyperswitchRouterData<AccessTokenAuth, AccessTokenRequestData, AccessToken>
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: ResponseRouterData<AccessTokenAuth, YourConnectorAuthResponse, AccessTokenRequestData, AccessToken>,
    ) -> Result<Self, Self::Error> {
        // item.response is YourConnectorAuthResponse (the direct connector response)
        // item.data is the original HyperswitchRouterData passed into crate::types::ResponseRouterData

        // Example: Calculate expiry if needed
        // let expires_in_seconds = (item.response.expires_at_datetime - common_utils::date_time::now()).whole_seconds();

        Ok(Self {
            response: Ok(AccessToken { // Construct the Hyperswitch AccessToken struct
                token: item.response.token_field,
                expires: item.response.expires_field, // Or calculated expires_in_seconds
            }),
            ..item.data // Spread all other fields from the original RouterData
        })
    }
}
```
### Why It Works:
*   **Orphan Rule (E0117)**: `crate::types::ResponseRouterData` is local to the `hyperswitch_connectors` crate, so implementing `TryFrom` for it is allowed.
*   **Data Flow**: The `item.response` field of `crate::types::ResponseRouterData` holds the direct deserialized response from the connector. `item.data` holds the original `RouterData` instance that was passed into the flow, allowing its fields to be copied over.
*   **Clarity**: Clearly separates the connector's raw response from the standardized Hyperswitch `AccessToken` and `RouterData` structures.

## Pattern: Correct `TryFrom` for Connector Payment Response to `RouterData<PaymentFlow, ...>`

### Context:
Implementing `TryFrom` to convert a connector-specific payment response into the Hyperswitch `RouterData` struct for various payment flows (Authorize, Capture, PSync, etc.). This also uses `crate::types::ResponseRouterData` to satisfy the orphan rule.

### Standard Implementation:
```rust
// In your_connector/transformers.rs
use crate::types::ResponseRouterData; // Local wrapper
use hyperswitch_domain_models::{
    router_data::{RouterData as HyperswitchRouterData},
    router_flow_types::payments::Authorize, // Example flow, replace with actual flow
    router_request_types::PaymentsAuthorizeData, // Example request, replace
    router_response_types::{PaymentsResponseData, RedirectForm}, // Hyperswitch response enum
    // Potentially: types::PaymentsAuthorizeRouterData as TargetRouterData, (if using specific type alias for output)
};
use hyperswitch_interfaces::errors;
use common_enums; // For AttemptStatus, etc.

// Define your connector's specific payment response structure
#[derive(Debug, serde::Deserialize, serde::Serialize)] // Ensure Serialize for event_builder
pub struct YourConnectorPaymentResponse {
    pub transaction_id: String,
    pub status_from_connector: String, // e.g., "approved", "failed"
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub redirect_url: Option<String>,
    // ... other fields
}

// Implement TryFrom using the local ResponseRouterData wrapper
impl TryFrom<ResponseRouterData<Authorize, YourConnectorPaymentResponse, PaymentsAuthorizeData, PaymentsResponseData>>
    for HyperswitchRouterData<Authorize, PaymentsAuthorizeData, PaymentsResponseData>
    // Or for types::PaymentsAuthorizeRouterData if that's the target alias
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: ResponseRouterData<Authorize, YourConnectorPaymentResponse, PaymentsAuthorizeData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        // Convert connector status to Hyperswitch AttemptStatus
        let hyperswitch_status = match item.response.status_from_connector.as_str() {
            "approved" => common_enums::AttemptStatus::Charged, // Or Authorized if capture is manual
            "failed" => common_enums::AttemptStatus::Failure,
            _ => common_enums::AttemptStatus::Pending,
        };

        let redirection_data = item.response.redirect_url.map(|url| {
            Box::new(RedirectForm::Form { // Or other RedirectForm variants
                endpoint: url,
                method: common_utils::request::Method::Get, // Or Post
                form_fields: std::collections::HashMap::new(), // Populate if POST with data
            })
        });
        
        // Construct PaymentsResponseData variant
        let payments_response_data = if hyperswitch_status == common_enums::AttemptStatus::Failure {
            // This construction is for the 'response: Err(ErrorResponse)' case,
            // which is typically handled by the main connector logic calling build_error_response.
            // This TryFrom usually handles the Ok(PaymentsResponseData) case.
            // For simplicity, we'll assume success path here.
            // If error, the main connector logic would return RouterData with response: Err(...)
            unreachable!("Error case should be handled before this TryFrom typically")
        } else {
            PaymentsResponseData::TransactionResponse {
                resource_id: hyperswitch_domain_models::router_request_types::ResponseId::ConnectorTransactionId(item.response.transaction_id.clone()),
                redirection_data,
                mandate_reference: None, // Populate if applicable
                connector_metadata: None, // Populate if applicable
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.transaction_id),
                incremental_authorization_allowed: None,
                charges: None,
            }
        };

        Ok(Self {
            status: hyperswitch_status, // Update RouterData status
            response: Ok(payments_response_data),
            // reference_id: Some(item.response.transaction_id), // If applicable for RouterData.reference_id
            ..item.data // Spread other fields
        })
    }
}
```

## Pattern: Importing Request Data Accessor Traits

### Context:
Accessing fields from request data structs (e.g., `PaymentsAuthorizeData`, `PaymentsPreProcessingData`) often requires helper methods like `get_amount()`, `get_currency()`, `get_browser_info()`. These methods are provided by traits.

### Standard Implementation:
These traits are typically defined in `crate::utils` (i.e., `hyperswitch_connectors::utils`).
```rust
// In your_connector/transformers.rs
use crate::utils::{
    PaymentsAuthorizeRequestData, 
    PaymentsPreProcessingRequestData, 
    RefundsRequestData, 
    // ... import other necessary traits
};

// Example usage:
// fn some_function(item: &types::PaymentsAuthorizeRouterData) {
//     let browser_info = item.request.get_browser_info();
//     let amount = item.request.get_amount();
// }
```
### Why It Works:
Importing these traits brings the accessor methods into scope, allowing them to be called on the respective request data structs.

## Pattern: Importing `ByteSliceExt` for `parse_struct`

### Context:
The `parse_struct` method, used to deserialize connector responses from raw byte slices (`&[u8]`), is an extension trait method.

### Standard Implementation:
The `ByteSliceExt` trait is defined in `common_utils::ext_traits`.
```rust
// In your_connector.rs (e.g., airwallex.rs)
use common_utils::ext_traits::ByteSliceExt;

// Example usage:
// fn handle_response(...) {
//     let response_struct: YourConnectorResponseType = res.response // res.response is &[u8]
//         .parse_struct("YourConnectorResponseType")
//         .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
// }
```
### Why It Works:
Importing `ByteSliceExt` makes the `parse_struct` method available on `&[u8]` types.

## Pattern: Populating `RedirectForm.form_fields`

### Context:
The `form_fields` field of `hyperswitch_domain_models::router_response_types::RedirectForm::Form` expects a `HashMap<String, String>`. If the redirect data comes as a JSON object, its values need to be converted to strings.

### Standard Implementation:
```rust
use serde_json::Value;
use std::collections::HashMap;
use hyperswitch_domain_models::router_response_types::RedirectForm;
use common_utils::request::Method as RequestMethod;


// Assuming `redirect_json_data` is a serde_json::Value::Object
// let redirect_json_data: Value = ...; 
// let redirect_url: String = ...;
// let redirect_method: RequestMethod = RequestMethod::Post; // Or Get

let form_fields = if let Value::Object(map) = redirect_json_data {
    map.into_iter()
        .filter_map(|(k, v)| {
            match v {
                Value::String(s) => Some((k, s)),
                Value::Number(n) => Some((k, n.to_string())),
                Value::Bool(b) => Some((k, b.to_string())),
                // Decide how to handle Null, Array, Object. Often skipped or error.
                _ => None, 
            }
        })
        .collect::<HashMap<String, String>>()
} else {
    HashMap::new()
};

let redirection_data = Some(Box::new(RedirectForm::Form {
    endpoint: redirect_url,
    method: redirect_method,
    form_fields,
}));
```
### Why It Works:
This pattern explicitly iterates through the JSON object's key-value pairs, converts values to strings (handling common types like String, Number, Bool), and collects them into the required `HashMap<String, String>`.

## Pattern: Constructing `ObjectReferenceId::PaymentId` for Webhooks

### Context:
When handling webhooks, the `get_webhook_object_reference_id` method needs to return an `api_models::webhooks::ObjectReferenceId`. For payment-related webhooks, this is often `ObjectReferenceId::PaymentId`.

### Standard Implementation:
```rust
// In your_connector.rs
use api_models::payments::PaymentIdType;
use api_models::webhooks::ObjectReferenceId;
use hyperswitch_domain_models::id_type; // For id_type::PaymentId or GlobalPaymentId
use std::borrow::Cow;
use error_stack::ResultExt; // For change_context & attach_printable
use hyperswitch_interfaces::errors;

// Assuming `webhook_source_id_string` is the relevant ID from the webhook payload (String)
// fn get_webhook_object_reference_id(...) -> CustomResult<ObjectReferenceId, errors::ConnectorError> {
//     let payment_id_v1 = id_type::PaymentId::try_from(Cow::Owned(webhook_source_id_string.clone()))
//         .change_context(errors::ConnectorError::WebhookReferenceIdNotFound)
//         .attach_printable("Failed to parse source_id as v1 PaymentId for webhook")?;
//     
//     // For v2, you might parse into id_type::GlobalPaymentId if applicable
//     // let global_payment_id = id_type::GlobalPaymentId::try_from(Cow::Owned(webhook_source_id_string))
//     //    .change_context(errors::ConnectorError::WebhookReferenceIdNotFound)
//     //    .attach_printable("Failed to parse source_id as GlobalPaymentId for webhook")?;

//     Ok(ObjectReferenceId::PaymentId(
//         PaymentIdType::PaymentIntentId(payment_id_v1) // Or appropriate variant of PaymentIdType
//     ))
// }
```
### Why It Works:
*   Uses the correct `api_models::payments::PaymentIdType` enum.
*   Uses the correct `api_models::webhooks::ObjectReferenceId::PaymentId` variant.
*   Correctly converts the string ID from the webhook payload into the appropriate Hyperswitch ID type (e.g., `id_type::PaymentId` or `id_type::GlobalPaymentId`) using `TryFrom`.

## Pattern: Deriving `Serialize` and `Default` for Connector Structs/Enums

### Context:
Connector-specific data structures often need to be serialized (e.g., for logging in `event_builder`) or require default instances.

### Standard Implementation:
*   **`serde::Serialize`**: Add `#[derive(serde::Serialize)]` (and often `serde::Deserialize`, `Debug`, `Clone`) to structs/enums that will be serialized.
    ```rust
    #[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
    pub struct YourConnectorResponse { /* ... */ }
    ```
*   **`Default`**:
    *   For structs where all fields have a natural default or are `Option`, derive `Default`:
      ```rust
      #[derive(Debug, Default, serde::Deserialize, serde::Serialize, Clone)]
      pub struct YourConnectorStructWithDefaults {
          pub field1: Option<String>,
          pub count: i32, // Default is 0
      }
      ```
    *   For enums used within a struct that derives `Default`, the enum itself must derive `Default` or have one variant marked with `#[default]` (if using `serde(default)` on the field in the parent struct).
      ```rust
      #[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize, PartialEq)]
      #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
      pub enum YourConnectorStatus {
          Succeeded,
          Failed,
          #[default] // This variant will be used if the struct derives Default
          Pending,
      }

      #[derive(Debug, Default, serde::Deserialize, serde::Serialize)]
      pub struct ResponseContainingStatus {
          pub status: YourConnectorStatus, // YourConnectorStatus needs to be Default
          pub other_field: Option<String>,
      }
      ```
### Why It Works:
*   `Serialize` allows the struct/enum to be converted into formats like JSON.
*   `Default` allows for easy creation of default instances, which is often required by other derives or generic code.

## Pattern: HTTP Basic Authentication

### Context:
Some connectors use HTTP Basic Authentication with username and password credentials that need to be base64 encoded.

### Standard Implementation:
```rust
// In connector's get_auth_header method
fn get_auth_header(
    &self,
    auth_type: &ConnectorAuthType,
) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
    let auth = connector_name::ConnectorNameAuthType::try_from(auth_type)
        .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
    
    // Format: username:password
    let auth_string = format!(
        "{}:{}",
        auth.username.expose(),
        auth.password.expose()
    );
    
    // Base64 encode using standard engine
    let encoded_auth = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        auth_string.as_bytes()
    );
    
    Ok(vec![(
        headers::AUTHORIZATION.to_string(),
        format!("Basic {}", encoded_auth).into_masked(),
    )])
}
```

### Why It Works:
- Uses the standard base64 engine for encoding
- Properly formats the header as `Basic <encoded_credentials>`
- Handles sensitive data with `expose()` only when necessary

### Example: Spreedly Implementation
```rust
let auth_string = format!(
    "{}:{}",
    auth.environment_key.expose(),
    auth.access_secret.expose()
);
let encoded_auth = base64::Engine::encode(
    &base64::engine::general_purpose::STANDARD,
    auth_string.as_bytes()
);
```

## Pattern: Merchant-Specific Configuration via Connector Metadata

### Context:
When a connector requires merchant-specific configuration beyond standard authentication credentials (e.g., gateway tokens, merchant IDs, endpoint prefixes).

### Standard Implementation:
```rust
// Extracting configuration from connector_meta_data
let config_value = req
    .connector_meta_data
    .as_ref()
    .and_then(|meta| meta.peek().as_object())
    .and_then(|obj| obj.get("config_key"))
    .and_then(|value| value.as_str())
    .ok_or(errors::ConnectorError::MissingRequiredField {
        field_name: "config_key in connector_meta_data",
    })?;
```

### Why It Works:
- Uses `connector_meta_data` for configuration that varies per merchant
- Provides clear error messages when configuration is missing
- Follows the chain of Option operations with proper error handling

### Example: Spreedly Gateway Token
```rust
// Spreedly requires a gateway token to route transactions
let gateway_token = req
    .connector_meta_data
    .as_ref()
    .and_then(|meta| meta.peek().as_object())
    .and_then(|obj| obj.get("gateway_token"))
    .and_then(|token| token.as_str())
    .ok_or(errors::ConnectorError::MissingRequiredField {
        field_name: "gateway_token in connector_meta_data",
    })?;
```

## Pattern: Transaction Token Management

### Context:
When a connector returns transaction-specific tokens that must be used for subsequent operations (capture, refund, sync).

### Standard Implementation:
```rust
// Store transaction token in response
PaymentsResponseData::TransactionResponse {
    resource_id: ResponseId::ConnectorTransactionId(
        connector_response.transaction.token.clone()
    ),
    // ... other fields
}

// Retrieve for subsequent operations
// For Capture:
let transaction_token = req.request.connector_transaction_id.clone();

// For Sync:
let transaction_token = req.request
    .get_connector_transaction_id()
    .change_context(errors::ConnectorError::MissingConnectorTransactionID)?;

// For Refund:
let transaction_token = req.request.connector_transaction_id.clone();
```

### URL Construction Pattern:
```rust
// Capture: /transactions/{token}/capture
// Refund: /transactions/{token}/credit  
// Sync: /transactions/{token}
Ok(format!(
    "{}/v1/transactions/{}/capture.json",
    self.base_url(connectors),
    transaction_token
))
```

### Why It Works:
- Consistently stores connector-specific identifiers
- Uses appropriate retrieval methods for different flows
- Maintains clear URL patterns for token-based operations

## Pattern: Flat Request/Response Structures

### Context:
Some connectors have simple, flat API structures rather than deeply nested objects.

### Standard Implementation:
```rust
// Simple, flat request structure
#[derive(Debug, Serialize)]
pub struct ConnectorPaymentsRequest {
    transaction: ConnectorTransaction,
}

#[derive(Debug, Serialize)]
pub struct ConnectorTransaction {
    amount: StringMinorUnit,
    currency_code: String,
    // Direct fields, not nested objects
    card_number: cards::CardNumber,
    card_exp_month: Secret<String>,
    card_exp_year: Secret<String>,
}
```

### Why It Works:
- Reduces complexity when the API doesn't require it
- Easier to maintain and understand
- Faster serialization/deserialization

### Anti-Pattern to Avoid:
```rust
// Overly complex structure when not needed
pub struct Request {
    data: RequestData {
        payment: PaymentData {
            card: CardData {
                details: CardDetails { /* ... */ }
            }
        }
    }
}
```

## Pattern: Name Splitting for Separate First/Last Fields

### Context:
When a connector requires separate first and last name fields but Hyperswitch provides a combined cardholder name.

### Standard Implementation:
```rust
// Split cardholder name into first and last
first_name: card_holder_name.clone()
    .and_then(|name| {
        let parts: Vec<&str> = name.peek().split_whitespace().collect();
        parts.first().map(|s| Secret::new(s.to_string()))
    })
    .unwrap_or_else(|| Secret::new("".to_string())),

last_name: card_holder_name
    .and_then(|name| {
        let parts: Vec<&str> = name.peek().split_whitespace().collect();
        if parts.len() > 1 {
            Some(Secret::new(parts[1..].join(" ")))
        } else {
            None
        }
    })
    .unwrap_or_else(|| Secret::new("".to_string())),
```

### Why It Works:
- Handles single names gracefully (empty last name)
- Handles multiple spaces (joins remaining parts for last name)
- Provides sensible defaults (empty string) when name is missing
- Maintains Secret wrapper for sensitive data

## Pattern: HMAC-SHA256 Webhook Verification

### Context:
Many connectors use HMAC-SHA256 for webhook signature verification with a straightforward pattern.

### Standard Implementation:
```rust
fn verify_webhook_source(
    &self,
    request: &webhooks::IncomingWebhookRequestDetails<'_>,
    connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
) -> CustomResult<(), errors::ConnectorError> {
    // Extract signature from header
    let signature_header = request
        .headers
        .get("x-connector-signature") // Connector-specific header name
        .ok_or(errors::ConnectorError::WebhookSourceVerificationFailed)?;
    
    // Get webhook secret
    let webhook_secret = connector_webhook_secrets
        .secret
        .as_ref()
        .ok_or(errors::ConnectorError::WebhookVerificationSecretNotFound)?;
    
    // Compute expected signature
    let expected_signature = crypto::HmacSha256::sign_message(
        webhook_secret.expose().as_bytes(),
        request.body.as_bytes(), // Raw request body
    )
    .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;
    
    // Encode to hex string
    let expected_signature_str = hex::encode(expected_signature);
    
    // Compare signatures
    if signature_header.as_bytes() != expected_signature_str.as_bytes() {
        return Err(errors::ConnectorError::WebhookSourceVerificationFailed.into());
    }
    
    Ok(())
}
```

### Why It Works:
- Uses raw request body for signature computation
- Handles hex encoding of the signature
- Provides clear error messages for missing headers/secrets
- Uses constant-time comparison (though as_bytes() comparison may not be)

## Pattern: Multi-Field Status Determination

### Context:
When a connector's status depends on multiple response fields rather than a single status field.

### Standard Implementation:
```rust
// Determine status based on transaction type + success flag
let status = match response.transaction_type.as_str() {
    "Authorize" => {
        if response.succeeded {
            common_enums::AttemptStatus::Authorized
        } else {
            common_enums::AttemptStatus::Failure
        }
    }
    "Capture" => {
        if response.succeeded {
            common_enums::AttemptStatus::Charged
        } else {
            common_enums::AttemptStatus::CaptureFailed
        }
    }
    "Void" => {
        if response.succeeded {
            common_enums::AttemptStatus::Voided
        } else {
            common_enums::AttemptStatus::VoidFailed
        }
    }
    _ => {
        if response.succeeded {
            common_enums::AttemptStatus::Pending
        } else {
            common_enums::AttemptStatus::Failure
        }
    }
};
```

### Why It Works:
- Provides precise status mapping based on operation context
- Handles unknown transaction types with a fallback
- Clearly separates success and failure cases for each operation

### Alternative Pattern (Single Field):
```rust
// When connector has a single status field
let status = match response.status.as_str() {
    "AUTHORIZED" => common_enums::AttemptStatus::Authorized,
    "CAPTURED" => common_enums::AttemptStatus::Charged,
    "FAILED" => common_enums::AttemptStatus::Failure,
    _ => common_enums::AttemptStatus::Pending,
};
```

# Active Context

## Current Focus

- Airwallex connector: Finalizing implementation by resolving compilation errors and warnings.

## Recent Changes (21/05/2025)

- **Airwallex Connector Compilation Fixes**:
    - **`transformers.rs`**:
        - Corrected `AirwallexPaymentsCaptureRequest.amount` field type from `Option<common_utils::types::StringMinorUnit>` to `Option<String>`.
        - Updated `TryFrom<&PaymentsCaptureRouterData> for AirwallexPaymentsCaptureRequest` to use `crate::utils::to_currency_base_unit(item.request.amount_to_capture, item.request.currency)` for amount conversion, resolving `E0308` (mismatched types for `StringMinorUnit::from(i64)`).
    - **`airwallex.rs`**:
        - Modified `get_request_body` in `ConnectorIntegration<AccessTokenAuth, ...>` to return `RequestContent::Json(Box::new(AirwallexAuthUpdateRequest {}))` instead of `RequestContent::Empty`, aligning with the expectation of an empty JSON body for login.
        - Corrected `handle_response` in `ConnectorIntegration<AccessTokenAuth, ...>` to use `..data.clone()` for `RefreshTokenRouterData` initialization and removed the erroneous `status: common_enums::AttemptStatus::Tokenized` field.
        - Added a placeholder implementation for `ConnectorRedirectResponse` trait for the `Airwallex` struct to satisfy `Connector` trait bounds. This involved:
            - Importing `ConnectorRedirectResponse` from `hyperswitch_interfaces::api`.
            - Importing `CallConnectorAction` and `PaymentAction` from `common_enums::enums`.
            - Implementing `get_flow_type` method with the correct signature and return type (`CustomResult<CallConnectorAction, errors::ConnectorError>`).
    - **`router/src/types/api.rs`**:
        - Corrected the instantiation of `Airwallex` connector in `ConnectorData::convert_connector` from `Box::new(&connector::Airwallex)` to `Box::new(connector::Airwallex::new())` to call the constructor, resolving `E0423`.
- **Build Process**:
    - Ran `cargo build` multiple times to identify and fix errors iteratively.
    - Attempted `cargo fix --lib -p hyperswitch_connectors` to address unused import warnings (output capture issues prevented confirmation of fixes).

## Next Steps

- Run `cargo build` to confirm all compilation errors are resolved and to check the status of warnings.
- If warnings persist, manually remove unused imports from `airwallex.rs` and `airwallex/transformers.rs`.
- Update `memory-bank/progress.md`.
- Compare the final code with `real-codebase/airwallex/` for any conceptual differences or missed patterns, and document these in `guides/patterns/patterns.md` or `guides/learnings/learning.md`.
- Inform the user about the successful compilation and the next steps (testing).

## Key Decisions & Considerations

- **`StringMinorUnit` vs. `String` for Amounts**: For connector request structs like `AirwallexPaymentsCaptureRequest`, if the API expects a string representation of a minor unit, the field should be `Option<String>`, and `crate::utils::to_currency_base_unit` should be used for conversion from `i64`.
- **`ConnectorRedirectResponse` Implementation**: Understanding the exact signature and purpose of traits like `ConnectorRedirectResponse` by reading their definitions in `hyperswitch_interfaces` is crucial. Placeholder implementations should match the trait definition to avoid further compilation errors.
- **Build Command Output**: Persistent issues with capturing output for long-running commands like `cargo build` and `cargo fix`. Proceeding with assumptions and asking for user-provided output when necessary.

## Important Patterns & Preferences

- **Iterative Debugging**: `cargo build` -> analyze error -> fix -> `cargo build` loop.
- **Trait Implementation**: Ensure all required traits for a `Connector` (like `ConnectorRedirectResponse`) are implemented, even if initially as placeholders, to satisfy trait bounds.

## Learnings & Insights (Airwallex Specific - New from this session)

- **`StringMinorUnit` Construction**: There's no direct public constructor `StringMinorUnit::from(i64)`. For amounts in request structs that need to be `StringMinorUnit` (or more commonly, `String` representing minor units), use utilities like `crate::utils::to_currency_base_unit`.
- **`ConnectorRedirectResponse` Trait**:
    - Defines `get_flow_type(&self, _query_params: &str, _json_payload: Option<serde_json::Value>, _action: common_enums::enums::PaymentAction) -> CustomResult<common_enums::enums::CallConnectorAction, errors::ConnectorError>`.
    - Does *not* include `get_connector_redirect_response`. This method is likely part of a specific flow's `ConnectorIntegration` (e.g., `PaymentsCompleteAuthorize`).
    - `PaymentAction` and `CallConnectorAction` enums are located in `common_enums::enums`.
- **Connector Instantiation in Router**: When a connector (e.g., `Airwallex`) has a `new()` constructor, it must be called (e.g., `connector::Airwallex::new()`) when being boxed in `ConnectorEnum` in `crates/router/src/types/api.rs`.
