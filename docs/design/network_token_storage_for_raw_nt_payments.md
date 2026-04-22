# Design: Store Network Token Data for Raw NT Payments

## Problem Statement

When a merchant sends a payment with `payment_method: network_token` (raw network token + cryptogram from their own VTS/MDES integration), and `setup_future_usage: off_session` is set, the system currently does NOT store the network token data (token number + expiry) in the locker or in `network_token_payment_method_data`.

This means future MIT (Merchant Initiated Transactions) cannot use the network token + NTI flow — they fall back to PSP mandate only.

## Current State

### What Works Today (NTS Flow — Card → Network Token)

When a customer pays with a **raw card** and `is_network_tokenization_enabled = true`:

```
Card payment succeeds
       │
       ▼
Call NTS service (make_card_network_tokenization_request)
  → sends: PAN + expiry → NTS
  ← gets back: token_number, token_expiry, card_reference
       │
       ▼
Save network token to LOCKER (as a card record)
  → locker returns: payment_method_id (locker handle)
       │
       ▼
Write to payment_methods table:
  locker_id                             = card's locker handle
  network_token_locker_id               = network token's locker handle
  network_token_requestor_reference_id  = card_reference from NTS
  network_token_payment_method_data     = encrypt(last4, expiry, card_network, ...)
  network_transaction_id                = NTI from connector response
```

### What's Broken (Raw NT Input Flow)

When a merchant sends `payment_method: network_token` directly:

```
Network Token payment succeeds
       │
       ▼
save_card_and_network_token_in_locker() is called
  → vault_operation is None for NetworkToken
  → falls into catch-all `_` branch
  → save_in_locker called with card: None
  → is_network_tokenization_enabled check:
       match payment_method_data {
           Card(card) => save_network_token_in_locker(...),  // only Card handled
           _ => Ok((..., None)),  // NetworkToken falls here → NO locker storage!
       }
       │
       ▼
Write to payment_methods table:
  locker_id                             = null (no card)
  network_token_locker_id               = null ❌
  network_token_requestor_reference_id  = null
  network_token_payment_method_data     = null ❌
  network_transaction_id                = NTI from connector ✅
```

**Only `network_transaction_id` is stored. The network token number + expiry are lost.**

## Proposed Solution

### Goal

Store the network token (number + expiry) in the **locker** and `network_token_payment_method_data`, following the exact same storage path as the NTS flow — but **without calling NTS** since the merchant already provided the token.

### What to Store vs What NOT to Store

| Field | Store? | Reason |
|-------|--------|--------|
| Network token number | ✅ Yes | Needed for future MIT payments |
| Token expiry month/year | ✅ Yes | Needed for future MIT payments |
| Card network (Visa/MC) | ✅ Yes | Routing + connector logic |
| Last4, card_isin | ✅ Yes | Display + dedup |
| Cryptogram | ❌ No | Single-use, consumed during this payment |
| ECI | ❌ No | Specific to this transaction's auth |
| NTI (network_transaction_id) | ✅ Yes | From connector response, stored already |

### Flow After Fix

```
Network Token payment succeeds
       │
       ▼
Convert NetworkTokenData → CardDetail (same as NTS response)
       │
       ▼
Save network token to LOCKER (as a card record — same as NTS flow)
  → locker returns: payment_method_id (locker handle)
       │
       ▼
Encrypt token metadata → pm_network_token_data_encrypted
       │
       ▼
Write to payment_methods table:
  locker_id                             = null (no PAN card)
  network_token_locker_id               = network token's locker handle ✅
  network_token_requestor_reference_id  = null (no NTS)
  network_token_payment_method_data     = encrypt(last4, expiry, card_network, ...) ✅
  network_transaction_id                = NTI from connector response ✅
```

### Future MIT Payment Flow

```
MIT payment request (off_session, mandate_id)
       │
       ▼
Fetch payment_method from DB
  → network_token_locker_id is present
  → Fetch token from locker: token_number + expiry
  → network_transaction_id is present
       │
       ▼
Construct CardWithNetworkTokenDetails:
  - card data from locker_id (if exists) OR network token data
  - network token: token_number + expiry
  - NTI: network_transaction_id
  - No cryptogram needed for MIT
       │
       ▼
Send to connector
```

---

## Code Changes — Pseudo Code

### Change 1: Handle `NetworkToken` in `get_payment_method_create_request`

**File:** `crates/router/src/core/payment_methods.rs` (~line 1005)

Currently NetworkToken falls into the catch-all `_` branch which sets `card: None`.

```
// ADD: Before the catch-all _ branch

domain::PaymentMethodData::NetworkToken(nt_data) => {
    // Convert network token to CardDetail for locker storage
    // (same as how NTS response is stored — network token is saved as a "card" in locker)
    let card_detail = payment_methods::CardDetail {
        card_number: nt_data.network_token.clone(),
        card_exp_month: nt_data.token_exp_month.clone(),
        card_exp_year: nt_data.token_exp_year.clone(),
        card_holder_name: nt_data.card_holder_name.clone(),
        nick_name: nt_data.nick_name.clone(),
        card_issuing_country: nt_data.card_issuing_country.clone(),
        card_issuing_country_code: None,
        card_network: nt_data.card_network.clone(),
        card_issuer: nt_data.card_issuer.clone(),
        card_type: nt_data.card_type.clone(),
        card_cvc: None,  // Never store cryptogram
    };

    PaymentMethodCreate {
        payment_method: Some(payment_method),
        payment_method_type,
        card: Some(card_detail),
        payment_method_data: None,
        ...
    }
}
```

### Change 2: Handle `NetworkToken` in `save_card_and_network_token_in_locker`

**File:** `crates/router/src/core/payments/tokenization.rs` (~line 2029)

Currently in the catch-all `_` branch, only `Card` variant triggers locker storage for network tokens.

```
// MODIFY: The is_network_tokenization_enabled block in catch-all branch

if is_network_tokenization_enabled {
    match &payment_method_data {
        domain::PaymentMethodData::Card(card) => {
            // EXISTING: Call NTS service, save network token to locker
            let (network_token_resp, _dc, network_token_requestor_ref_id) =
                Box::pin(save_network_token_in_locker(
                    state, platform, card,
                    None,  // triggers NTS call
                    payment_method_create_request.clone(),
                )).await?;
            Ok(((res, dc, network_token_requestor_ref_id), network_token_resp))
        }

        // ADD: NetworkToken case — save directly to locker without NTS call
        domain::PaymentMethodData::NetworkToken(nt_data) => {
            // Convert NetworkTokenData to CardDetail (same shape as NTS response)
            let network_token_card_detail = api::CardDetail {
                card_number: nt_data.network_token.clone(),
                card_exp_month: nt_data.token_exp_month.clone(),
                card_exp_year: nt_data.token_exp_year.clone(),
                card_cvc: None,
                card_holder_name: nt_data.card_holder_name.clone(),
                nick_name: nt_data.nick_name.clone(),
                card_issuing_country: nt_data.card_issuing_country.clone(),
                card_issuing_country_code: None,
                card_network: nt_data.card_network.clone(),
                card_issuer: nt_data.card_issuer.clone(),
                card_type: nt_data.card_type.clone(),
            };

            // Save network token to locker (Some = skip NTS call, save directly)
            let (network_token_resp, dc, _) = Box::pin(save_network_token_in_locker(
                state,
                platform,
                &dummy_card_from_nt,  // not used when network_token_data is Some
                Some(network_token_card_detail),
                payment_method_create_request.clone(),
            )).await?;

            // network_token_requestor_ref_id = None (no NTS service involved)
            Ok(((res, dc, None), network_token_resp))
        }

        _ => Ok(((res, dc, None), None)),
    }
}
```

### Change 3: Store `network_token_payment_method_data` for NetworkToken

**File:** `crates/router/src/core/payments/tokenization.rs` (~line 327)

Currently `optional_pm_details` only handles `Card`, `Wallet`, `BankDebit`. NetworkToken gets `None`.

```
// ADD: After the BankDebit match arm in optional_pm_details

(
    _,
    domain::PaymentMethodData::NetworkToken(nt_data),
) => {
    // Store network token metadata in payment_method_data column
    // (last4, expiry, card_network — same as what Card stores)
    Some(domain::PaymentMethodsData::NetworkToken(
        domain::NetworkTokenDetailsPaymentMethod {
            last4_digits: Some(nt_data.network_token.get_last4()),
            network_token_expiry_month: Some(nt_data.token_exp_month.clone()),
            network_token_expiry_year: Some(nt_data.token_exp_year.clone()),
            card_network: nt_data.card_network.clone(),
            card_isin: Some(nt_data.network_token.get_card_isin()),
            card_issuer: nt_data.card_issuer.clone(),
            card_issuing_country: nt_data.card_issuing_country.clone(),
            card_type: nt_data.card_type.clone(),
            nick_name: nt_data.nick_name.clone(),
            card_holder_name: nt_data.card_holder_name.clone(),
        },
    ))
}
```

### Change 4: Handle `network_token_locker_id` population

**File:** `crates/router/src/core/payments/tokenization.rs` (~line 316)

Currently `network_token_locker_id` is only set when `network_token_requestor_ref_id.is_some()`. For raw NT flow, `network_token_requestor_ref_id` is `None` (no NTS), but we still have a locker ID.

```
// MODIFY: The network_token_locker_id logic

let network_token_locker_id = match network_token_resp {
    Some(ref token_resp) => {
        // EXISTING: If NTS was used, network_token_requestor_ref_id must be present
        if network_token_requestor_ref_id.is_some() {
            Some(token_resp.payment_method_id.clone())
        }
        // ADD: If raw NT was saved to locker (no NTS), still set locker ID
        else if is_network_token_from_request {
            Some(token_resp.payment_method_id.clone())
        }
        else {
            None
        }
    }
    None => None,
};
```

**Simpler alternative:** Always set `network_token_locker_id` when `network_token_resp` exists, regardless of `network_token_requestor_ref_id`:

```
let network_token_locker_id = network_token_resp
    .as_ref()
    .map(|token_resp| token_resp.payment_method_id.clone());
```

Then `network_token_requestor_ref_id` being `None` is fine — it just means "no NTS was used".

### Change 5: Handle `locker_id` for NetworkToken payment method

**File:** `crates/router/src/core/payments/tokenization.rs` (~line 900)

Currently `locker_id` is only set for `Card` and `BankDebit`:

```
locker_id = resp.payment_method.and_then(|pm| {
    if pm == PaymentMethod::Card || pm == PaymentMethod::BankDebit {
        Some(resp.payment_method_id)
    } else {
        None
    }
});
```

For NetworkToken, the `locker_id` should remain `None` (no PAN card stored). The network token's locker handle goes into `network_token_locker_id` instead. **No change needed here.**

---

## Summary of DB Columns After Fix

| Column | NTS Flow (Card → NT) | Raw NT Flow (After Fix) |
|--------|---------------------|------------------------|
| `locker_id` | Card's locker handle | `null` |
| `network_token_locker_id` | NT's locker handle | NT's locker handle ✅ |
| `network_token_requestor_reference_id` | NTS card_reference | `null` |
| `network_token_payment_method_data` | Encrypted NT metadata | Encrypted NT metadata ✅ |
| `network_transaction_id` | NTI from connector | NTI from connector ✅ |
| `payment_method` | `card` | `network_token` |
| `payment_method_type` | `credit`/`debit` | `network_token` |

---

## Files to Modify

| # | File | Change |
|---|------|--------|
| 1 | `crates/router/src/core/payment_methods.rs` | Add `NetworkToken` arm in `get_payment_method_create_request` |
| 2 | `crates/router/src/core/payments/tokenization.rs` | Add `NetworkToken` arm in `save_card_and_network_token_in_locker` catch-all branch |
| 3 | `crates/router/src/core/payments/tokenization.rs` | Add `NetworkToken` in `optional_pm_details` match |
| 4 | `crates/router/src/core/payments/tokenization.rs` | Fix `network_token_locker_id` logic to not require `network_token_requestor_ref_id` |
| 5 | `crates/hyperswitch_domain_models/src/payment_methods.rs` | Verify `NetworkTokenDetailsPaymentMethod` has all needed fields |
| 6 | Tests | Add integration test for raw NT + `setup_future_usage: off_session` |

---

## Open Questions

1. **Dedup logic**: When same network token is used again, should we check for existing `network_token_locker_id` and skip locker save? (Same as card dedup flow)
2. **Locker disabled**: When `locker_enabled = false`, should we still store token metadata in `network_token_payment_method_data` without the locker? (Current NTS flow skips entirely when locker is disabled)
3. **`network_tokenization_data` column**: Currently always `None`. Should we populate this for the raw NT flow, or leave it for a future PR?
