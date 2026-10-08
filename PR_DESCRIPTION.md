# Pull Request: Fix PIX Payment Methods Incorrectly Appearing in Mandate Flows

## Summary

This PR fixes a critical bug in payment method filtering where unsupported payment methods were incorrectly appearing in the SDK response for mandate (recurring/off-session) transactions.

## Problem

Payment methods that were not configured in the mandate configuration sections were still being included in the SDK response for mandate flows. Specifically:

- **PixQr** appeared in mandate transactions even though it's only meant for non-mandate flows
- **PixAutomaticoPush** appeared for non-zero mandate amounts even though it's only meant for 0-amount setup mandates

### Root Cause

The mandate flow validation logic in `filter_payment_methods()` used a fallback to `PaymentType::NonMandate` for methods not found in the mandate configuration, rather than excluding them entirely. This caused unsupported methods to pass through the filter.

## Solution

Implemented explicit mandate flow validation:

1. **Check mandate support upfront** - Verify if the payment method is in the appropriate mandate configuration before proceeding
2. **Skip unsupported methods** - Use `continue` to completely exclude payment methods not configured for the mandate type
3. **Only add PaymentType context for supported methods** - Eliminate fallback to NonMandate for unsupported methods in mandate flows
4. **Clear separation of concerns** - Zero-amount mandates and non-zero mandates have explicit separate paths

## Changes

### Modified Files

- `crates/router/src/core/payment_methods/cards.rs`

### What Changed

**Function:** `filter_payment_methods()` (lines 5414-5485)

**Before:**
```rust
if is_mandate_flow {
    payment_intent.map(|intent| intent.amount).map(|amount| {
        if amount == MinorUnit::zero() {
            if configs.zero_mandates.supported_payment_methods... {
                context_values.push(PaymentType::SetupMandate);
            }
            // Falls through without checking - BUG!
        } else if configs.mandates.supported_payment_methods... {
            context_values.push(PaymentType::NewMandate);
        } else {
            // Still includes the method! - BUG!
            context_values.push(PaymentType::NonMandate);
        }
    });
}
```

**After:**
```rust
if is_mandate_flow {
    let is_supported_for_zero_mandate = /* check zero_mandates config */;
    let is_supported_for_mandate = /* check mandates config */;

    // Skip if not supported
    if is_zero_amount && !is_supported_for_zero_mandate {
        continue;  // ← Explicitly skip
    }
    if !is_zero_amount && !is_supported_for_mandate {
        continue;  // ← Explicitly skip
    }

    // Only include if supported
    if is_zero_amount && is_supported_for_zero_mandate {
        context_values.push(PaymentType::SetupMandate);
    } else if !is_zero_amount && is_supported_for_mandate {
        context_values.push(PaymentType::NewMandate);
    }
}
```

## Key Improvements

1. **Correct filtering behavior** - Payment methods are now properly excluded if not configured for mandate flows
2. **Explicit validation** - Support checks are performed explicitly before proceeding
3. **Better maintainability** - Clear separation between zero-amount and non-zero mandate handling
4. **No fallback vulnerabilities** - Removes the problematic fallback to NonMandate

## Testing

### Manual Testing Performed

- [x] PixQr excluded from mandate responses
- [x] PixAutomaticoPush included only when configured
- [x] 0-amount mandate flows work correctly
- [x] Non-mandate flows unaffected
- [x] Merchant configurations properly respected

### Test Cases to Verify

1. **Mandate flow exclusion:**
   ```
   POST /payments
   {
     "amount": 1000,
     "currency": "BRL",
     "setup_future_usage": "OffSession",
     "connector": "santander"
   }
   ```
   - PixQr should NOT appear
   - PixAutomaticoPush should NOT appear (unless configured in mandates)

2. **Zero-amount mandate inclusion:**
   ```
   POST /payments
   {
     "amount": 0,
     "currency": "BRL",
     "setup_future_usage": "OffSession",
     "connector": "santander"
   }
   ```
   - PixAutomaticoPush should appear (if in zero_mandates)
   - PixAutomaticoQr should appear (if in zero_mandates)

3. **Non-mandate flow (no regression):**
   ```
   POST /payments
   {
     "amount": 1000,
     "currency": "BRL",
     "connector": "santander"
   }
   ```
   - All non-mandate methods should appear normally

## Breaking Changes

None. This is a bug fix that makes the system behave as originally intended.

## Migration Guide

No migration needed. This fix corrects the filtering logic without changing APIs or configurations.

## Related Issues

Fixes: Unsupported PIX payment methods appearing in SDK for mandate flows

## Checklist

- [x] Code follows style guidelines
- [x] No breaking changes
- [x] Includes proper error handling
- [x] Tested manually
- [x] Clear commit messages
- [x] Updated code comments where necessary

## Questions for Reviewers

1. Is the separation between zero-amount and non-zero mandate handling clear?
2. Should we add logging for when payment methods are skipped due to mandate configuration?
3. Are there any edge cases related to amount calculation (net vs gross) we should consider?

## Deployment Notes

- Safe to deploy with no database migrations
- No configuration changes required
- Existing configurations will work correctly with this fix
