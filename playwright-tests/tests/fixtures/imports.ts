/**
 * Centralized Imports for Playwright Tests
 *
 * Barrel export file for all fixtures, utilities, and helpers.
 * Import everything you need from this single file.
 *
 * Usage in test files:
 * import { test, expect, globalState, RequestBodyUtils } from './fixtures/imports';
 */

// Core test fixtures
export { test, expect } from './test-fixtures';

// State management
export { State } from '../utils/State';
export type { StateData } from '../utils/State';

// Request body utilities
export * as RequestBodyUtils from '../utils/RequestBodyUtils';
export {
  setClientSecret,
  setCardNo,
  setApiKey,
  generateRandomString,
  setMerchantId,
  isoTimeTomorrow,
  validateEnv,
  generateRandomEmail,
  generateRandomName,
  isCI,
  getTimeoutMultiplier,
} from '../utils/RequestBodyUtils';

// API helpers
export { ApiHelpers } from '../helpers/ApiHelpers';

// Test data fixtures
export * as fixtures from './test-data';
export {
  merchantCreateBody,
  customerCreateBody,
  customerUpdateBody,
  createPaymentBody,
  confirmBody,
  createConfirmPaymentBody,
  captureBody,
  voidBody,
  refundBody,
  citConfirmBody,
  pmIdConfirmBody,
  ntidConfirmBody,
} from './test-data';
