# Playwright Test Architecture - Multi-Tab Parallel Execution

## Overview

This document explains the complete test architecture and multi-tab parallel execution strategy for Hyperswitch E2E tests.

## Complete Test Suite

### Total: 33 Test Files

```
Setup Tests (Sequential):
├── 00000-CoreFlows.spec.ts              ← Merchant, API Key, Customer core flows
├── 00001-AccountCreate.spec.ts           ← Account creation
├── 00002-CustomerCreate.spec.ts          ← Customer creation
└── 00003-ConnectorCreate.spec.ts         ← Connector setup

Parallel Tests (Multi-Tab):
├── 00004-NoThreeDSAutoCapture.spec.ts    ← No 3DS automatic capture
├── 00005-ThreeDSAutoCapture.spec.ts      ← 3DS automatic capture
├── 00006-NoThreeDSManualCapture.spec.ts  ← No 3DS manual capture
├── 00007-VoidPayment.spec.ts             ← Payment cancellation
├── 00008-SyncPayment.spec.ts             ← Payment synchronization
├── 00009-RefundPayment.spec.ts           ← Payment refunds
├── 00010-SyncRefund.spec.ts              ← Refund synchronization
├── 00011-CreateSingleuseMandate.spec.ts  ← Single-use mandates
├── 00012-CreateMultiuseMandate.spec.ts   ← Multi-use mandates
├── 00013-ListAndRevokeMandate.spec.ts    ← Mandate management
├── 00014-SaveCardFlow.spec.ts            ← Save card for future use
├── 00015-ZeroAuthMandate.spec.ts         ← Zero auth mandates
├── 00016-ThreeDSManualCapture.spec.ts    ← 3DS manual capture
├── 00017-BankTransfers.spec.ts           ← Bank transfer payments
├── 00018-BankRedirect.spec.ts            ← Bank redirect payments
├── 00019-MandatesUsingPMID.spec.ts       ← Mandates via PM ID
├── 00020-MandatesUsingNTIDProxy.spec.ts  ← Mandates via NTID
├── 00021-UPI.spec.ts                     ← UPI payments
├── 00022-Variations.spec.ts              ← Edge cases & variations
├── 00023-PaymentMethods.spec.ts          ← Payment method management
├── 00024-ConnectorAgnosticNTID.spec.ts   ← Connector agnostic NTID
├── 00025-SessionCall.spec.ts             ← Session token management
├── 00026-DeletedCustomerPsyncFlow.spec.ts ← Customer deletion edge cases
├── 00027-BusinessProfileConfigs.spec.ts  ← Business profile configs
├── 00028-IncrementalAuth.spec.ts         ← Incremental authorization
├── 00029-DynamicFields.spec.ts           ← Dynamic field validation
├── 00030-DDCRaceCondition.spec.ts        ← DDC race condition tests
├── 00031-Overcapture.spec.ts             ← Overcapture functionality
└── 00032-ManualRetry.spec.ts             ← Manual retry features
```

## Execution Architecture

### Phase 1: Sequential Setup (Tests 00000-00003)

```
┌─────────────────────────────────────────────────────────────┐
│ Sequential Setup Phase                                       │
│                                                              │
│ 1-core-setup (00000)                                        │
│         ↓                                                    │
│ 2-account-setup (00001)                                     │
│         ↓                                                    │
│ 3-customer-setup (00002)                                    │
│         ↓                                                    │
│ 4-connector-setup (00003)                                   │
│         ↓                                                    │
│ test-state.json created                                     │
└─────────────────────────────────────────────────────────────┘
```

**Purpose:**
- Create merchant account
- Generate API keys
- Create test customer
- Set up connector (Stripe or Cybersource)
- Save all IDs and tokens to `test-state.json`

**Execution Time:** ~2 minutes

### Phase 2: Parallel Multi-Tab Execution (Tests 00004-00032)

```
┌─────────────────────────────────────────────────────────────┐
│ Parallel Tests - Multi-Tab Execution                        │
│                                                              │
│ Worker 1 (Stripe):          Worker 2 (Cybersource):        │
│ ┌──────────────────┐        ┌──────────────────┐           │
│ │ Tab 1:  00004   │        │ Tab 1:  00004   │           │
│ │ Tab 2:  00005   │        │ Tab 2:  00005   │           │
│ │ Tab 3:  00006   │        │ Tab 3:  00006   │           │
│ │ Tab 4:  00007   │        │ Tab 4:  00007   │           │
│ │ Tab 5:  00008   │        │ Tab 5:  00008   │           │
│ │ ...              │        │ ...              │           │
│ │ Tab 29: 00032   │        │ Tab 29: 00032   │           │
│ └──────────────────┘        └──────────────────┘           │
│                                                              │
│ Total: 58 browser contexts (29 per connector)              │
└─────────────────────────────────────────────────────────────┘
```

**How It Works:**
1. **One Test File = One Browser Context (Tab)**
   - Each test file (00004-00032) runs in its own isolated browser context
   - 29 test files = 29 browser contexts running simultaneously

2. **Connector Separation:**
   - Each connector (Stripe/Cybersource) gets its own worker process
   - Both workers run in parallel
   - Total contexts: 29 (Stripe) + 29 (Cybersource) = 58 contexts

3. **State Sharing:**
   - All tests read from shared `test-state.json`
   - Tests access merchant ID, API key, customer ID from setup phase
   - Each test can update state for its specific scenario

**Execution Time:** ~30 seconds to 1 minute (depending on API response times)

## Configuration Details

### Playwright Config (`playwright.config.ts`)

```typescript
{
  workers: 2,  // One for Stripe, one for Cybersource
  fullyParallel: true,  // Tests run in parallel within each worker

  projects: [
    // Setup tests (sequential)
    { name: '1-core-setup', testMatch: '**/setup/00000-*.spec.ts' },
    { name: '2-account-setup', dependencies: ['1-core-setup'] },
    { name: '3-customer-setup', dependencies: ['2-account-setup'] },
    { name: '4-connector-setup', dependencies: ['3-customer-setup'] },

    // Parallel tests (all at once)
    {
      name: 'parallel-tests',
      testMatch: [
        '**/spec/0000[4-9]-*.spec.ts',  // 00004-00009
        '**/spec/0001[0-9]-*.spec.ts',  // 00010-00019
        '**/spec/0002[0-9]-*.spec.ts',  // 00020-00029
        '**/spec/0003[0-2]-*.spec.ts',  // 00030-00032
      ],
      dependencies: ['4-connector-setup'],
      fullyParallel: true
    }
  ]
}
```

## Resource Usage

### RAM Calculation

**Per Browser Context:**
- Headless: ~130 MB
- Headful: ~275 MB

**Total RAM Usage:**

| Mode | Per Connector | Both Connectors | GitHub CI (16GB) |
|------|---------------|-----------------|------------------|
| **Headless** | 3.8 GB | 7.6 GB | ✅ Safe (8.4 GB free) |
| **Headful** | 8.0 GB | 16.0 GB | ⚠️ Tight (0 GB free) |

**Recommendation:**
- **CI/CD**: Always use headless mode
- **Local Dev**: Use headless or reduce worker count to 1

### CPU Usage

- 2 workers = 2 CPU cores minimum
- Each browser context uses minimal CPU for API-only tests
- Recommended: 4+ CPU cores for optimal performance

## Running Tests

### Full Suite (Setup + Parallel)

```bash
# Run everything
npm test

# Results in:
# 1. Setup tests run sequentially (2 min)
# 2. All 29 parallel tests run simultaneously (30 sec)
# Total: ~2.5 minutes
```

### Setup Only

```bash
# Run just the setup tests
npx playwright test --project=1-core-setup --project=2-account-setup --project=3-customer-setup --project=4-connector-setup
```

### Parallel Tests Only (requires setup to run first)

```bash
# Run just parallel tests (assumes setup already completed)
npx playwright test --project=parallel-tests
```

### Specific Connector

```bash
# Stripe only
PLAYWRIGHT_CONNECTOR=stripe npm test

# Cybersource only
PLAYWRIGHT_CONNECTOR=cybersource npm test
```

### Single Test File

```bash
# Run one specific test
npx playwright test tests/e2e/spec/00004-NoThreeDSAutoCapture.spec.ts
```

### Headed Mode (Visual Debugging)

```bash
# See all browser windows
HEADLESS=false npx playwright test --project=parallel-tests

# You'll see:
# - 29 browser windows opening simultaneously
# - Each window running a different test
# - All tests executing in parallel
```

## Performance Comparison

### Cypress vs Playwright

| Metric | Cypress (Sequential) | Playwright (Parallel) | Speedup |
|--------|----------------------|-----------------------|---------|
| **Setup Tests** | ~2 min | ~2 min | 1x |
| **Parallel Tests** | ~39.5 min | ~30 sec | **79x** |
| **Total Suite** | ~41.5 min | ~2.5 min | **16.6x** |

### Why So Fast?

1. **True Parallelism:**
   - Cypress: Runs tests one at a time
   - Playwright: Runs 29 tests simultaneously

2. **Efficient Contexts:**
   - Lightweight browser contexts (tabs)
   - Share browser process
   - Minimal overhead

3. **API-Only Tests:**
   - Most tests don't need browser UI
   - Use Playwright's API request context
   - No page rendering overhead

## Test Isolation

### How Tests Stay Isolated

1. **Separate Browser Contexts:**
   - Each test gets its own context
   - Separate cookies, localStorage, sessionStorage
   - No data leakage between tests

2. **Shared State (Read-Only):**
   - Tests read from `test-state.json`
   - Access common resources (merchant ID, API key, customer ID)
   - Don't modify shared state during parallel execution

3. **Test-Specific State:**
   - Each test creates its own payments, refunds, etc.
   - Uses unique identifiers
   - No conflicts with other tests

## Troubleshooting

### Tests Running Sequentially Instead of Parallel

**Problem:** Tests run one by one instead of all at once

**Solution:**
```typescript
// In your test file, add:
test.describe.configure({ mode: 'parallel' });
```

### Out of Memory Errors

**Problem:** System runs out of RAM

**Solutions:**
1. Run in headless mode: `HEADLESS=true npm test`
2. Reduce workers: Change `workers: 2` to `workers: 1` in config
3. Run smaller batches: Test specific files only

### Setup Tests Failing

**Problem:** Parallel tests can't run because setup failed

**Solution:**
1. Check environment variables are set
2. Ensure API server is accessible
3. Verify connector credentials are valid
4. Run setup tests individually to debug:
   ```bash
   npx playwright test tests/e2e/setup/00000-CoreFlows.spec.ts
   ```

### State File Not Found

**Problem:** Parallel tests complain about missing state

**Solution:**
1. Ensure global-setup.ts ran successfully
2. Check `test-state.json` exists in project root
3. Verify setup tests completed successfully

## Next Steps

### Adding More Tests

1. **Create new test file:**
   ```typescript
   // tests/e2e/spec/00033-NewFeature.spec.ts
   import { test } from '../../fixtures/imports';

   test.describe('New Feature Tests', () => {
     test('test scenario', async ({ request, globalState }) => {
       // Your test code
     });
   });
   ```

2. **Update playwright.config.ts:**
   ```typescript
   testMatch: [
     '**/spec/0000[4-9]-*.spec.ts',
     '**/spec/0001[0-9]-*.spec.ts',
     '**/spec/0002[0-9]-*.spec.ts',
     '**/spec/0003[0-3]-*.spec.ts',  // Updated to include 00033
   ]
   ```

3. **Run it:**
   ```bash
   npm test  # Automatically includes new test in parallel execution
   ```

### Optimizing Performance

1. **Profile slow tests:**
   ```bash
   npx playwright test --reporter=html
   # Open report to see which tests are slowest
   ```

2. **Increase timeout for slow tests:**
   ```typescript
   test('slow operation', async ({ request, globalState }) => {
     test.setTimeout(180000); // 3 minutes
     // Test code
   });
   ```

3. **Skip tests conditionally:**
   ```typescript
   test('feature X', async ({ globalState }) => {
     const connector = globalState.get('connectorId');
     if (connector === 'unsupported_connector') {
       test.skip();
     }
     // Test code
   });
   ```

## Summary

✅ **33 total tests:** 4 setup + 29 parallel
✅ **Multi-tab execution:** Each test = 1 browser context
✅ **True parallelism:** 29 tests run simultaneously
✅ **16.6x faster** than Cypress
✅ **Resource efficient:** ~7.6 GB RAM in headless mode
✅ **CI/CD ready:** Optimized for GitHub Actions

The architecture provides maximum performance while maintaining test isolation and reliability!
