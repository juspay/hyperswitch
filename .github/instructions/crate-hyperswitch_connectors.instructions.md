# Connector Code Review Guidelines

---

name: API Crate Guidelines
applyTo: "crates/hyperswitch*connectors/\**/\_"

---

## Purpose

This document provides code review guidelines and standards for connectors in the `hyperswitch_connectors` crate. Use these guidelines to ensure PII compliance, proper conversions, security best practices, and consistent code quality.

## Crate Context

The `hyperswitch_connectors` crate is specifically designed to write objects and transformers for various payment processors, converting them to Hyperswitch types while maintaining:

- **PII compliance** - Proper masking of sensitive data
- **Type safety** - Proper conversions and enums
- **Security** - No accidental exposure of critical data

### File Structure

Connectors typically consist of:

- **`{connector_name}.rs`** - Implements payment flow traits (authorize, psync, refunds)
- **`{connector_name}/transformers.rs`** - Contains request/response objects according to payment processor documentation and mappings/conversions from Hyperswitch types

Transformers map different `crates::hyperswitch_domain_models::PaymentMethodData` variants (cards, wallets, etc.) that the processor supports.

---

## Review Output Template

Use the following structure when providing code review feedback:

### Issue Severity Levels

| Symbol | Level      | Description                                                           |
| ------ | ---------- | --------------------------------------------------------------------- |
| 🚨     | Critical   | Must fix before proceeding - security, data leaks, or breaking issues |
| ⚠️     | Warning    | Should fix - code quality, performance, or maintainability issues     |
| 💡     | Suggestion | Nice to have - improvements and optimizations                         |
| ✨     | Success    | Positive patterns worth highlighting and reusing                      |

### Critical Issues Template

````markdown
### 🚨 Critical Issues (Must Fix Before Proceeding)

#### CRITICAL-[N]: [Issue Title]

**Category:** RUST_BEST_PRACTICE | SECURITY | PII_COMPLIANCE | TYPE_SAFETY
**Location:** `file_path:line_number`

**Problem:**
[Clear description of what is wrong]

**Code Example:**

```rust
// Current problematic code
[code snippet]
```
````

**Why This Is Critical:**
[Explanation of why this must be fixed]

**Required Fix:**

```rust
// Correct implementation
[fixed code snippet]
```

````

### Warning Issues Template

```markdown
### ⚠️ Warning Issues (Should Fix)

#### WARNING-[N]: [Issue Title]

**Category:** CODE_QUALITY | CONNECTOR_PATTERN | PERFORMANCE | MAINTAINABILITY
**Location:** `file_path:line_number`

**Problem:**
[Description of the suboptimal pattern]

**Current Code:**
```rust
[code snippet]
````

**Recommended Improvement:**

```rust
[improved code snippet]
```

**Impact:**
[What improves if this is fixed]

**References:**

- See: [relevant documentation or PR]

````

### Suggestions Template

```markdown
### 💡 Suggestions (Nice to Have)

#### SUGGESTION-[N]: [Issue Title]

**Category:** DOCUMENTATION | TESTING_GAP | CODE_ORGANIZATION
**Location:** `file_path:line_number`

**Suggestion:**
[What could be improved]

**Benefit:**
[Why this would be beneficial]
````

### Success Patterns Template

````markdown
### ✨ Success Patterns Observed

#### SUCCESS-[N]: [What Was Done Well]

**Category:** [Category]
**Location:** `file_path:line_number`

**Pattern:**

```rust
[example of good code]
```
````

**Why This Is Good:**
[Explanation of what makes this excellent]

**Reusability:**
[Can this pattern be applied elsewhere?]

````

### Code Quality Checklist

```markdown
### Code Quality Checklist

- [✅/❌] No code duplication
- [✅/❌] Proper error handling
- [✅/❌] No unnecessary unwrap()
- [✅/❌] Consistent naming conventions
- [✅/❌] Adequate documentation
- [✅/❌] Efficient transformations
- [✅/❌] PII data properly masked
- [✅/❌] Correct amount types used
````

---

## Critical Review Guidelines

### 01. MASKING PII DATA | 🚨 CRITICAL

**Rule:** All structs containing PII must use `Secret` wrapper to prevent accidental logging of sensitive data.

**What is PII:**

- Card numbers
- Customer names or any person names
- Physical addresses
- Passwords
- CVV/CVC
- Expiration dates
- Credentials
- Secret keys
- Email addresses
- Phone numbers

**Correct Implementation:**

```rust
use masking::Secret;
use common_utils::pii;

#[derive(Debug, Serialize, Deserialize)]
struct PaymentRequest {
    pub name: Secret<String>,           // Secret won't be logged
    pub email: pii::Email,              // PII type handles masking
    pub card_number: Secret<String>,    // Always mask card data
    pub amount: MinorUnit,              // Not PII, no masking needed
}
```

**Incorrect Implementation:**

```rust
// ❌ WRONG - PII will be logged
struct PaymentRequest {
    pub name: String,                   // Exposed in logs!
    pub email: String,                  // Exposed in logs!
    pub card_number: String,            // Major security issue!
}
```

---

### 02. USING PROPER AMOUNT TYPES | 🚨 CRITICAL

**Rule:** Never use `integer` or `float` for monetary amounts. Always use the appropriate amount type from `common_utils::types`.

**Available Amount Types:**

| Type              | Use Case                                                           |
| ----------------- | ------------------------------------------------------------------ |
| `MinorUnit`       | Connectors accepting minor unit as integer (e.g., cents as `1050`) |
| `StringMinorUnit` | Connectors accepting minor unit as string (e.g., `"1050"`)         |
| `FloatMajorUnit`  | Connectors accepting major unit as float (e.g., `10.50`)           |
| `StringMajorUnit` | Connectors accepting major unit as string (e.g., `"10.50"`)        |

**Correct Implementation:**

```rust
use common_utils::types::{MinorUnit, StringMinorUnit, FloatMajorUnit};

#[derive(Debug, Serialize, Deserialize)]
struct PaymentRequest {
    pub amount: MinorUnit,                      // For integer cents
    pub tax_amount: Option<StringMinorUnit>,    // For string cents
    pub fee: Option<FloatMajorUnit>,            // For decimal dollars
}
```

**Incorrect Implementation:**

```rust
// ❌ WRONG - Never use these for amounts
struct PaymentRequest {
    pub amount: i64,              // No! Use MinorUnit
    pub tax_amount: Option<f64>,  // No! Use FloatMajorUnit
    pub fee: String,              // No! Use StringMajorUnit
}
```

---

### 03. HANDLING WEBHOOK BODY | 🚨 CRITICAL

**Reference:** https://github.com/juspay/hyperswitch/issues/10633

**Rule:** The `get_webhook_resource_object` function must return a strongly-typed struct, never serialized strings or `serde_json::Value`, to prevent PII leakage in logs.

**Context:**

- Function signature: `fn get_webhook_resource_object(&self, request: &webhooks::IncomingWebhookRequestDetails<'_>) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError>`
- The returned object is stored and logged
- Must have all PII fields properly masked
- CVV/PIN must NEVER be included

**Incorrect Implementation:**

```rust
// ❌ WRONG - in {connector_name}.rs
fn get_webhook_resource_object(
    &self,
    request: &webhooks::IncomingWebhookRequestDetails<'_>,
) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
    let body_str = std::str::from_utf8(request.body)
        .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;

    // ❌ WRONG: Stringifying may expose unmasked PII
    let details = serde_json::to_string(&details)
        .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;

    Ok(Box::new(details))
}
```

**Correct Implementation:**

```rust
// ✅ CORRECT - in transformers.rs
#[derive(Debug, Serialize, Deserialize)]
pub struct ConnectorNameWebhookResourceObject {
    pub name: Secret<String>,           // PII masked
    pub amount: MinorUnit,              // Proper amount type
    pub status: WebhookStatus,
    pub transaction_id: String,
    // Note: Never include CVV/PIN fields
}

impl ConnectorNameWebhookResourceObject {
    pub fn decode_from_url(body_str: &str) -> Result<Self, errors::ConnectorError> {
        serde_urlencoded::from_str(body_str)
            .map_err(|_| errors::ConnectorError::WebhookBodyDecodingFailed)
    }
}

// ✅ CORRECT - in {connector_name}.rs
fn get_webhook_resource_object(
    &self,
    request: &webhooks::IncomingWebhookRequestDetails<'_>,
) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
    let body_str = std::str::from_utf8(request.body)
        .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;

    // ✅ Returns strongly-typed struct with masked PII
    let details = transformers::ConnectorNameWebhookResourceObject::decode_from_url(body_str)?;

    Ok(Box::new(details))
}
```

**Additional Requirements:**

- ❌ Never store CVV/PIN in webhook objects
- ⚠️ Avoid `serde_json::Value` fields; use strongly-typed fields when possible
- ✅ All PII must use `Secret<T>` wrapper

---

### 04. CHECKING 3DS SUPPORT ONLY FOR CARDS | ⚠️ WARNING

**Rule:** 3DS support checks should only apply to card payment methods, not other payment methods like wallets.

**Problem:**
Some connectors don't support 3DS for cards. However, the 3DS check should only throw an error when the payment method is a card. For other payment methods (wallets, bank transfers, etc.), the 3DS flag is irrelevant and should be ignored.

**Incorrect Implementation:**

```rust
// ❌ WRONG - Checking 3DS before determining payment method type
impl TryFrom<&ConnectorPayRouterData<&PaymentsAuthorizeRouterData>> for ConnectorPaymentsRequest {
    type Error = Error;

    fn try_from(
        item: &ConnectorPayRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        // ❌ WRONG: This blocks ALL payment methods, not just cards
        if item.router_data.is_three_ds() {
            return Err(errors::ConnectorError::NotSupported {
                message: "3DS flow".to_string(),
                connector: "ConnectorName",
            }
            .into());
        }

        let payment_method = match &item.router_data.request.payment_method_data {
            // ... rest of implementation
        };

        Ok(Self { payment_method })
    }
}
```

**Correct Implementation:**

```rust
// ✅ CORRECT - Check 3DS only within card payment method handling
impl TryFrom<&ConnectorPayRouterData<&PaymentsAuthorizeRouterData>> for ConnectorPaymentsRequest {
    type Error = Error;

    fn try_from(
        item: &ConnectorPayRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let payment_method = match &item.router_data.request.payment_method_data {
            payment_method_data::PaymentMethodData::Card(card) => {
                // ✅ CORRECT: Check 3DS only for cards
                if item.router_data.is_three_ds() {
                    return Err(errors::ConnectorError::NotSupported {
                        message: "3DS flow".to_string(),
                        connector: "ConnectorName",
                    }
                    .into());
                }

                requests::PaymentMethodData::Card(requests::Card {
                    number: card.card_number.clone(),
                    expiry_month: card.card_exp_month.clone(),
                    expiry_year: card.get_card_expiry_year_2_digit()?,
                    cvv: card.card_cvc.clone(),
                })
            }

            // ✅ CORRECT: Other payment methods ignore 3DS flag
            payment_method_data::PaymentMethodData::Wallet(wallet_data) => {
                // 3DS doesn't apply to wallets, process normally
                requests::PaymentMethodData::Wallet(/* ... */)
            }

            payment_method_data::PaymentMethodData::BankTransfer(bank_data) => {
                // 3DS doesn't apply to bank transfers, process normally
                requests::PaymentMethodData::BankTransfer(/* ... */)
            }

            // ... other payment methods
        };

        Ok(Self {
            payment_method,
            // ... other fields
        })
    }
}
```

**Key Points:**

- ✅ Place 3DS check inside the `Card` match arm
- ✅ Use `item.router_data.is_three_ds()` trait method
- ✅ Other payment methods should process normally regardless of 3DS flag
- ⚠️ Only add this check if the connector truly doesn't support 3DS for cards

---

### 05. USE TRAIT FUNCTIONS IN ROUTERDATA | ⚠️ WARNING

**Rule:** Use trait functions defined on `RouterData` to access common data instead of manually extracting fields.

**Available Trait Functions:**

The `RouterData` type has several helpful trait methods for accessing common data:

- `get_billing_address()` - Extract billing address data
- `get_shipping_address()` - Extract shipping address data
- `get_optional_billing()` - Get optional billing address
- `get_optional_shipping()` - Get optional shipping address
- `is_three_ds()` - Check if 3DS is enabled
- `get_browser_info()` - Extract browser information
- `get_card_expiry_year_2_digit()` - Get 2-digit card expiry year
- `get_card_expiry_year_4_digit()` - Get 4-digit card expiry year
- `get_description()` - Get payment description
- `get_return_url()` - Get return URL after payment

**Benefits:**

- ✅ Consistent error handling
- ✅ Proper null/option handling
- ✅ Type safety
- ✅ Less boilerplate code
- ✅ Easier maintenance

**Correct Implementation:**

```rust
impl TryFrom<&ConnectorRouterData<&PaymentsAuthorizeRouterData>> for ConnectorPaymentRequest {
    type Error = Error;

    fn try_from(
        item: &ConnectorRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        // ✅ Use trait methods for common data extraction
        let billing_address = item.router_data.get_billing_address()?;
        let shipping_address = item.router_data.get_optional_shipping();
        let browser_info = item.router_data.get_browser_info()?;
        let return_url = item.router_data.get_return_url()?;

        Ok(Self {
            billing: billing_address,
            shipping: shipping_address,
            browser_info,
            return_url,
            // ... other fields
        })
    }
}
```

**Incorrect Implementation:**

```rust
// ❌ Avoid manually extracting fields when trait methods exist
impl TryFrom<&ConnectorRouterData<&PaymentsAuthorizeRouterData>> for ConnectorPaymentRequest {
    type Error = Error;

    fn try_from(
        item: &ConnectorRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        // ❌ Manual extraction is error-prone
        let billing_address = item.router_data.address.billing
            .clone()
            .ok_or(errors::ConnectorError::MissingRequiredField {
                field_name: "billing_address"
            })?;

        // ❌ Inconsistent error handling
        let shipping_address = item.router_data.address.shipping.clone();

        Ok(Self {
            billing: billing_address,
            shipping: shipping_address,
            // ... other fields
        })
    }
}
```

---

## Additional Best Practices

### Error Handling

- ✅ Use `change_context()` to provide meaningful error context
- ✅ Return appropriate `ConnectorError` types
- ❌ Avoid `unwrap()` or `expect()` - handle errors explicitly

### Code Organization

- ✅ Keep connector logic in `{connector_name}.rs`
- ✅ Keep transformers in `{connector_name}/transformers.rs`
- ✅ Use clear, descriptive struct names (e.g., `ConnectorNamePaymentRequest`)

### Documentation

- ✅ Document connector-specific quirks or requirements
- ✅ Add inline comments for complex transformations
- ✅ Reference connector documentation where relevant

---

## Summary

When reviewing connector code, prioritize:

1. **Security First**: PII masking, proper types, no data leaks
2. **Type Safety**: Correct amount types, strong typing over `Value`
3. **Patterns**: Use trait methods, follow connector patterns
4. **Quality**: Error handling, testing, documentation

Use the templates above to provide structured, actionable feedback.
