# Issue: Unsupported PIX Payment Methods Incorrectly Appearing in SDK for Mandate Flows

## Problem Description

When creating a payment intent with a mandate (recurring/off-session) flow, certain PIX payment methods were incorrectly appearing in the SDK's payment methods list even though they were not configured to support mandate transactions.

### Specific Cases Observed

1. **PixQr appearing in SDK for `payment_type=new_mandate` with non-zero amount**
   - Expected: PixQr should NOT appear (only for non-mandate transactions)
   - Actual: PixQr was appearing in the SDK response
   - Request: `amount=10 BRL, payment_type=new_mandate`

2. **PixAutomaticoPush appearing in SDK for non-zero mandate amounts**
   - Expected: PixAutomaticoPush should only appear for 0-amount setup mandates
   - Actual: PixAutomaticoPush was appearing for non-zero mandate amounts
   - Request: `amount=10 BRL, setup_future_usage=OffSession`

## Root Cause Analysis

The bug was in the payment method filtering logic in `crates/router/src/core/payment_methods/cards.rs`, specifically in the `filter_payment_methods` function (lines 5414-5485).

### How the Bug Worked

The payment method filtering logic worked as follows:

```rust
// OLD LOGIC (BUGGY)
if is_mandate_flow {
    if amount == 0 {
        // Check [zero_mandates.supported_payment_methods]
        if supported_in_zero_mandates {
            context_values.push(PaymentType::SetupMandate);
        }
        // else: ← BUG: Falls through without adding any PaymentType context
    } else if supported_in_mandates {
        context_values.push(PaymentType::NewMandate);
    } else {
        // ← BUG: STILL ADDS NON-MANDATE CONTEXT!
        context_values.push(PaymentType::NonMandate);
    }
} else {
    context_values.push(PaymentType::NonMandate);
}
```

**The Critical Bug:**
When a payment method was NOT supported in the mandate configuration (`[mandates.supported_payment_methods]`), the code would still add `PaymentType::NonMandate` context and include the payment method in the response. This bypassed the mandate support validation entirely.

### Why This Happened

The logic treated unsupported methods as "non-mandate" instead of "unsupported in this flow". This meant:

- PixQr (not in `[mandates.supported_payment_methods]`):
  - Mandate request arrives
  - Method not found in mandate config
  - Falls through to `else` clause
  - Gets marked as `PaymentType::NonMandate` ← **WRONG!**
  - Included in response

- PixAutomaticoPush (not explicitly in `[mandates.supported_payment_methods]`):
  - Non-zero mandate request arrives
  - Method not found in non-zero mandate config
  - Falls through to `else` clause
  - Gets marked as `PaymentType::NonMandate` ← **WRONG!**
  - Included in response

## Impact

- Users could see payment methods in the SDK that their merchant hasn't configured for mandates
- Potential for confused customer experience if they select a payment method that ultimately fails
- Configuration-level restrictions were being ignored by the code logic

## Solution

The fix implements explicit validation: **if it's a mandate flow, the payment method MUST be in the corresponding mandate configuration, otherwise it is completely skipped**.

### How the Fix Works

```rust
// NEW LOGIC (FIXED)
if is_mandate_flow {
    let is_supported_for_zero_mandate = /* check [zero_mandates.supported_payment_methods] */;
    let is_supported_for_mandate = /* check [mandates.supported_payment_methods] */;

    // Skip payment method if it's not supported for the mandate type
    if is_zero_amount && !is_supported_for_zero_mandate {
        continue;  // ← Skip completely, don't add to response
    }
    if !is_zero_amount && !is_supported_for_mandate {
        continue;  // ← Skip completely, don't add to response
    }

    // Only add PaymentType context for supported methods
    if is_zero_amount && is_supported_for_zero_mandate {
        context_values.push(PaymentType::SetupMandate);
    } else if !is_zero_amount && is_supported_for_mandate {
        context_values.push(PaymentType::NewMandate);
    }
} else {
    context_values.push(PaymentType::NonMandate);
}
```

**Key Changes:**
1. Extract all support checks upfront
2. Use `continue` to skip the payment method entirely if not supported for the mandate type
3. Only add PaymentType context for explicitly supported methods
4. Eliminates the fallback to `NonMandate` for unsupported methods in mandate flows

### Behavior After Fix

| Scenario | Before | After |
|----------|--------|-------|
| PixQr + non-zero mandate | ❌ Shows | ✅ Hidden |
| PixQr + 0-amount non-mandate | ✅ Shows | ✅ Shows |
| PixAutomaticoPush + non-zero mandate (not configured) | ❌ Shows | ✅ Hidden |
| PixAutomaticoPush + non-zero mandate (configured in mandates) | ✅ Shows | ✅ Shows |
| PixAutomaticoPush + 0-amount mandate (configured in zero_mandates) | ✅ Shows | ✅ Shows |

## Files Changed

- `crates/router/src/core/payment_methods/cards.rs` - Updated mandate flow validation logic in `filter_payment_methods` function

## Testing Recommendations

1. **Test PixQr exclusion from mandates:**
   - Create payment intent with `setup_future_usage=OffSession` and amount > 0
   - Verify PixQr does NOT appear in SDK response

2. **Test PixAutomaticoPush for 0-amount mandates:**
   - Create payment intent with `setup_future_usage=OffSession` and amount = 0
   - Verify PixAutomaticoPush DOES appear in SDK response
   - Verify it's marked as SetupMandate

3. **Test PixAutomaticoPush for non-zero mandates (if configured):**
   - Ensure configuration includes it in `[mandates.supported_payment_methods]`
   - Create payment intent with `setup_future_usage=OffSession` and amount > 0
   - Verify PixAutomaticoPush appears only if configured

4. **Test non-mandate flows still work:**
   - Regular payment (no mandate) should show all configured non-mandate methods
   - Verify no regression in non-mandate payment flows
