# Spreedly Connector Fixes Summary

## Date: 2025-01-26

### Issues Found and Fixed

1. **Credit Card Field Names Mismatch**
   - Changed `cvc` → `verification_value`
   - Changed `expiry_month` → `month`
   - Changed `expiry_year` → `year`
   - Changed single `name` field → split into `first_name` and `last_name`

2. **Name Splitting Logic Added**
   - Implemented logic to split cardholder name into first and last names
   - Handles edge cases:
     - Empty name: Both fields set to None
     - Single name: First name only, last name set to None
     - Multiple names: First word as first name, rest as last name

3. **Kept the `complete` Field**
   - Retained in the structure as it appears to be related to auto-capture functionality
   - Set based on `is_auto_capture()` method

### Notes for Future Implementation

1. **Refund Endpoint**
   - Documentation shows refund endpoint as: `/v1/transactions/{transaction_token}/credit.json`
   - NOT `/refund.json` as might be expected

### Build Status
✅ All changes compile successfully with `cargo build`

### Files Modified
- `crates/hyperswitch_connectors/src/connectors/spreedly/transformers.rs`
