/**
 * Core Flows Setup Test
 *
 * Tests merchant, API key, customer, and connector core flows
 * Ported from Cypress 00000-CoreFlows.cy.js
 */

import { test, ApiHelpers, fixtures } from '../../fixtures/imports';
import { payment_methods_enabled } from '../configs/Commons';

test.describe.serial('Core Flows', () => {
  test.describe('Merchant core flows', () => {
    test('merchant create call', async ({ request, globalState }) => {
      const apiHelpers = new ApiHelpers(request, globalState);
      await apiHelpers.merchantCreateCall(fixtures.merchantCreateBody);
    });

    test('merchant retrieve call', async ({ request, globalState }) => {
      const apiHelpers = new ApiHelpers(request, globalState);
      await apiHelpers.merchantRetrieveCall();
    });

    test('API key create call', async ({ request, globalState }) => {
      const apiHelpers = new ApiHelpers(request, globalState);
      await apiHelpers.apiKeyCreateTest(fixtures.apiKeyCreateBody);
    });

    test('API key retrieve call', async ({ request, globalState }) => {
      const apiHelpers = new ApiHelpers(request, globalState);
      await apiHelpers.apiKeyRetrieveCall();
    });
  });

  test.describe('Customer core flows', () => {
    test('customer create call', async ({ request, globalState }) => {
      const apiHelpers = new ApiHelpers(request, globalState);
      await apiHelpers.createCustomerCall(fixtures.customerCreateBody);
    });

    test('customer retrieve call', async ({ request, globalState }) => {
      const apiHelpers = new ApiHelpers(request, globalState);
      await apiHelpers.customerRetrieveCall();
    });
  });

  test.describe('Merchant Connector Account core flows', () => {
    test('connector create call', async ({ request, globalState }) => {
      const apiHelpers = new ApiHelpers(request, globalState);
      await apiHelpers.createConnectorCall(
        'payment_processor',
        fixtures.createConnectorBody,
        payment_methods_enabled
      );
    });

    test('connector retrieve call', async ({ request, globalState }) => {
      const apiHelpers = new ApiHelpers(request, globalState);
      await apiHelpers.connectorRetrieveCall();
    });
  });

  test.describe('List Connector Feature Matrix', () => {
    test('list connector feature matrix call', async ({
      request,
      globalState,
    }) => {
      const apiHelpers = new ApiHelpers(request, globalState);
      await apiHelpers.listConnectorsFeatureMatrix();
    });
  });
});
