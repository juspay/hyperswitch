# Common Issues and Solutions Guide for Hyperswitch Connector Integration

This guide documents common issues encountered during connector integration and their solutions. These patterns apply to most connector implementations and can help developers avoid common pitfalls.

## Table of Contents
1. [Import and Trait Issues](#import-and-trait-issues)
2. [Error Response Structure Mismatches](#error-response-structure-mismatches)
3. [Authentication Implementation](#authentication-implementation)
4. [Build and Compilation Issues](#build-and-compilation-issues)
5. [Request and Response Structure Patterns](#request-and-response-structure-patterns)
6. [Metadata and Configuration Handling](#metadata-and-configuration-handling)
7. [Best Practices](#best-practices)

## Import and Trait Issues

### Problem 1: Missing Trait Imports
**Symptom**: Methods like `encode()` or `peek()` not found even though they should be available.

```rust
// Error example:
error[E0599]: no method named `encode` found for struct `GeneralPurpose`
error[E0599]: no method named `peek` found for struct `Secret`
```

**Root Cause**: Rust requires traits to be in scope to use their methods, even if the type implements the trait.

**Solution**: Always import the required traits:
```rust
// For base64 encoding
use base64::Engine;

// For masking operations
use masking::{PeekInterface, ExposeInterface, Mask};
```

**Generic Pattern**: When using external crate methods, check if they're provided by traits and import those traits.

### Problem 2: Unused Imports
**Symptom**: Compiler warnings about unused imports.

```rust
warning: unused import: `ExposeInterface`
```

**Solution**: Remove unused imports or use more specific imports:
```rust
// Instead of
use masking::{ExposeInterface, Mask, PeekInterface};

// Use only what's needed
use masking::{Mask, PeekInterface};
```

### Problem 3: Missing Extension Trait Imports
**Symptom**: Methods on common types not found even though they should be available.

```rust
// Error examples:
error[E0599]: no method named `parse_value` found for reference `&Secret<serde_json::Value>`
error[E0599]: no method named `change_context` found for enum `Result`
```

**Root Cause**: Extension traits that add methods to existing types must be imported.

**Solution**: Import the necessary extension traits:
```rust
// For parsing JSON values
use common_utils::ext_traits::ValueExt;

// For error handling with error-stack
use error_stack::ResultExt;
```

**Common Extension Traits to Remember**:
- `ValueExt` - Adds parsing methods to JSON values
- `ResultExt` - Adds error context methods
- `BytesExt` - Adds byte manipulation methods
- `StringExt` - Adds string manipulation methods

## Error Response Structure Mismatches

### Problem: Boilerplate vs Actual API Structure
**Symptom**: Field access errors when parsing error responses.

```rust
// Error example:
error[E0609]: no field `code` on type `SpreedlyErrorResponse`
error[E0609]: no field `message` on type `SpreedlyErrorResponse`
```

**Root Cause**: The boilerplate generator creates a generic error structure that doesn't match the actual connector's API response format.

**Solution Process**:
1. **Research the actual API error format** from the connector's documentation
2. **Update the error structures** to match:

```rust
// Generic boilerplate (often incorrect)
pub struct ConnectorErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}

// Actual connector format (example: array of errors)
pub struct ConnectorErrorResponse {
    pub errors: Vec<ConnectorError>,
}

pub struct ConnectorError {
    pub attribute: Option<String>,
    pub key: String,
    pub message: String,
}
```

3. **Update the error parsing logic** accordingly:

```rust
// Adapt to handle arrays, nested structures, etc.
let message = response.errors
    .iter()
    .map(|e| e.message.clone())
    .collect::<Vec<_>>()
    .join("; ");
```

**Generic Pattern**: Always verify the actual API response format before implementing error handling.

## Authentication Implementation

### Problem 1: Credential Format Parsing
**Symptom**: Authentication fails because credentials aren't parsed correctly.

**Common Patterns**:
1. **Colon-separated credentials**: `"key:secret"`
2. **JSON credentials**: `{"api_key": "xxx", "secret": "yyy"}`
3. **Single API key**: Just one credential value
4. **Multiple header requirements**: Multiple auth headers needed

**Solution Template**:
```rust
impl TryFrom<&ConnectorAuthType> for YourAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::HeaderKey { api_key } => {
                // For colon-separated format
                let api_key_str = api_key.peek();
                let parts: Vec<&str> = api_key_str.split(':').collect();
                
                if parts.len() != 2 {
                    return Err(errors::ConnectorError::FailedToObtainAuthType.into());
                }
                
                Ok(Self {
                    field1: Secret::new(parts[0].to_string()),
                    field2: Secret::new(parts[1].to_string()),
                })
            }
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}
```

### Problem 2: Auth Header Construction
**Common Auth Types**:
1. **HTTP Basic Auth**: Base64 encoded "username:password"
2. **Bearer Token**: "Bearer <token>"
3. **API Key Headers**: Custom header with API key
4. **Multiple Headers**: Multiple auth-related headers

**Solution Templates**:

```rust
// HTTP Basic Auth
let auth_string = format!("{}:{}", username.peek(), password.peek());
let encoded = common_utils::consts::BASE64_ENGINE.encode(auth_string.as_bytes());
let auth_header = format!("Basic {}", encoded);

// Bearer Token
let auth_header = format!("Bearer {}", token.peek());

// Custom API Key Header
headers.push(("X-API-Key".to_string(), api_key.peek().into_masked()));
```

## Build and Compilation Issues

### Problem: Feature Flag Confusion
**Symptom**: Build fails with feature-related errors.

**Solution**: Always use plain `cargo build` without feature flags for connector development:
```bash
# Correct
cargo build

# Avoid
cargo build --features some_feature
```

### Problem: Import Path Resolution
**Common Issues**:
1. Using wrong crate prefixes
2. Module visibility issues
3. Re-export confusion

**Solutions**:
- Use fully qualified paths when in doubt
- Check if types are re-exported in the crate root
- Verify module declarations in parent modules

## Request and Response Structure Patterns

### Problem 1: Nested Transaction Structures
**Symptom**: Payment APIs often wrap the actual transaction data in a parent object.

**Common Pattern**:
```rust
// Request often looks like:
{
  "transaction": {
    "amount": 1000,
    "currency": "USD",
    "payment_method": { ... }
  }
}

// Not just:
{
  "amount": 1000,
  "currency": "USD",
  "payment_method": { ... }
}
```

**Solution**: Create wrapper structures:
```rust
#[derive(Serialize)]
pub struct PaymentRequest {
    pub transaction: Transaction,
}

#[derive(Serialize)]
pub struct Transaction {
    pub amount: StringMinorUnit,
    pub currency_code: String,
    pub credit_card: CreditCard,
}
```

### Problem 2: Response Structure Verification
**Symptom**: Boilerplate response structures may not match actual API responses.

**Solution Process**:
1. **Check existing structures** - The boilerplate may have already created response structs
2. **Verify against API docs** - Ensure fields match the actual API response
3. **Update if needed** - Modify structures to match reality

```rust
// Check if this exists and matches API:
pub struct PaymentsResponse {
    pub transaction: TransactionResponse,  // Often nested
    // or
    pub status: PaymentStatus,            // Sometimes flat
    pub id: String,
}
```

**Real Example - Spreedly Implementation**:
- The boilerplate created a flat response structure with `status` and `id`
- The actual API returns a nested structure with a `transaction` object
- Solution: Created `SpreedlyTransactionResponse` and updated `SpreedlyPaymentsResponse` to use it
- Remember to update the TryFrom implementation to extract data from the nested structure

### Problem 3: Field Name Mismatches
**Common Issues**:
- API uses `currency_code` but struct has `currency`
- API returns `transaction_id` but struct expects `id`
- Different casing conventions (camelCase vs snake_case)

**Solution**: Use serde attributes:
```rust
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]  // or "snake_case"
pub struct Response {
    #[serde(rename = "transaction_id")]
    pub id: String,
    #[serde(rename = "currency_code")]
    pub currency: String,
}
```

## Metadata and Configuration Handling

### Problem 1: Extracting Values from Connector Metadata
**Symptom**: Need to parse configuration values from connector metadata (like gateway tokens).

```rust
error[E0507]: cannot move out of `*metadata` which is behind a shared reference
```

**Root Cause**: `parse_value` method takes ownership, but metadata is borrowed.

**Solution**: Clone before parsing:
```rust
pub fn get_gateway_token(
    connector_meta: &Option<common_utils::pii::SecretSerdeValue>,
) -> Result<String, error_stack::Report<errors::ConnectorError>> {
    let metadata = connector_meta
        .as_ref()
        .ok_or(errors::ConnectorError::InvalidConnectorConfig {
            config: "metadata",
        })?;
    
    // Clone before parsing to avoid ownership issues
    let parsed_metadata = metadata
        .clone()
        .parse_value::<serde_json::Value>("ConnectorMetadata")
        .change_context(errors::ConnectorError::InvalidConnectorConfig {
            config: "metadata",
        })?;
    
    // Extract the value
    let gateway_token = parsed_metadata
        .get("gateway_token")
        .and_then(|token| token.as_str())
        .ok_or(errors::ConnectorError::InvalidConnectorConfig {
            config: "gateway_token",
        })?;
    
    Ok(gateway_token.to_string())
}
```

### Problem 2: URL Construction with Dynamic Values
**Common Pattern**: APIs often require dynamic values in URLs (tokens, IDs, etc.)

**Solution**: Extract values early and format URLs:
```rust
fn get_url(
    &self,
    req: &PaymentsAuthorizeRouterData,
    connectors: &Connectors,
) -> CustomResult<String, errors::ConnectorError> {
    let base_url = self.base_url(connectors);
    let gateway_token = get_gateway_token(&req.connector_meta_data)?;
    Ok(format!("{}/v1/gateways/{}/authorize.json", base_url, gateway_token))
}
```

## Best Practices

### 1. Research Before Implementation
- **Read the connector's API documentation** thoroughly
- **Examine example requests/responses** from the API docs
- **Understand the authentication mechanism** before coding

### 2. Verify Existing Code
Before implementing new structures or methods:
1. **Check what the boilerplate generated** - It may already have basic structures
2. **Verify if traits are already implemented** - Look for existing empty implementations
3. **Review imports** - ResponseRouterData and other types may already be imported

### 3. Currency and Amount Handling
```rust
fn get_currency_unit(&self) -> api::CurrencyUnit {
    // Check API docs: does it accept cents (Minor) or dollars (Base)?
    api::CurrencyUnit::Minor  // Most common for payment processors
}
```

### 4. Error Handling Patterns
- Always check the actual error response format
- Handle both single errors and error arrays
- Provide meaningful error messages by combining multiple errors
- Map connector-specific error codes to generic ones when possible

### 5. Testing Approach
1. Start with `cargo build` to catch compilation errors
2. Fix imports and type mismatches
3. Implement minimal functionality that compiles
4. Add actual logic incrementally
5. Test with real API responses when possible

### 6. Common Gotchas
- **Serialization field names**: Check if the API uses `camelCase`, `snake_case`, or other formats
- **Optional vs Required fields**: Verify which fields are actually required by the API
- **Response status mapping**: Ensure correct mapping between connector statuses and Hyperswitch statuses
- **Timezone handling**: Be aware of timezone differences in timestamp fields
- **Character encoding**: Ensure proper UTF-8 handling for international characters
- **Ownership issues**: Clone values when methods require ownership but you only have a reference

### 7. Documentation Tips
- Document any deviations from standard patterns
- Note any specific requirements for the connector
- Include examples of actual API requests/responses
- Document test credentials and endpoints if available

## Debugging Checklist

When encountering build errors:

1. ✅ Check all trait imports are included (including extension traits)
2. ✅ Verify struct field names match API responses
3. ✅ Ensure authentication parsing matches the expected format
4. ✅ Confirm error response structure matches actual API
5. ✅ Remove unused imports
6. ✅ Use correct amount units (Minor vs Base)
7. ✅ Check serialization attributes match API format
8. ✅ Verify all required fields are included in requests
9. ✅ Ensure proper error type conversions
10. ✅ Test with `cargo build` (no feature flags)
11. ✅ Handle ownership issues (clone when needed)
12. ✅ Verify nested structure patterns in requests/responses

## Incremental Implementation Strategy

When implementing a connector, follow this order to minimize confusion:

1. **Phase 1: Setup and Authentication**
   - Get basic structure compiling
   - Implement authentication parsing and header construction
   - Verify error response handling

2. **Phase 2: Request Structures**
   - Build request types matching API documentation
   - Implement conversion traits
   - Handle nested structures properly

3. **Phase 3: Response Handling**
   - Verify/update response structures
   - Implement status mappings
   - Handle all response scenarios

4. **Phase 4: Edge Cases**
   - Add proper error handling
   - Implement optional features
   - Add comprehensive logging

## Connector-Specific Variations

While this guide covers common patterns, each connector may have unique requirements:

- **Webhook handling**: Some connectors require specific webhook verification
- **3D Secure flows**: Payment authentication may vary significantly
- **Tokenization**: Different approaches to storing payment methods
- **Refund windows**: Time limits on refund operations
- **Partial operations**: Support for partial captures/refunds
- **Metadata handling**: Custom fields and their limitations

Always refer to the specific connector's API documentation for these variations.

## Real-World Example: Maxpay Connector Issues and Solutions

Here are actual issues encountered during the Maxpay connector implementation:

### 1. StringMinorUnit Field Access
**Issue**: Attempted to access private field directly
```rust
// ❌ Wrong - private field
let amount_str = item.amount.0.clone();
```

**Solution**: Use JSON serialization as workaround
```rust
// ✅ Correct - serialize to get string value
let amount_str = serde_json::to_string(&item.amount)
    .change_context(errors::ConnectorError::RequestEncodingFailed)?
    .trim_matches('"')
    .to_string();
```

### 2. Missing PeekInterface Import
**Issue**: Method `peek()` not found on Email, Secret fields
```rust
// ❌ Error: no method named `peek` found
user_email: payment_data.email.as_ref().map(|email| email.peek().to_string()),
```

**Solution**: Import the trait
```rust
// ✅ Add to imports
use masking::{ExposeInterface, PeekInterface, Secret};
```

### 3. Secret Field Move Errors
**Issue**: Using `expose()` moves the value causing borrow checker errors
```rust
// ❌ Wrong - moves the value
user_first_name: billing_address.first_name.as_ref()
    .map(|name| name.expose().to_string()),
```

**Solution**: Use `peek()` for borrowing
```rust
// ✅ Correct - borrows the value
user_first_name: billing_address.first_name.as_ref()
    .map(|name| name.peek().to_string()),
```

### 4. Card Number Type Mismatch
**Issue**: Type mismatch between CardNumber and Secret<String>
```rust
// ❌ Wrong - type mismatch
card_number: card.card_number.clone(),
```

**Solution**: Use peek() and wrap in Secret
```rust
// ✅ Correct
card_number: Secret::new(card.card_number.peek().to_string()),
```

### 5. Authentication Placement
**Issue**: Initially assumed auth goes in headers
```rust
// ❌ Wrong assumption
fn get_auth_header(...) -> ... {
    Ok(vec![(
        headers::AUTHORIZATION.to_string(),
        auth.api_key.expose().into_masked(),
    )])
}
```

**Solution**: Maxpay sends auth in request body
```rust
// ✅ Correct - no auth headers, credentials in body
fn get_auth_header(...) -> ... {
    Ok(vec![])  // Empty headers
}

// Auth fields included in request struct
pub struct MaxpayAuthRequest {
    pub merchant_account: Secret<String>,
    pub merchant_password: Secret<String>,
    // ... other fields
}
```

### 6. Missing Trait for Helper Methods
**Issue**: Method not found even though it should exist
```rust
// ❌ Error: no method named `get_connector_transaction_id`
let reference = item.request.get_connector_transaction_id()
```

**Solution**: Import the trait that provides the method
```rust
// ✅ Add to imports
use crate::utils::PaymentsSyncRequestData;
```

### 7. Reference vs Value in TryFrom
**Issue**: Passing wrong reference level
```rust
// ❌ Wrong - double reference
let connector_req = maxpay::MaxpayCaptureRequest::try_from(&req)?;
```

**Solution**: Check the trait implementation signature
```rust
// ✅ Correct - matches TryFrom<&PaymentsCaptureRouterData>
let connector_req = maxpay::MaxpayCaptureRequest::try_from(req)?;
```

### 8. Amount Conversion Complexity
**Issue**: Complex conversion from StringMinorUnit to f64 major units
```rust
// The full conversion chain needed:
// StringMinorUnit -> String (via JSON) -> i64 -> major unit string -> f64
let amount_str = serde_json::to_string(&item.amount)?
    .trim_matches('"').to_string();
let amount_i64: i64 = amount_str.parse::<i64>()?;
let amount_str = utils::to_currency_base_unit(amount_i64, currency)?;
let amount: f64 = amount_str.parse::<f64>()?;
```

**Lesson**: Some connectors have complex amount requirements requiring multiple conversions.

### 9. Build Command Feature Flags
**Issue**: Using unnecessary feature flags
```bash
# ❌ Overcomplicated
cargo build --features="maxpay,v1"
```

**Solution**: Use simple build command
```bash
# ✅ Simple
cargo build
```

### 10. Phone Number Access Pattern
**Issue**: Not using the provided helper method
```rust
// ❌ Wrong - trying to access directly
user_phone: billing_address.phone.as_ref().map(|p| p.expose()),
```

**Solution**: Use the router data helper
```rust
// ✅ Correct - using helper method
user_phone: item.router_data.get_optional_billing_phone_number()
    .map(|phone| phone.expose()),
```

### Key Learnings from Maxpay Implementation

1. **Always check field visibility**: Many types have private fields requiring special access methods
2. **Import traits for methods**: Rust requires traits in scope to use their methods
3. **Prefer peek() over expose()**: Use `peek()` when you need to borrow Secret values
4. **Check authentication patterns**: Not all connectors use header-based auth
5. **Verify type conversions**: Amount conversions can be complex with multiple steps
6. **Use helper methods**: RouterData provides many helper methods for common operations
7. **Start simple with builds**: Don't add feature flags unless specifically needed
8. **Read error messages carefully**: They often hint at the solution (like suggesting trait imports)
