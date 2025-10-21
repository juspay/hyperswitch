# Playwright E2E Tests for Hyperswitch

Playwright-based end-to-end testing framework for Hyperswitch payment processing, converted from Cypress with advanced browser pool parallelization.

## Quick Start

Get started in 3 simple steps:

### 1. Install Dependencies

```bash
cd playwright-tests
npm install
npx playwright install chromium
```

### 2. Configure Environment

Copy the example environment file and update with your values:

```bash
cp .env.example .env
```

Edit `.env` and set:
- `PLAYWRIGHT_BASEURL` - Your Hyperswitch API URL (default: `http://127.0.0.1:8081`)
- `PLAYWRIGHT_ADMINAPIKEY` - Admin API key
- `PLAYWRIGHT_CONNECTOR_AUTH_FILE_PATH` - Path to connector credentials JSON

### 3. Run Tests

```bash
# Run with browser pool mode (recommended - fastest)
USE_BROWSER_POOL=true CONTEXTS_PER_BROWSER=25 npx playwright test --project=stripe-parallel-tests --project=cybersource-parallel-tests

# Or use default mode (slower but uses less RAM)
npx playwright test --project=stripe-parallel-tests --project=cybersource-parallel-tests
```

View results:
```bash
npx playwright show-report
```

---

## Table of Contents

- [Architecture](#architecture)
- [Browser Pool Mode](#browser-pool-mode)
- [Prerequisites](#prerequisites)
- [Installation](#installation)
- [Running Tests](#running-tests)
- [GitHub Actions Configuration](#github-actions-configuration)
- [Project Structure](#project-structure)
- [Writing Tests](#writing-tests)
- [Environment Configuration](#environment-configuration)
- [Debugging](#debugging)
- [Performance Comparison](#performance-comparison)
- [Troubleshooting](#troubleshooting)

---

## Architecture

### Test Execution Strategy

Tests are organized into two phases to optimize performance while maintaining reliability:

#### Phase 1: Sequential Setup (Tests 00000-00003)
- **00000-CoreFlows**: Creates merchant account, API keys, customer, and connector
- **00001-AccountCreate**: Additional account configuration
- **00002-CustomerCreate**: Customer management tests
- **00003-ConnectorCreate**: Connector setup and configuration

These tests run **sequentially** using Playwright's project dependencies to create shared global state stored in connector-specific state files (`test-state-stripe.json`, `test-state-cybersource.json`).

#### Phase 2: Parallel Execution (Tests 0004+)

The parallel execution phase uses one of two modes:

**Traditional Mode** (Default - 2 Workers):
- 2 workers running tests in parallel
- Each test gets its own browser instance
- RAM usage: ~2-4 GB
- Duration: ~3-5 minutes for full suite

**Browser Pool Mode** (Recommended - High Parallelism):
- 2 persistent browsers (1 per connector: Stripe + Cybersource)
- Configurable reusable browser contexts (tabs) per browser
- Each context runs tests independently
- Total concurrent tests = `CONTEXTS_PER_BROWSER` × 2 connectors
- Example: 25 contexts/browser = 50 concurrent tests
- RAM usage: ~3.75 GB (25 contexts) to ~6.75 GB (50 contexts)
- Duration: ~1.5-2 minutes for full suite
- **20x faster** than sequential execution

---

## Browser Pool Mode

### What is Browser Pool Mode?

Browser Pool Mode is an advanced parallelization strategy that dramatically improves test execution speed while using reasonable RAM.

**How it works:**
1. Creates **2 persistent browsers** (one for Stripe, one for Cybersource)
2. Creates a **pool of reusable browser contexts** (tabs) in each browser
3. Tests acquire a context from the pool, run, then release it back
4. Contexts are immediately reused by waiting tests
5. No browser launch overhead after initial setup

**Visual Example:**
```
Browser 1 (Stripe)                Browser 2 (Cybersource)
├─ Context 1 → Test A → Release   ├─ Context 1 → Test X → Release
├─ Context 2 → Test B → Release   ├─ Context 2 → Test Y → Release
├─ Context 3 → Test C → Release   ├─ Context 3 → Test Z → Release
├─ Context 4 → Test D → Release   ├─ Context 4 → Test W → Release
└─ ...                             └─ ...
```

### RAM Calculation

**Formula:**
```
Total RAM = (Number of Browsers × 600 MB) + (Total Contexts × 51 MB) + Overhead (~1.5 GB)
```

**Examples:**

| Contexts/Browser | Total Contexts | Base RAM | Total RAM | Concurrent Tests |
|------------------|----------------|----------|-----------|------------------|
| 10 | 20 | 2.22 GB | ~3.75 GB | 20 |
| 15 | 30 | 2.73 GB | ~4.25 GB | 30 |
| 25 | 50 | 3.75 GB | ~5.25 GB | 50 |
| 50 | 100 | 6.30 GB | ~7.80 GB | 100 |

### Configuration Recommendations

**Local Development:**

| Your RAM | Recommended `CONTEXTS_PER_BROWSER` | Expected RAM Usage | Concurrent Tests |
|----------|-----------------------------------|-------------------|------------------|
| 8 GB | 5-10 | ~3-4 GB | 10-20 |
| 16 GB | 15-25 | ~4-5.5 GB | 30-50 |
| 32+ GB | 40-50 | ~7-8 GB | 80-100 |

**GitHub Actions / CI:**

| Runner RAM | Recommended `CONTEXTS_PER_BROWSER` | Expected RAM Usage | Notes |
|------------|-----------------------------------|-------------------|-------|
| 7 GB (ubuntu-latest) | 10-15 | ~4-4.5 GB | Safe, leaves buffer |
| 14 GB (macos-latest) | 25 | ~5.5 GB | Optimal performance |
| 16 GB (custom) | 25 | ~5.5 GB | Recommended |
| 32+ GB | 40-50 | ~7-8 GB | Maximum speed |

### Enabling Browser Pool Mode

**Option 1: Environment Variables (Recommended)**
```bash
USE_BROWSER_POOL=true CONTEXTS_PER_BROWSER=25 npx playwright test --project=stripe-parallel-tests --project=cybersource-parallel-tests
```

**Option 2: Update .env File**
```bash
# Add to .env
USE_BROWSER_POOL=true
CONTEXTS_PER_BROWSER=25
```

Then run:
```bash
npx playwright test --project=stripe-parallel-tests --project=cybersource-parallel-tests
```

---

## Prerequisites

- **Node.js**: v18+ (v20 recommended)
- **npm**: v8+
- **RAM**:
  - Minimum: 4 GB (default mode)
  - Recommended: 16 GB (browser pool with 25 contexts)
- **Hyperswitch Server**: Running and accessible

**Environment Variables:**
- `PLAYWRIGHT_BASEURL` - Hyperswitch API base URL
- `PLAYWRIGHT_ADMINAPIKEY` - Admin API key for authentication
- `PLAYWRIGHT_CONNECTOR_AUTH_FILE_PATH` - Path to connector credentials JSON file
- `PLAYWRIGHT_CONNECTORS` - Comma-separated list of connectors to test (default: `stripe,cybersource`)

---

## Installation

```bash
# Navigate to playwright-tests directory
cd playwright-tests

# Install Node dependencies
npm install

# Install Playwright browsers
npx playwright install chromium

# Verify installation
npx playwright --version
```

---

## Running Tests

### Local Development

**Basic Commands:**

```bash
# Run with browser pool mode (recommended - fastest, ~1.5-2 min)
USE_BROWSER_POOL=true CONTEXTS_PER_BROWSER=25 npx playwright test --project=stripe-parallel-tests --project=cybersource-parallel-tests

# Run in default mode (slower but less RAM, ~3-5 min)
npx playwright test --project=stripe-parallel-tests --project=cybersource-parallel-tests

# Run in headed mode (see browser windows - useful for debugging)
USE_BROWSER_POOL=true CONTEXTS_PER_BROWSER=2 HEADLESS=false npx playwright test --project=stripe-parallel-tests --project=cybersource-parallel-tests

# Run in UI mode (interactive debugging)
npx playwright test --ui

# Run specific test file
npx playwright test tests/e2e/spec/00004-NoThreeDSAutoCapture.spec.ts

# Run only Stripe tests
npx playwright test --project=stripe-1-core-setup --project=stripe-2-account-setup --project=stripe-3-customer-setup --project=stripe-4-connector-setup --project=stripe-parallel-tests

# Run only Cybersource tests
npx playwright test --project=cybersource-1-core-setup --project=cybersource-2-account-setup --project=cybersource-3-customer-setup --project=cybersource-4-connector-setup --project=cybersource-parallel-tests
```

**npm Scripts:**

```bash
# Run all tests (uses .env configuration)
npm test

# Run in headed mode
npm run test:headed

# Run with UI mode
npm run test:ui

# Show HTML report
npm run report

# Format code
npm run format

# Type checking
npm run type-check
```

### Advanced Usage

**Adjust Parallelism:**

```bash
# Conservative (low RAM - 8 GB machine)
USE_BROWSER_POOL=true CONTEXTS_PER_BROWSER=10 npx playwright test --project=stripe-parallel-tests --project=cybersource-parallel-tests

# Balanced (medium RAM - 16 GB machine)
USE_BROWSER_POOL=true CONTEXTS_PER_BROWSER=25 npx playwright test --project=stripe-parallel-tests --project=cybersource-parallel-tests

# Aggressive (high RAM - 32+ GB machine)
USE_BROWSER_POOL=true CONTEXTS_PER_BROWSER=50 npx playwright test --project=stripe-parallel-tests --project=cybersource-parallel-tests
```

**Run with Logging:**

```bash
# Save output to log file
npx playwright test --project=stripe-parallel-tests --project=cybersource-parallel-tests 2>&1 | tee test-results.log

# Enable verbose pool logging
VERBOSE_POOL=true USE_BROWSER_POOL=true CONTEXTS_PER_BROWSER=25 npx playwright test --project=stripe-parallel-tests --project=cybersource-parallel-tests
```

---

## GitHub Actions Configuration

### Example Workflow for 16 GB Runner

Create `.github/workflows/playwright-tests.yml`:

```yaml
name: Playwright Tests

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main, develop]

jobs:
  test:
    name: Run Playwright E2E Tests
    runs-on: ubuntu-latest-16-cores  # 16 GB RAM runner

    steps:
      - name: Checkout code
        uses: actions/checkout@v4

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '20'
          cache: 'npm'
          cache-dependency-path: playwright-tests/package-lock.json

      - name: Install dependencies
        working-directory: playwright-tests
        run: |
          npm ci
          npx playwright install chromium --with-deps

      - name: Start Hyperswitch Server
        run: |
          # Add your server start command here
          # Example: docker-compose up -d
          # Or: cargo run --bin router &
          echo "Server starting..."
          # Wait for server to be ready
          sleep 10

      - name: Run Playwright Tests (Browser Pool Mode)
        working-directory: playwright-tests
        env:
          PLAYWRIGHT_BASEURL: http://127.0.0.1:8081
          PLAYWRIGHT_ADMINAPIKEY: ${{ secrets.ADMIN_API_KEY }}
          PLAYWRIGHT_CONNECTOR_AUTH_FILE_PATH: ./connector-creds.json
          PLAYWRIGHT_CONNECTORS: stripe,cybersource
          USE_BROWSER_POOL: true
          CONTEXTS_PER_BROWSER: 25  # Optimal for 16 GB
          HEADLESS: true
          CI: true
        run: |
          npx playwright test --project=stripe-parallel-tests --project=cybersource-parallel-tests

      - name: Upload Playwright Report
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: playwright-report
          path: playwright-tests/playwright-report/
          retention-days: 30

      - name: Upload Test Results
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: test-results
          path: playwright-tests/test-results/
          retention-days: 30
```

### Configuration for Different Runner Sizes

**7 GB Runner (ubuntu-latest):**
```yaml
env:
  USE_BROWSER_POOL: true
  CONTEXTS_PER_BROWSER: 10  # Conservative for 7 GB
```

**14 GB Runner (macos-latest):**
```yaml
env:
  USE_BROWSER_POOL: true
  CONTEXTS_PER_BROWSER: 25  # Optimal performance
```

**16 GB Runner (custom):**
```yaml
env:
  USE_BROWSER_POOL: true
  CONTEXTS_PER_BROWSER: 25  # Recommended
```

**32+ GB Runner:**
```yaml
env:
  USE_BROWSER_POOL: true
  CONTEXTS_PER_BROWSER: 50  # Maximum speed
```

---

## Project Structure

```
playwright-tests/
├── .env.example                 # Environment variables template
├── README.md                    # This file
├── global-setup.ts              # Initialize connector-specific state files
├── global-teardown.ts           # Cleanup and archive state after tests
├── playwright.config.ts         # Main Playwright configuration
├── package.json                 # Dependencies and scripts
├── tsconfig.json                # TypeScript configuration
├── utils/
│   ├── BrowserPool.ts           # Browser pool implementation
│   └── ParallelTestOrchestrator.ts  # Multi-connector orchestration
├── reporters/
│   └── performance-reporter.ts  # Custom performance metrics reporter
├── tests/
│   ├── e2e/
│   │   ├── configs/             # Connector configurations
│   │   │   ├── ConnectorTypes.ts    # TypeScript interfaces
│   │   │   ├── Commons.ts           # Shared test data
│   │   │   ├── Stripe.ts            # Stripe test configurations
│   │   │   └── Cybersource.ts       # Cybersource test configurations
│   │   ├── setup/               # Sequential setup tests (00000-00003)
│   │   │   ├── 00000-CoreFlows.spec.ts      # Core merchant/API setup
│   │   │   ├── 00001-AccountCreate.spec.ts  # Account configuration
│   │   │   ├── 00002-CustomerCreate.spec.ts # Customer setup
│   │   │   └── 00003-ConnectorCreate.spec.ts # Connector creation
│   │   └── spec/                # Parallel test files (0004+)
│   │       ├── 00004-NoThreeDSAutoCapture.spec.ts
│   │       ├── 00005-ThreeDSAutoCapture.spec.ts
│   │       ├── 00006-NoThreeDSManualCapture.spec.ts
│   │       └── ... (all payment flow tests)
│   ├── fixtures/                # Shared test fixtures
│   │   ├── imports.ts           # Barrel export for all utilities
│   │   ├── test-data.ts         # Request body fixtures
│   │   └── test-fixtures.ts     # Playwright fixtures (globalState, browser pool)
│   └── helpers/                 # Helper functions
│       ├── ApiHelpers.ts        # HTTP request helpers
│       └── RedirectionHelper.ts # 3DS/redirect handling
├── test-state-stripe.json       # Stripe connector state (generated)
├── test-state-cybersource.json  # Cybersource connector state (generated)
└── test-results/                # Test results and reports (gitignored)
```

---

## Key Files Explained

### `playwright.config.ts`
Main configuration file that:
- Configures workers (2 for default mode, or uses `CONTEXTS_PER_BROWSER` for pool mode)
- Defines project dependencies for sequential setup
- Configures reporters (HTML, JSON, GitHub, Performance)
- Sets timeouts and retry strategies
- Enables browser pool when `USE_BROWSER_POOL=true`

### `utils/BrowserPool.ts`
Implements browser context pooling:
- Creates and manages reusable browser contexts
- Provides `acquire()` and `release()` methods
- Handles context cleanup and error recovery
- Tracks context usage and metrics

### `utils/ParallelTestOrchestrator.ts`
Orchestrates multi-connector testing:
- Creates separate browser pools for each connector
- Manages browser pool lifecycle (setup/teardown)
- Provides connector-specific context acquisition

### `tests/fixtures/test-fixtures.ts`
Provides custom Playwright fixtures:
- **`globalState`**: Connector-specific state management
- **`page`** (overridden): Automatically acquires context from browser pool
- Handles browser pool integration transparently

### `tests/helpers/ApiHelpers.ts`
TypeScript port of Cypress custom commands:
- `merchantCreateCall()`, `apiKeyCreateTest()`, `createCustomerCall()`, etc.
- Uses Playwright's `APIRequestContext` for HTTP requests
- Integrates with `globalState` for data persistence

### `tests/helpers/RedirectionHelper.ts`
Handles payment redirections:
- **`handle3DSRedirection()`**: Handles 3DS authentication flows
- **`handleGenericRedirection()`**: Handles bank transfer/redirect flows
- Auto-corrects port 8080→8081 mismatches
- Connector-specific handling (Stripe iframe, Cybersource wait)

### `tests/e2e/configs/`
Connector test configurations:
- **Commons.ts**: Shared card details, mandate data, customer acceptance
- **Stripe.ts**: Stripe-specific test data and expected responses
- **Cybersource.ts**: Cybersource-specific test data
- **ConnectorTypes.ts**: TypeScript interfaces for type safety

---

## Writing Tests

### Example: Basic API Test

```typescript
import { test, expect } from '../../fixtures/imports';
import * as fixtures from '../../fixtures/test-data';
import { getConnectorDetails, shouldContinueFurther } from '../configs/Payment/Utils';

test.describe.serial('My Test Suite', () => {
  test('create payment', async ({ request, globalState }) => {
    const connectorId = globalState.get('connectorId');
    const connectorConfig = getConnectorDetails(connectorId);
    const data = connectorConfig.card_pm.PaymentIntent;

    const baseUrl = globalState.get('baseUrl');
    const apiKey = globalState.get('apiKey');

    const requestBody = {
      ...fixtures.createPaymentBody,
      ...data.Request,
      customer_id: globalState.get('customerId'),
    };

    const response = await request.post(`${baseUrl}/payments`, {
      headers: {
        'Content-Type': 'application/json',
        'api-key': apiKey,
      },
      data: requestBody,
    });

    const body = await response.json();

    expect(response.status()).toBe(data.Response.status);
    expect(body.status).toBe(data.Response.body.status);

    globalState.set('paymentId', body.payment_id);
  });
});
```

### Example: Browser Interaction Test

```typescript
import { test } from '../../fixtures/imports';
import { handle3DSRedirection } from '../../helpers/RedirectionHelper';

test('Handle 3DS authentication', async ({ page, globalState }) => {
  const connectorId = globalState.get('connectorId');
  const nextActionUrl = globalState.get('nextActionUrl');
  const expectedRedirection = 'https://example.com';

  // Automatically handles Stripe iframe clicks or Cybersource waits
  await handle3DSRedirection(page, connectorId, nextActionUrl, expectedRedirection);

  console.log('✓ 3DS authentication completed');
});
```

### Accessing Global State

```typescript
test('my test', async ({ globalState }) => {
  // Get values
  const merchantId = globalState.get('merchantId');
  const apiKey = globalState.get('apiKey');
  const connectorId = globalState.get('connectorId');

  // Set values (only in setup tests)
  globalState.set('customerId', 'cust_123');
  globalState.set('paymentId', 'pay_456');

  // Check existence
  if (globalState.has('connectorId')) {
    console.log(`Testing with connector: ${connectorId}`);
  }
});
```

### Test Organization Best Practices

1. **Use `test.describe.serial()`** for tests that must run in order
2. **Use `test.describe.configure({ mode: 'parallel' })`** for independent tests
3. **Store test data in configs/** for reusability
4. **Use globalState** for sharing data between tests
5. **Follow naming convention**: `XXXXX-TestName.spec.ts` (e.g., `00004-NoThreeDSAutoCapture.spec.ts`)

---

## Environment Configuration

### Required Environment Variables

Create a `.env` file based on `.env.example`:

```bash
# Copy example file
cp .env.example .env
```

Edit `.env` with your values:

```bash
# ===== Base Configuration =====
PLAYWRIGHT_BASEURL=http://127.0.0.1:8081
PLAYWRIGHT_ADMINAPIKEY=test_admin
PLAYWRIGHT_CONNECTOR_AUTH_FILE_PATH=/path/to/connector-creds.json
PLAYWRIGHT_CONNECTORS=stripe,cybersource

# ===== Browser Configuration =====
HEADLESS=true

# ===== Browser Pool Configuration =====
USE_BROWSER_POOL=true
CONTEXTS_PER_BROWSER=25

# ===== Optional =====
CI=false
VERBOSE_POOL=false
```

### Connector Credentials File

The `PLAYWRIGHT_CONNECTOR_AUTH_FILE_PATH` should point to a JSON file with this structure:

```json
{
  "connector_1": {
    "connector_account_details": {
      "auth_type": "BodyKey",
      "api_key": "sk_test_xxxxxxxxxxxxx"
    }
  },
  "connector_2": {
    "connector_account_details": {
      "auth_type": "SignatureKey",
      "api_key": "merchant_id_here",
      "key1": "secret_key_here",
      "api_secret": "shared_secret_here"
    }
  }
}
```

**Note**: The JSON structure uses nested `connector_1`, `connector_2`, etc. The setup will automatically extract the first connector's details.

---

## Debugging

### View Test Results

```bash
# Open HTML report (auto-generated after test run)
npx playwright show-report

# View on specific host/port
npx playwright show-report --host 127.0.0.1 --port 9323

# View JSON results
cat test-results/results.json | jq '.'
```

### Run Tests in Debug Mode

```bash
# Debug specific test with Playwright Inspector
PWDEBUG=1 npx playwright test tests/e2e/spec/00004-NoThreeDSAutoCapture.spec.ts

# Run in headed mode (see browser windows)
HEADLESS=false npx playwright test --project=stripe-parallel-tests

# Run with browser pool in headed mode
USE_BROWSER_POOL=true CONTEXTS_PER_BROWSER=2 HEADLESS=false npx playwright test --project=stripe-parallel-tests

# Enable verbose pool logging
VERBOSE_POOL=true USE_BROWSER_POOL=true npx playwright test --project=stripe-parallel-tests
```

### Check Global State

```bash
# View Stripe connector state
cat test-state-stripe.json | jq '.'

# View Cybersource connector state
cat test-state-cybersource.json | jq '.'

# View archived states (from previous runs)
ls -la test-results/test-state-*.json
```

### Interactive UI Mode

```bash
# Launch Playwright UI for visual debugging
npx playwright test --ui

# Note: UI mode has limited parallelism (good for debugging, not performance)
```

---

## Performance Comparison

### Execution Time

| Mode | Configuration | Duration | Speedup vs Sequential |
|------|---------------|----------|----------------------|
| **Sequential** (Cypress) | 1 worker | ~41.5 minutes | 1x (baseline) |
| **Default Parallel** | 2 workers | ~3-5 minutes | ~10-15x |
| **Browser Pool (Conservative)** | 10 contexts/browser (20 total) | ~2-3 minutes | ~15-20x |
| **Browser Pool (Recommended)** | 25 contexts/browser (50 total) | ~1.5-2 minutes | ~20-25x |
| **Browser Pool (Aggressive)** | 50 contexts/browser (100 total) | ~1-1.5 minutes | ~30-40x |

### RAM Usage

| Mode | Configuration | Peak RAM Usage | Suitable For |
|------|---------------|----------------|--------------|
| Sequential | 1 worker | ~1-2 GB | Very low RAM (<4 GB) |
| Default Parallel | 2 workers | ~2-4 GB | Low RAM (4-8 GB) |
| Browser Pool (10 contexts) | 20 total contexts | ~3.75 GB | Medium RAM (8-12 GB) |
| Browser Pool (25 contexts) | 50 total contexts | ~5.25 GB | High RAM (16 GB) |
| Browser Pool (50 contexts) | 100 total contexts | ~7.80 GB | Very high RAM (32+ GB) |

### Test Breakdown

| Phase | Tests | Default Mode | Browser Pool (25 contexts) |
|-------|-------|--------------|---------------------------|
| Setup (sequential) | 8 tests | ~20-30 sec | ~20-30 sec |
| Parallel tests | ~400 tests | ~3-4 min | ~1-1.5 min |
| **Total** | **~408 tests** | **~4-5 min** | **~1.5-2 min** |

### Real-World Metrics

Based on actual test runs with 832 tests (Stripe + Cybersource):

```bash
# Browser Pool Mode (CONTEXTS_PER_BROWSER=25)
Workers: 2 (browser pool mode)
Concurrent tests: 50 (25 per connector)
Duration: ~2 minutes
RAM: ~5.5 GB peak
Speedup: 20x vs sequential

# Default Mode
Workers: 2
Concurrent tests: 2
Duration: ~4 minutes
RAM: ~3 GB peak
Speedup: 10x vs sequential
```

---

## Troubleshooting

### Tests Fail with "State file not found"

**Problem**: Global state files not initialized

**Solutions**:
- Ensure global-setup runs before tests
- Check that setup projects completed successfully
- Verify state files exist: `ls -la test-state-*.json`
- Run setup manually: `npx playwright test --project=stripe-1-core-setup --project=cybersource-1-core-setup`

### RAM/Memory Issues

**Problem**: Out of memory errors or system slowdown

**Solutions**:
```bash
# Reduce contexts per browser
USE_BROWSER_POOL=true CONTEXTS_PER_BROWSER=10 npx playwright test

# Use default mode (lower RAM)
npx playwright test --project=stripe-parallel-tests --project=cybersource-parallel-tests

# Enable headless mode (uses less RAM)
HEADLESS=true npx playwright test

# Monitor RAM usage
# macOS: Activity Monitor
# Linux: htop or free -h
# Windows: Task Manager
```

### Browser Pool Context Exhaustion

**Problem**: Tests waiting for available context, slow execution

**Symptoms**:
- Log shows: "Waiting for context... (attempt X)"
- Some tests take much longer than expected

**Solutions**:
```bash
# Increase contexts per browser
USE_BROWSER_POOL=true CONTEXTS_PER_BROWSER=50 npx playwright test

# Enable verbose logging to see pool behavior
VERBOSE_POOL=true USE_BROWSER_POOL=true npx playwright test

# Check if some tests are hanging (increase timeout)
```

### API Connection Errors

**Problem**: Tests fail with connection refused or timeout

**Solutions**:
- Verify server is running: `curl http://127.0.0.1:8081/health`
- Check `PLAYWRIGHT_BASEURL` in `.env`
- Ensure firewall allows connections
- Use IPv4 address `127.0.0.1` instead of `localhost`
- Verify port (8081 not 8080)

### Connector Authentication Fails

**Problem**: 401/403 errors or invalid connector credentials

**Solutions**:
- Verify `PLAYWRIGHT_CONNECTOR_AUTH_FILE_PATH` points to correct file
- Check JSON structure matches expected format (see Environment Configuration)
- Ensure credentials are valid and not expired
- Verify connector is enabled in Hyperswitch
- Check nested structure: `connector_1.connector_account_details`

### Port 8080 vs 8081 Issues

**Problem**: Tests fail with redirect errors or 3DS authentication fails

**Explanation**: The server runs on port 8081 but sometimes generates redirect URLs with port 8080

**Solution**: Already handled automatically in `RedirectionHelper.ts`:
- Auto-corrects 8080→8081 in redirect URLs
- Gracefully handles connection failures
- Skips verification when connection refused

If still having issues:
```bash
# Verify server port
netstat -an | grep LISTEN | grep 808

# Update .env to correct port
PLAYWRIGHT_BASEURL=http://127.0.0.1:8081
```

### 3DS Authentication Fails

**Problem**: Stripe or Cybersource 3DS tests timeout

**Debugging**:
```bash
# Run in headed mode to see what's happening
HEADLESS=false npx playwright test tests/e2e/spec/00005-ThreeDSAutoCapture.spec.ts

# Check for iframe loading issues (Stripe)
# Check for redirect URL issues (Cybersource)

# Enable verbose logging
VERBOSE_POOL=true npx playwright test
```

**Common Causes**:
- Redirect URL port mismatch (auto-fixed)
- Stripe iframe not loading (check test card)
- Cybersource redirect timeout (increase timeout)

### Tests Pass Locally but Fail in CI

**Problem**: Tests work on local machine but fail in GitHub Actions

**Solutions**:

1. **RAM constraints**:
   ```yaml
   env:
     CONTEXTS_PER_BROWSER: 10  # Reduce from 25
   ```

2. **Timeout issues**:
   ```yaml
   env:
     TEST_TIMEOUT: 180000  # Increase to 3 minutes
   ```

3. **Server startup delays**:
   ```yaml
   - name: Wait for server
     run: |
       timeout 60 bash -c 'until curl -f http://127.0.0.1:8081/health; do sleep 2; done'
   ```

4. **Headless mode**:
   ```yaml
   env:
     HEADLESS: true  # Ensure headless in CI
   ```

### Browser Launch Failures

**Problem**: "browserType.launch: Executable doesn't exist" or similar

**Solutions**:
```bash
# Reinstall browsers
npx playwright install chromium --with-deps

# Or install system dependencies (Linux)
npx playwright install-deps chromium

# Verify installation
npx playwright --version
ls -la ~/.cache/ms-playwright/
```

### TypeScript Errors

**Problem**: Type errors when running tests or during development

**Solutions**:
```bash
# Check types without running tests
npm run type-check

# Reinstall dependencies
rm -rf node_modules package-lock.json
npm install

# Ensure TypeScript version is correct
npm list typescript
```

---

## Migration from Cypress

### Key Differences

| Aspect | Cypress | Playwright |
|--------|---------|------------|
| **Test Runner** | Cypress Test Runner | Playwright Test |
| **Custom Commands** | `cy.merchantCreateCallTest()` | `apiHelpers.merchantCreateCall()` |
| **Global State** | Cypress tasks (`cy.task('setGlobalState')`) | Test fixtures (`globalState`) |
| **Parallelism** | Limited (spec files) | Advanced (browser contexts, browser pool) |
| **TypeScript** | Optional | First-class support |
| **Configuration** | `cypress.config.js` | `playwright.config.ts` |
| **Browser Support** | Chrome, Firefox, Edge | Chromium, Firefox, WebKit |
| **API Testing** | `cy.request()` | `request.post()` (APIRequestContext) |

### Test Conversion Example

**Cypress:**
```javascript
it('merchant create call', () => {
  cy.merchantCreateCallTest(fixtures.merchantCreateBody, globalState);
});
```

**Playwright:**
```typescript
test('merchant create call', async ({ request, globalState }) => {
  const apiHelpers = new ApiHelpers(request, globalState);
  await apiHelpers.merchantCreateCall(fixtures.merchantCreateBody);
});
```

---

## Contributing

When adding new tests:

1. **Follow naming convention**: `XXXXX-TestName.spec.ts` (e.g., `00030-MyNewTest.spec.ts`)
2. **Use TypeScript** for all test files
3. **Import from `fixtures/imports.ts`** for consistency
4. **Add test data to `tests/e2e/configs/`** (Stripe.ts, Cybersource.ts)
5. **Use `globalState` fixture** for state management
6. **Run type checking** before committing: `npm run type-check`
7. **Format code**: `npm run format`
8. **Test locally** with browser pool mode before pushing

### Code Quality

```bash
# Format code
npm run format

# Check formatting
npm run format:check

# Type checking
npm run type-check

# Lint (if configured)
npm run lint
```

---

## Additional Resources

- **Playwright Documentation**: https://playwright.dev/
- **Hyperswitch Documentation**: https://docs.hyperswitch.io/
- **GitHub Issues**: https://github.com/juspay/hyperswitch/issues

---

## License

Same as Hyperswitch main project

---

## Support

For issues, questions, or contributions:
- **Issues**: https://github.com/juspay/hyperswitch/issues
- **Discussions**: https://github.com/juspay/hyperswitch/discussions
- **Documentation**: https://docs.hyperswitch.io
