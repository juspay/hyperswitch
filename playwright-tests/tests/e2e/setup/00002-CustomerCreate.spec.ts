/**
 * Customer Create Setup Test
 *
 * Creates customer for testing
 * Ported from Cypress 00002-CustomerCreate.cy.js
 */

import { test, ApiHelpers, fixtures } from '../../fixtures/imports';

test.describe.serial('Customer Create Flow', () => {
  test('customer create call', async ({ request, globalState }) => {
    const apiHelpers = new ApiHelpers(request, globalState);
    await apiHelpers.createCustomerCall(fixtures.customerCreateBody);
  });
});
