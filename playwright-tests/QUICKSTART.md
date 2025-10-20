# Quick Start Guide - Playwright POC

Get up and running with the Playwright test framework in 5 minutes.

## Prerequisites

Before running tests, ensure you have:
- Node.js v18+ installed
- Access to a running Hyperswitch API instance
- Admin API key
- Connector credentials file

## Installation

```bash
# Navigate to playwright-tests directory
cd playwright-tests

# Install dependencies
npm install

# Install Playwright browsers
npx playwright install chromium
```

## Configuration

### 1. Set Environment Variables

Create a `.env` file or export these variables:

```bash
# Required
export PLAYWRIGHT_BASEURL="http://localhost:8080"
export PLAYWRIGHT_ADMINAPIKEY="your_admin_api_key_here"
export PLAYWRIGHT_CONNECTOR_AUTH_FILE_PATH="./connector-creds.json"

# Optional
export PLAYWRIGHT_CONNECTOR="stripe"  # or "cybersource"
export HEADLESS="true"  # Set to "false" to see browser windows
```

### 2. Create Connector Credentials File

Create `connector-creds.json` in the project root:

```json
{
  "stripe": {
    "connector_account_details": {
      "auth_type": "BodyKey",
      "api_key": "sk_test_your_stripe_key_here"
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

## Running Tests

### Run All Tests (Full Suite)

```bash
# Sequential setup + parallel execution
npm test
```

This will:
1. ✅ Run setup tests sequentially (00000-00003)
2. ✅ Run Stripe & Cybersource parallel tests simultaneously
3. ✅ Generate HTML report

### Run Specific Tests

```bash
# Run only setup tests
npx playwright test --grep "setup"

# Run only Stripe tests
npm run test:stripe

# Run only Cybersource tests
npm run test:cybersource

# Run in headed mode (see browsers)
npm run test:headed
```

### Using the Shell Script

```bash
# Make executable (first time only)
chmod +x scripts/execute_playwright.sh

# Run tests
./scripts/execute_playwright.sh stripe
```

## Verify Installation

Run a quick test to verify everything is working:

```bash
# Test TypeScript compilation
npx tsc --noEmit

# Run just the first setup test
npx playwright test tests/e2e/setup/00000-CoreFlows.spec.ts
```

Expected output:
```
✓ Global setup completed
✓ Running 1 test using 1 worker

  ✓ Core Flows › Merchant core flows › merchant create call (2s)
  ✓ Core Flows › Merchant core flows › merchant retrieve call (1s)
  ...

✓ All tests passed
```

## View Results

```bash
# Open HTML report
npx playwright show-report

# View test state
cat test-state.json
```

## Troubleshooting

### Tests fail immediately

**Problem**: Environment variables not set
**Solution**:
```bash
# Check variables are set
echo $PLAYWRIGHT_BASEURL
echo $PLAYWRIGHT_ADMINAPIKEY

# If empty, export them
export PLAYWRIGHT_BASEURL="http://localhost:8080"
export PLAYWRIGHT_ADMINAPIKEY="your_key"
```

### "Cannot find module" errors

**Problem**: Dependencies not installed
**Solution**:
```bash
cd playwright-tests
rm -rf node_modules package-lock.json
npm install
```

### Connector authentication fails

**Problem**: Invalid credentials file
**Solution**:
1. Verify `connector-creds.json` exists
2. Check JSON is valid: `cat connector-creds.json | jq`
3. Ensure connector credentials are correct

### Server connection errors

**Problem**: Hyperswitch API not accessible
**Solution**:
1. Verify API is running: `curl http://localhost:8080/health`
2. Check firewall/network settings
3. Verify PLAYWRIGHT_BASEURL is correct

## Next Steps

After successful test run:

1. **Extend API Helpers**: Add more methods to `tests/helpers/ApiHelpers.ts`
2. **Port More Tests**: Convert remaining Cypress tests to Playwright
3. **Add Assertions**: Enhance test validation with more `expect()` statements
4. **Optimize Performance**: Fine-tune worker count and timeouts
5. **CI Integration**: Add to GitHub Actions workflow

## File Structure Reference

```
playwright-tests/
├── tests/
│   ├── e2e/
│   │   ├── setup/          ← Sequential tests (00000-00003)
│   │   ├── spec/           ← Parallel tests (stripe, cybersource)
│   │   └── configs/        ← Connector configurations
│   ├── fixtures/           ← Test data and fixtures
│   └── helpers/            ← API helper functions
├── scripts/
│   └── execute_playwright.sh  ← Test execution script
├── playwright.config.ts    ← Main configuration
└── test-state.json         ← Shared state (auto-generated)
```

## Getting Help

- **Documentation**: See [README.md](./README.md) for detailed information
- **TypeScript Errors**: Run `npx tsc --noEmit` for type checking
- **Playwright Docs**: https://playwright.dev/docs/intro
- **Project Issues**: https://github.com/juspay/hyperswitch/issues

## Performance Expectations

| Test Phase | Duration | Details |
|------------|----------|---------|
| Setup (00000-00003) | ~2 minutes | Sequential execution |
| Parallel (Stripe + Cybersource) | ~30 seconds | 40 contexts total |
| **Total** | **~2.5 minutes** | 16.6x faster than Cypress |

## Success Criteria

✅ All setup tests pass
✅ Global state created (`test-state.json`)
✅ Parallel tests execute simultaneously
✅ HTML report generated
✅ No TypeScript errors

---

**Ready to scale?** Add more test scenarios by creating additional `test.describe` blocks in the parallel test files!
