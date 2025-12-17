# connector code review

---

name: API Crate Guidelines
applyTo: "crates/hyperswitch_connectors/\*\*/\_"

---

Instruction for code improvements for connectors present in crate hyperswitch_connectors

## Context

- This crate is specifically used to write objects and transformers for various payment processors to hyperswitch types which needs to maintain PII complience , proper conversions, enums etc
- the crate usually contains `{connector_name}.rs` and `{connector_name}/transformers.rs`
- `{connector_name}.rs` : implements payment flows traits like authorize , psync , refunds.
- `{connector_name}/transformers.rs` : contains the request and response objects accroding to payment processor documentation and mappings/conversions from hyperswitch types.
- In transformers we usually map different `crates::hyperswitch_domain_models::PaymentMethodData` (like card, wallets) which the processor supports

## issue type

| 🚨 Critical |
| ⚠️ Warning |
| 💡 Suggestion |

### 🚨 Critical Issues (Must Fix Before Proceeding)

#### CRITICAL : [Issue Title]

**Category:** | RUST_BEST_PRACTICE | SECURITY | etc.
**Location:** `file_path:line_number`

**Problem:**

```
[Clear description of what is wrong]
```

**Code Example:**

```rust
// Current problematic code
[code snippet]
```

**Why This Is Critical:**
[Explanation of why this must be fixed]

**Required Fix:**

```rust
// Correct implementation
[fixed code snippet]
```

---

### ⚠️ Warning Issues (Should Fix)

#### WARNING]: [Issue Title]

**Category:** CODE_QUALITY | CONNECTOR_PATTERN | PERFORMANCE | etc.
**Location:** `file_path:line_number`

**Problem:**
[Description of the suboptimal pattern]

**Current Code:**

```rust
[code snippet]
```

**Recommended Improvement:**

```rust
[improved code snippet]
```

**Impact:**
[What improves if this is fixed]

**References:**

- See: [relevant documentation]

---

### 💡 Suggestions (Nice to Have) - Count: [N]

#### SUGGESTION-[N]: [Issue Title]

**Category:** DOCUMENTATION | TESTING_GAP | etc.
**Location:** `file_path:line_number`

**Suggestion:**
[What could be improved]

**Benefit:**
[Why this would be beneficial]

---

### ✨ Success Patterns Observed - Count: [N]

#### SUCCESS: [What Was Done Well]

**Category:** [Category]
**Location:** `file_path:line_number`

**Pattern:**

```rust
[example of good code]
```

**Why This Is Good:**
[Explanation of what makes this excellent]

**Reusability:**
[Can this pattern be applied elsewhere?]

---

#### Code Quality

- [✅/❌] No code duplication
- [✅/❌] Proper error handling
- [✅/❌] No unnecessary unwrap
- [✅/❌] Consistent naming conventions
- [✅/❌] Adequate documentation
- [✅/❌] Efficient transformations

---

# Instructions for Review and issue type and learnings from if any related pr.

-- code agents to ignore learnings from as it is just for dev doc

## 01. MASKING PII DATA | CRITICAL

- Any structs defined here must strictly be of `Secret` wrap all pii data . We log the request and response for debug purposes and it shouldn't accidently expose critical pii data.

eg

```rust

struct PaymentRequest {
 pub name:masking::Secret<String>, // Secret wont be logged
 pub email: pii::Email, //
 ..
}

```

- PII data includes Card numbers , Customer names or any person names , address , Passwords , CVV, Expiration date , Credentials , Secret keys etc

## 02. Using Proper Amount in request/response objects | CRITICAL

- All struct which contains any amount related data should never be `integer` or `float`. It should always be of struct `common_utils::types::StringMinorUnit` for connectors accepting minor unit in string integer format, `common_utils::types::FloatMajorUnit`for connectors accepting minor unit in float format , `common_utils::types::StringMajorUnit` for connectors accepting minor unit in string format, `common_utils::types::MinorUnit`for connectors accepting minor unit in integer format.

eg

```rust
struct PaymentRequest {
 pub tax_amount: Option<i64>, // Not aceepetable instead use MinorUnit
 ..
}
```

## 03. Handling webhook body | CRITICAL

### learnings from : https://github.com/juspay/hyperswitch/issues/10633

In {connector_name}.rs for the trait `hyperswitch_interfaces::webhooks::IncomingWebhook` we have a function fn get_webhook_resource_object which returns CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> . This return object is stored and it will be logged. the return object should follow the follwoing rule

- The object should be of strict type, not serialized or should be serde_json::Value as it may contain PII data which will be logged

eg :

Wrong example

```rust
 // {connector_name}.rs
   fn get_webhook_resource_object(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        let body_str = std::str::from_utf8(request.body)
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;

        let details =  serde_json::to_string(&details).change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?; // this is wrong as we are stringifying the body and it may result in unmasked logs of pii data
        Ok(Box::new(details))
    }
```

Correct format

```rust

 // in transformers.rs
#[derive(Debug, Serialize, Deserialize)]
pub struct {ConnectorName}WebhookResourceObject {
  pub name : Secret<String>,
  pub amount : MinorUnit,
  ... other fields
}
impl {ConnectorName}WebhookResourceObject {
    pub fn decode_from_url(body_str: &str) -> Result<Self, errors::ConnectorError> {
        serde_urlencoded::from_str(body_str)
            .map_err(|_| errors::ConnectorError::WebhookBodyDecodingFailed)
    }
}


 // // {connector_name}.rs
  ...
   fn get_webhook_resource_object(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        let body_str = std::str::from_utf8(request.body)
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        let details = transformers::{{ConnectorName}}WebhookKeyValueBody::decode_from_url(body_str)?;
        Ok(Box::new(details))
    }
```

- CVV/PINS should never be passed to this object as we should never store this

- Avoide the any fields in the struct `{ConnectorName}WebhookResourceObject` to be serde_json::Value with less critial and analyse if that is really needed.

## 04. Checking 3ds support only for cards | WARNING

some connectors doesn't support 3ds for cards and throw error only if payment_method_data is of type Card. For other payment methods its important we ignore if the type is 3ds or non 3ds as it

eg

```rust
impl TryFrom<&GlobalPayRouterData<&PaymentsAuthorizeRouterData>> for GlobalpayPaymentsRequest {
    type Error = Error;

    fn try_from(
        item: &{ConnectorName}PayRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
          // if item.router_data.is_three_ds() {
          //          return Err(errors::ConnectorError::NotSupported {
          //             message: "3DS flow".to_string(),
          //            connector: "Globalpay",
          //      }
          //         .into());
          //       }  // this shouldnt be on top level where the payment method data is not card.


        let payment_method = match &item.router_data.request.payment_method_data {
            payment_method_data::PaymentMethodData::Card(ccard) => {
                if item.router_data.is_three_ds() { // check 3ds by trait function
                    return Err(errors::ConnectorError::NotSupported {
                        message: "3DS flow".to_string(),
                        connector: "CONNECTOR_NAME",
                    }
                    .into());
                } // add only to cards
                requests::GlobalPayPaymentMethodData::Common(CommonPaymentMethodData {
                    payment_method_data: PaymentMethodData::Card(requests::Card {
                        number: ccard.card_number.clone(),
                        expiry_month: ccard.card_exp_month.clone(),
                        expiry_year: ccard.get_card_expiry_year_2_digit()?,
                        cvv: ccard.card_cvc.clone(),
                    }),
                    entry_mode: Default::default(),
                })
            }

            payment_method_data::PaymentMethodData::Wallet(wallet_data) => /// other implementations
        };

        Ok(Self {
          payment_method
       .... other
        })
    }
}
```

## 05. Use trait functions in crates::hyperswitch_connectors::RouterData | WARNING

There are several traits defined in RouterData and use this functions effectively get the following field, like billing address data, shipping address data
