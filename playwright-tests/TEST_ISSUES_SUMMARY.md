# Playwright Test Suite - Issues Found and Fixed

## Summary

**Setup Tests:** ✅ 13/13 passing  
**Parallel Tests:** ⚠️ Failing due to connector config loading issue  

---

## Issues Fixed

### 1. ✅ Connector Duplicate Label Error (CRITICAL FIX)
**Problem:** Tests failed with "connector with profile_id and connector_label already exists"  
**Root Cause:** Multiple test runs created connectors with same label  
**Solution:** Added unique random suffix to connector_label in `ApiHelpers.ts:463`
```typescript
const connectorLabel = `${connectorId}_${RequestBodyUtils.generateRandomString('')}`;
createConnectorBody.connector_label = connectorLabel;
```

### 2. ✅ IPv6 Connection Error
**Problem:** `ECONNREFUSED ::1:8081`  
**Solution:** Changed baseURL from `localhost` to `127.0.0.1` in `.env`

### 3. ✅ Setup Tests Not Found
**Problem:** Setup directory tests weren't discovered  
**Solution:** Changed `testDir` from `'./tests/e2e/spec'` to `'./tests/e2e'`

### 4. ✅ ES Module __dirname Error
**Problem:** `ReferenceError: __dirname is not defined in ES module scope`  
**Solution:** Added ES module-compatible __dirname using `import.meta.url`

### 5. ✅ Unsupported baseUrl Error
**Problem:** validateEnv didn't recognize '127.0.0.1'  
**Solution:** Added '127.0.0.1' to keyPrefixes in `RequestBodyUtils.ts`

### 6. ✅ Nested Connector Structure
**Problem:** Connector credentials had nested structure (connector_1, connector_2)  
**Solution:** Fixed `getValueByKey()` to handle nested connectors

---

## Current Issue

### 🔧 Connector Config Loading in Test Files

**Problem:**  
Test files use `require()` to dynamically load TypeScript connector configs:
```typescript
const getConnectorConfig = (connectorId: string) => {
  try {
    return require(`../configs/${connectorId.charAt(0).toUpperCase() + connectorId.slice(1)}`).connectorDetails;
  } catch {
    return null;
  }
};
```

This doesn't work because:
- Files are TypeScript (`.ts`) not JavaScript
- Using ES modules (`type: "module"` in package.json)
- `require()` doesn't work with ES modules

**Impact:**  
- `getConnectorConfig()` returns `null`
- Tests check `if (!connectorConfig) test.skip()`
- All payment tests are skipped
- Tests that don't check dependencies (like `payment_methods-call-test`) run with missing data and fail

**Solution Applied:**  
Changed dynamic `require()` to static imports in `00004-NoThreeDSAutoCapture.spec.ts`:
```typescript
import { connectorDetails as stripeConfig } from '../configs/Stripe';
import { connectorDetails as cybersourceConfig } from '../configs/Cybersource';

const getConnectorConfig = (connectorId: string) => {
  const configs: Record<string, typeof stripeConfig> = {
    stripe: stripeConfig,
    cybersource: cybersourceConfig,
  };
  return configs[connectorId.toLowerCase()] || null;
};
```

**Remaining Work:**  
Apply this same fix to all other test files (00005-00029) that use dynamic `require()`.

---

## Test Files That Need Fixing

All spec test files from 00004-00029 need the connector config import updated:
```
tests/e2e/spec/00004-NoThreeDSAutoCapture.spec.ts ✅ FIXED
tests/e2e/spec/00005-ThreeDSAutoCapture.spec.ts
tests/e2e/spec/00006-NoThreeDSManualCapture.spec.ts
tests/e2e/spec/00007-VoidPayment.spec.ts
tests/e2e/spec/00008-SyncPayment.spec.ts
... (and others)
```

---

## Next Steps

1. Apply the connector config import fix to all remaining test files
2. Run full test suite to verify all tests pass
3. Document any API-specific issues found during testing

