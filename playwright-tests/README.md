# Playwright E2E Tests for Hyperswitch

Playwright-based end-to-end testing framework for Hyperswitch payment processing, converted from Cypress.

## Architecture

### Test Execution Strategy

Tests are organized into two phases to optimize performance while maintaining reliability:

#### Phase 1: Sequential Setup (Tests 00000-00003)
- **00000-CoreFlows**: Creates merchant account, API keys, customer, and connector
- **00001-AccountCreate**: Additional account configuration
- **00002-CustomerCreate**: Customer management tests
- **00003-ConnectorCreate**: Connector setup and configuration

These tests run **sequentially** using Playwright's project dependencies to create shared global state stored in `test-state.json`.

#### Phase 2: Parallel Execution (Tests 0004+)
- Tests run in **20 isolated browser contexts** (tabs) per connector
- Each connector (Stripe, Cybersource) runs independently in parallel
- Uses shared state from setup phase
- Estimated performance: **~2.5 minutes** vs Cypress **~41.5 minutes** (16.6x faster)

### RAM Usage

- **Per connector**: ~5.8 GB (20 browser contexts)
- **Total (Stripe + Cybersource)**: ~11.6 GB
- **GitHub CI runners** (16GB): Safe with 4.4 GB margin
- **Local development**: Headful mode uses slightly more RAM

## Prerequisites

- **Node.js**: v18+ recommended
- **npm**: v8+
- **Environment Variables**:
  - `PLAYWRIGHT_BASEURL`: Hyperswitch API base URL (default: `http://localhost:8080`)
  - `PLAYWRIGHT_ADMINAPIKEY`: Admin API key for authentication
  - `PLAYWRIGHT_CONNECTOR_AUTH_FILE_PATH`: Path to connector credentials JSON file
  - `PLAYWRIGHT_CONNECTOR`: Connector to test (default: `stripe`)

## Installation

```bash
cd playwright-tests
npm install
npx playwright install
```

## Running Tests

### Local Development

```bash
# Run all tests (sequential setup + parallel execution)
npm test

# Run in headed mode (see browsers)
npm run test:headed

# Run specific connector
npm run test:stripe
npm run test:cybersource

# Run only setup tests
npx playwright test --project=1-core-setup --project=2-account-setup --project=3-customer-setup --project=4-connector-setup
```

### Using the Shell Script

```bash
# Make script executable (first time only)
chmod +x scripts/execute_playwright.sh

# Run Stripe tests
./scripts/execute_playwright.sh stripe

# Run Cybersource tests
./scripts/execute_playwright.sh cybersource
```

### CI/CD (GitHub Actions)

```bash
export PLAYWRIGHT_BASEURL="https://your-api-endpoint.com"
export PLAYWRIGHT_ADMINAPIKEY="your-admin-api-key"
export PLAYWRIGHT_CONNECTOR_AUTH_FILE_PATH="./connector-creds.json"
export PLAYWRIGHT_CONNECTOR="stripe"

npm test
```

## Project Structure

```
playwright-tests/
├── global-setup.ts              # Initialize test-state.json before all tests
├── global-teardown.ts           # Cleanup and archive state after tests
├── playwright.config.ts         # Main Playwright configuration
├── package.json                 # Dependencies and scripts
├── tsconfig.json                # TypeScript configuration
├── scripts/
│   └── execute_playwright.sh    # Test execution script
├── tests/
│   ├── e2e/
│   │   ├── configs/             # Connector configurations
│   │   │   ├── ConnectorTypes.ts
│   │   │   ├── Commons.ts
│   │   │   ├── Stripe.ts
│   │   │   └── Cybersource.ts
│   │   ├── setup/               # Sequential setup tests (0000-0003)
│   │   │   └── 00000-CoreFlows.spec.ts
│   │   └── spec/                # Parallel test files (0004+)
│   ├── fixtures/                # Shared test fixtures
│   │   ├── imports.ts           # Barrel export for all test utilities
│   │   ├── test-data.ts         # Request body fixtures
│   │   └── test-fixtures.ts     # Playwright fixtures (globalState)
│   ├── helpers/                 # API helper functions
│   │   └── ApiHelpers.ts        # HTTP request helpers (ported from Cypress commands)
│   └── utils/                   # Utility functions
│       ├── State.ts             # Global state management
│       └── RequestBodyUtils.ts  # Request body utilities
└── test-state.json              # Shared state (generated at runtime)
```

## Key Files Explained

### `playwright.config.ts`
Configures:
- 2 workers (one per connector) for parallel execution
- Project dependencies to enforce sequential setup
- RAM-optimized settings for GitHub CI
- Headless/headful mode support
- HTML/JSON/GitHub reporters

### `tests/fixtures/test-fixtures.ts`
Provides `globalState` fixture that:
- Loads state from `test-state.json`
- Makes state available to all tests
- Saves state after setup tests complete
- Enables state sharing across test phases

### `tests/helpers/ApiHelpers.ts`
TypeScript port of Cypress custom commands:
- `merchantCreateCall()`, `apiKeyCreateTest()`, `createCustomerCall()`, etc.
- Uses Playwright's `APIRequestContext` for HTTP requests
- Integrates with `globalState` for data persistence

### `tests/e2e/configs/`
Connector test configurations:
- **Commons.ts**: Shared card details, mandate data, customer acceptance
- **Stripe.ts**: Stripe-specific test data and expected responses
- **Cybersource.ts**: Cybersource-specific test data
- **ConnectorTypes.ts**: TypeScript interfaces for type safety

## Writing Tests

### Example: Basic API Test

```typescript
import { test, expect, ApiHelpers, fixtures } from '../../fixtures/imports';

test.describe.serial('My Test Suite', () => {
  test('create merchant', async ({ request, globalState }) => {
    const apiHelpers = new ApiHelpers(request, globalState);
    await apiHelpers.merchantCreateCall(fixtures.merchantCreateBody);

    // Assertions
    expect(globalState.get('merchantId')).toBeTruthy();
  });

  test('retrieve merchant', async ({ request, globalState }) => {
    const apiHelpers = new ApiHelpers(request, globalState);
    await apiHelpers.merchantRetrieveCall();
  });
});
```

### Accessing Global State

```typescript
test('my test', async ({ globalState }) => {
  // Get values
  const merchantId = globalState.get('merchantId');
  const apiKey = globalState.get('apiKey');

  // Set values (only in setup tests)
  globalState.set('customerId', 'cust_123');

  // Check existence
  if (globalState.has('connectorId')) {
    // ...
  }
});
```

## Environment Configuration

### Required Environment Variables

Create a `.env` file (or set environment variables):

```bash
# API Configuration
PLAYWRIGHT_BASEURL=http://localhost:8080
PLAYWRIGHT_ADMINAPIKEY=your_admin_api_key_here

# Connector Configuration
PLAYWRIGHT_CONNECTOR=stripe
PLAYWRIGHT_CONNECTOR_AUTH_FILE_PATH=./connector-creds.json

# Optional: Execution Mode
HEADLESS=true
CI=false
```

### Connector Credentials File

The `PLAYWRIGHT_CONNECTOR_AUTH_FILE_PATH` should point to a JSON file with this structure:

```json
{
  "stripe": {
    "connector_account_details": {
      "auth_type": "BodyKey",
      "api_key": "your_stripe_secret_key"
    }
  },
  "cybersource": {
    "connector_account_details": {
      "auth_type": "SignatureKey",
      "api_key": "your_merchant_id",
      "key1": "your_secret_key",
      "api_secret": "your_shared_secret"
    }
  }
}
```

## Debugging

### View Test Results

```bash
# Open HTML report (auto-generated after test run)
npx playwright show-report

# View JSON results
cat test-results/results.json
```

### Run Tests in Debug Mode

```bash
# Debug specific test
PWDEBUG=1 npx playwright test tests/e2e/setup/00000-CoreFlows.spec.ts

# Run in headed mode with slow motion
HEADLESS=false npx playwright test --project=1-core-setup
```

### Check Global State

```bash
# View current state
cat test-state.json

# View archived states
ls -la test-results/test-state-*.json
```

## Migration from Cypress

### Key Differences

| Aspect | Cypress | Playwright |
|--------|---------|------------|
| **Test Runner** | Cypress Test Runner | Playwright Test |
| **Custom Commands** | `cy.merchantCreateCallTest()` | `apiHelpers.merchantCreateCall()` |
| **Global State** | Cypress tasks (`cy.task('setGlobalState')`) | Test fixtures (`globalState`) |
| **Parallelism** | Limited (spec files) | Native (browser contexts) |
| **TypeScript** | Optional | First-class support |
| **Configuration** | `cypress.config.js` | `playwright.config.ts` |

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

## Performance Comparison

| Scenario | Cypress (Sequential) | Playwright (Parallel) | Speedup |
|----------|----------------------|-----------------------|---------|
| **Full test suite** | ~41.5 minutes | ~2.5 minutes | **16.6x** |
| **Setup tests (0000-0003)** | ~2 minutes | ~2 minutes | 1x (sequential) |
| **Payment tests (0004+)** | ~39.5 minutes | ~30 seconds | **79x** |

## Troubleshooting

### Tests Fail with "State file not found"
- Ensure global-setup runs before tests
- Check that `test-state.json` is not in `.gitignore`
- Verify project dependencies are configured correctly

### RAM/Memory Issues
- Reduce worker count in `playwright.config.ts`
- Run fewer browser contexts (reduce from 20 to 10)
- Enable headless mode: `HEADLESS=true`

### API Connection Errors
- Verify `PLAYWRIGHT_BASEURL` is correct
- Check API server is running
- Ensure firewall allows connections

### Connector Authentication Fails
- Verify connector credentials file path
- Check JSON structure matches expected format
- Ensure credentials are valid and not expired

## Contributing

When adding new tests:
1. Follow the existing file naming convention (`XXXXX-TestName.spec.ts`)
2. Use TypeScript for all test files
3. Import from `fixtures/imports.ts` for consistency
4. Add appropriate test data to `fixtures/test-data.ts`
5. Use `globalState` fixture for state management
6. Run `npm run typecheck` before committing

## License

Same as Hyperswitch main project

## Support

- **Issues**: https://github.com/juspay/hyperswitch/issues
- **Documentation**: https://docs.hyperswitch.io
