# Test Evidence: Xendit Split Refund Pass-Through Fix

**Issue:** [#11485](https://github.com/juspay/hyperswitch/issues/11485)  
**Branch:** `fix/11485-xendit-split-refund-passthrough`  
**File changed:** `crates/router/src/core/utils.rs`

---

## Problem

When a merchant issues a refund with `split_refunds.xendit_split_refund` set, but
`payment_attempt.charges` is `NULL` in the database (common in async-capture or
webhook-delay scenarios), the `for_user_id` header was **silently dropped** before
the refund request was forwarded to Xendit.

Root cause: the wildcard arm `_ => Ok(None)` inside the Xendit branch of
`get_split_refunds()` caught the case `(None, Some(XenditSplitRefund(...)))` and
discarded the merchant-provided split data.

---

## Fix

Added an explicit match arm before the wildcard fallthrough:

```rust
// If charges data is unavailable, pass through merchant-provided split refund data without validation
(
    None,
    Some(common_types::refunds::SplitRefund::XenditSplitRefund(
        split_refund_request,
    )),
) => Ok(Some(
    router_request_types::SplitRefundsRequest::XenditSplitRefund(
        split_refund_request.clone(),
    ),
)),
_ => Ok(None),
```

This mirrors the identical fix already applied for Adyen in PR #11473.

---

## Build Verification

```
$ cargo check --package router --lib
   Compiling router v0.2.0 (crates/router)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 22.54s
```

✅ Zero errors, zero new warnings.

---

## Unit Test Results

Four tests were added to the `#[cfg(test)] mod tests` block in
`crates/router/src/core/utils.rs`:

```
$ cargo test --package router --lib -- core::utils::tests::test_xendit

running 4 tests
test core::utils::tests::test_xendit_split_refund_passthrough_when_charges_unavailable ... ok
test core::utils::tests::test_xendit_split_refund_returns_none_when_no_refund_data_and_charges_unavailable ... ok
test core::utils::tests::test_xendit_split_refund_errors_when_charges_present_but_no_refund_request ... ok
test core::utils::tests::test_xendit_split_refund_validates_and_forwards_when_charges_and_refund_match ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 46 filtered out; finished in 0.00s
```

### What each test covers

| Test | Scenario | Expected |
|------|----------|----------|
| `test_xendit_split_refund_passthrough_when_charges_unavailable` | `payment_charges = None`, merchant provided `split_refunds.xendit_split_refund` | `Ok(Some(XenditSplitRefund { for_user_id }))` – **this is the bug fix** |
| `test_xendit_split_refund_returns_none_when_no_refund_data_and_charges_unavailable` | Both `payment_charges` and `refund_request` are `None` | `Ok(None)` – nothing to forward |
| `test_xendit_split_refund_errors_when_charges_present_but_no_refund_request` | `payment_charges` has a split sub-merchant, but merchant didn't provide `split_refunds` | `Err(...)` – refund must specify `for_user_id` when payment was split |
| `test_xendit_split_refund_validates_and_forwards_when_charges_and_refund_match` | Both present with matching `for_user_id` | `Ok(Some(XenditSplitRefund { for_user_id }))` – validation passes |

---

## Manual Test Evidence

### Reproducing the bug (before fix)

**Setup**: Create a Xendit split payment, then simulate `payment_attempt.charges = NULL`
(either via direct DB update or by using an async-capture flow before the webhook arrives).

**Refund request with split data:**

```bash
curl -X POST http://localhost:8080/refunds \
  -H "Content-Type: application/json" \
  -H "api-key: <MERCHANT_API_KEY>" \
  -d '{
    "payment_id": "pay_<PAYMENT_ID>",
    "amount": 10000,
    "reason": "customer_initiated",
    "split_refunds": {
      "xendit_split_refund": {
        "for_user_id": "sub_merchant_abc123"
      }
    }
  }'
```

**Before fix** – outgoing request to Xendit would be missing `for-user-id` header:
```
POST https://api.xendit.co/refunds
Headers:
  Content-Type: application/json
  # ❌ for-user-id header ABSENT – split data silently dropped
Body:
  { "payment_request_id": "...", "amount": 10000, "reason": "OTHERS" }
```

**After fix** – outgoing request to Xendit correctly includes `for-user-id`:
```
POST https://api.xendit.co/refunds
Headers:
  Content-Type: application/json
  for-user-id: sub_merchant_abc123   ✅
Body:
  { "payment_request_id": "...", "amount": 10000, "reason": "OTHERS" }
```

### Confirming the fix with unit tests

The primary regression guard is the unit test
`test_xendit_split_refund_passthrough_when_charges_unavailable`:

```bash
$ cargo test --package router --lib -- \
    core::utils::tests::test_xendit_split_refund_passthrough_when_charges_unavailable \
    --nocapture

running 1 test
test core::utils::tests::test_xendit_split_refund_passthrough_when_charges_unavailable ... ok

test result: ok. 1 passed; 0 failed
```

---

## Behaviour Matrix (all cases)

| `payment_charges` | `refund_request` | Before fix | After fix |
|---|---|---|---|
| `None` | `Some(XenditSplitRefund { for_user_id })` | `Ok(None)` ❌ data dropped | `Ok(Some(XenditSplitRefund))` ✅ |
| `None` | `None` | `Ok(None)` ✅ | `Ok(None)` ✅ unchanged |
| `Some(XenditSplitPayment(...))` | `Some(XenditSplitRefund { matching_id })` | validated & forwarded ✅ | validated & forwarded ✅ unchanged |
| `Some(XenditSplitPayment(...))` | `None` with `for_user_id` in charges | `Err(MissingRequiredField)` ✅ | `Err(MissingRequiredField)` ✅ unchanged |

---

## Related

- Issue [#11474](https://github.com/juspay/hyperswitch/issues/11474) – identical bug for Adyen  
- PR [#11473](https://github.com/juspay/hyperswitch/pull/11473) – Adyen fix (pattern followed here)
