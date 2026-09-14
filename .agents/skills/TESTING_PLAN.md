# Testing Plan

Strategy for validating Hyperswitch AI coding skills against the live sandbox API.

## Objectives

1. Verify all API endpoints referenced in skills return expected responses
2. Confirm all field names, enum values, and status codes are accurate
3. Ensure code examples are copy-pasteable without modification (beyond API key)
4. Catch API version drift when Hyperswitch updates

## Test Environment Setup

### Sandbox Account

1. Create account at [app.hyperswitch.io](https://app.hyperswitch.io)
2. Add a Stripe connector with your Stripe test secret key (`sk_test_...` from [dashboard.stripe.com](https://dashboard.stripe.com) → Developers → API Keys)
3. Retrieve your API key from **Developers → API Keys**
4. Set environment variable: `export HYPERSWITCH_API_KEY=snd_...`

### Local Webhook Testing

```bash
# Install smee
npm install -g smee-client

# Forward to local server
smee --url https://smee.io/your-channel --path /webhooks --port 3000
```

Register `https://smee.io/your-channel` as webhook URL in dashboard.

## Test Categories

### Category 1: Core API Tests (Automated)

Run via `test-api.sh`:

| Test | Endpoint | Validates |
|------|----------|-----------|
| Create payment (auto capture) | `POST /payments` | Success, status: succeeded |
| Create payment (manual capture) | `POST /payments` | Status: requires_capture |
| Capture payment | `POST /payments/{id}/capture` | Status: succeeded |
| Partial capture | `POST /payments/{id}/capture` | Status: succeeded, amount_capturable: 0 |
| Retrieve payment | `GET /payments/{id}` | Correct fields returned |
| Create refund | `POST /refunds` | Status: pending |
| Retrieve refund | `GET /refunds/{id}` | Status: succeeded or pending |
| Create payment link | `POST /payment_links` | Link URL returned |
| Save card | `POST /payments` (setup_future_usage) | mandate_id returned |
| Charge saved card | `POST /payments` (mandate_id) | Status: succeeded |
| List payments | `GET /payments/list` | count > 0 |

### Category 2: Webhook Tests (Semi-Manual)

1. Start a local webhook server
2. Register webhook URL with smee
3. Create a payment → verify `payment.succeeded` event received
4. Create a refund → verify `refund.succeeded` or `refund.failed` received
5. Verify HMAC signature verification works

### Category 3: 3DS Flow (Manual — Requires Browser)

1. Create payment with `authentication_type: "three_ds"` and card `4000000000003220`
2. Follow redirect URL in browser
3. Complete challenge (test OTP: `123456`)
4. Verify redirect to `return_url`
5. Call `POST /payments/{id}/complete_authorize`
6. Verify status: `succeeded`

### Category 4: SDK Tests (Manual — Requires Browser)

1. Set up demo store locally
2. Complete a payment using `PaymentElement`
3. Verify card, Apple Pay, and Google Pay render correctly
4. Test 3DS redirect flow in the SDK
5. Verify `confirmPayment` error messages render for declined cards

### Category 5: Connector-Specific Tests

#### Stripe
- Verify test cards from the skill produce expected outcomes
- Verify 3DS cards trigger correct flow
- Verify refund timing (sandbox: immediate)

#### Adyen (requires Adyen sandbox account)
- Verify HMAC webhook signature
- Verify iDEAL redirect flow
- Verify Adyen test card `4111111111111111` succeeds

## Regression Testing

Run `test-api.sh` after:
- Hyperswitch version bumps (check `CHANGELOG.md`)
- Any changes to skill content
- Before merging skill PRs

### Automated Regression via CI

```yaml
# .github/workflows/validate-skills.yml
name: Validate Skills
on:
  pull_request:
    paths:
      - '.agents/skills/**'
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Validate skill frontmatter
        run: |
          for f in .agents/skills/**/*.md; do
            python3 -c "
import sys, re
content = open('$f').read()
if not content.startswith('---'):
    print(f'FAIL: {\"$f\"} missing frontmatter')
    sys.exit(1)
print(f'OK: {\"$f\"}')
"
          done
```

## Validation Scenarios

See `VALIDATION_SCENARIOS.md` for the full scenario matrix used to validate each skill end-to-end with an AI assistant.
