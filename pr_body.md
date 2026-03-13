## Type of Change
- [ ] Bugfix
- [ ] New feature
- [x] Enhancement
- [ ] Refactoring
- [ ] Dependency updates
- [ ] Documentation
- [ ] CI/CD

## Description
Add `card_discovery` as a new dimension in the 3DS decision rule engine to allow rules based on how the card was discovered during checkout.

### Card Discovery Values:
- `manual` → New card entry (user enters full PAN)
- `saved_card` → Stored card token (COF)
- `click_to_pay` → Click to Pay

### Example Rules:
- If `card_discovery = saved_card`, then request a 3DS challenge
- If `card_discovery = manual` AND amount > 50 EUR, then mandate a 3DS challenge

### Changes:
1. **euclid crate**: Export `CardDiscovery` enum and add to DSL framework
2. **common_types**: Add `DirKeyKind::CardDiscovery` to allowed 3DS dimensions
3. **api_models**: Add `card_discovery` field to request models
4. **router**: Pass `card_discovery` from `payment_attempt` to 3DS decision rule

## Motivation and Context
The 3DS rules system currently supports dimensions like amount, currency, etc. Merchants need the ability to apply different 3DS rules based on whether a payment uses a new card entry or a stored card token (card-on-file).

## How did you test it?
The implementation reuses the existing `card_discovery` field from `payment_attempts` table which is already populated during payment confirm.

## Checklist
- [x] I formatted the code `cargo +nightly fmt`
- [x] I addressed lints thrown by `cargo clippy`
- [x] I reviewed the submitted code
- [x] I followed the [commit message guidelines](https://github.com/juspay/hyperswitch/blob/main/docs/CONTRIBUTING.md#commit-message-guidelines)

## Impacted Areas
- 3DS Decision Rule Engine
- Payment Confirm flow
