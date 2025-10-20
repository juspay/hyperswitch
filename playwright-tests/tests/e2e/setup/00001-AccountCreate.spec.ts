/**
 * Account Create Setup Test
 *
 * Creates merchant account and API key for testing
 * Ported from Cypress 00001-AccountCreate.cy.js
 */

import { test, ApiHelpers, fixtures } from '../../fixtures/imports';

test.describe.serial('Account Create Flow', () => {
  test('merchant create call', async ({ request, globalState }) => {
    const apiHelpers = new ApiHelpers(request, globalState);
    await apiHelpers.merchantCreateCall(fixtures.merchantCreateBody);
  });

  test('API key create call', async ({ request, globalState }) => {
    const apiHelpers = new ApiHelpers(request, globalState);
    await apiHelpers.apiKeyCreateTest(fixtures.apiKeyCreateBody);
  });
});
