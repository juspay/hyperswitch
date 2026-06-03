# RUNNER_RESULT for QAKA-257

**Connector:** stripe
**SpecFile:** cypress/e2e/spec/Payment/55-StripeWallet.cy.js
**PrereqsUsed:** spec/Payment/01-AccountCreate, 02-CustomerCreate, 03-ConnectorCreate
**TotalTests:** 20
**Passed:** 4
**Failed:** 3
**Skipped:** 13
**OverallStatus:** BLOCKED

---

## Failures

### 1. merchant-create-call-test
- **FailureType:** state_seed_failure
- **ErrorMessage:** AssertionError: expected 401 to equal 200
- **ScreenshotPath:** screenshots/service/01-AccountCreate.cy.js/Account Create flow test -- merchant-create-call-test (failed).png
- **RouteTo:** BLOCKED
- **InstructionForNextAgent:** Environment issue - 401 Unauthorized on merchant-create. Re-ran with correct prereqs, still failing. Server requires authentication/merchant setup not available in current environment.

### 2. api-key-create-call-test
- **FailureType:** state_seed_failure
- **ErrorMessage:** Error: API Key create call failed with status 401 and message - "API key not provided or invalid API key used"
- **ScreenshotPath:** screenshots/service/01-AccountCreate.cy.js/Account Create flow test -- api-key-create-call-test (failed).png
- **RouteTo:** BLOCKED
- **InstructionForNextAgent:** Environment issue - 401 Unauthorized on API key creation. Re-ran with correct prereqs, still failing.

### 3. Create merchant connector account
- **FailureType:** state_seed_failure
- **ErrorMessage:** TypeError: Cannot read properties of null (reading 'connector_account_details')
- **ScreenshotPath:** screenshots/service/03-ConnectorCreate.cy.js/Connector Account Create flow test -- Create merchant connector account (failed).png
- **RouteTo:** BLOCKED
- **InstructionForNextAgent:** Environment issue - Connector create returned null body (cascading auth failure from 401 state seed issue). Re-ran with correct prereqs, still failing.

---

## SkippedTests

- **TestName:** All 13 tests in 55-StripeWallet.cy.js (AliPay + CashApp flows)
- **SkipType:** shouldContinue_chain
- **Reason:** All target spec tests pending/skipped because prerequisite specs failed (state seed 401 errors), preventing globalState initialization
- **ActionRequired:** NONE
- **Note:** Tests will execute normally once the state seed/environment issue is resolved

---

## FlakeyTests
- []

---

## BlockedReasons
- Prerequisite spec 01-AccountCreate fails with 401 Unauthorized on merchant-create and api-key-create calls. Re-run attempted with identical results. Hyperswitch server at localhost:8080 returns 401, indicating the test environment lacks proper authentication/merchant setup. This is an environment configuration issue, not a spec bug or API bug, so it cannot be fixed by routing to Process 2 or Process 4.

---

*Note: Unable to post this result as a comment on QAKA-257 via Paperclip API because the issue is checked out by the CEO agent (checkoutRunId: 6f0ade7c-9ab8-4561-9898-0eea4514c8a1). This file serves as the durable record.*
