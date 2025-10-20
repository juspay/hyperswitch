/**
 * Connector Create Setup Test
 *
 * Creates merchant connector account for testing
 * Ported from Cypress 00003-ConnectorCreate.cy.js
 */

import { test, ApiHelpers, fixtures } from '../../fixtures/imports';
import { payment_methods_enabled } from '../configs/Commons';

test.describe.serial('Connector Account Create Flow', () => {
  test('create merchant connector account', async ({ request, globalState }) => {
    const apiHelpers = new ApiHelpers(request, globalState);
    await apiHelpers.createConnectorCall(
      'payment_processor',
      fixtures.createConnectorBody,
      payment_methods_enabled
    );
  });

  // TODO: Implement multiple connector support
  // If MULTIPLE_CONNECTORS flag is set in state, create additional:
  // 1. Business profile (apiHelpers.createBusinessProfile)
  // 2. Merchant connector account for second connector
  test.skip('create additional connector for multi-connector setup', async ({
    request,
    globalState,
  }) => {
    const multipleConnectors = globalState.get('MULTIPLE_CONNECTORS');

    if (!multipleConnectors) {
      return;
    }

    const apiHelpers = new ApiHelpers(request, globalState);

    // Create business profile
    await apiHelpers.createBusinessProfile(
      fixtures.businessProfile,
      'profile2'
    );

    // Create second connector
    await apiHelpers.createConnectorCall(
      'payment_processor',
      fixtures.createConnectorBody,
      payment_methods_enabled,
      'profile2',
      'merchantConnector2'
    );
  });
});
