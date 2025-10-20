/**
 * DDC Race Condition Tests
 *
 * These tests ensure that device data collection works properly during payment authentication
 * and prevents issues when multiple requests happen at the same time.
 *
 * Server-side validation:
 * - Checks that our backend properly handles duplicate device data submissions
 * - Makes sure that once device data is collected, any additional attempts are rejected
 *
 * Client-side validation:
 * - Verifies that the payment page prevents users from accidentally submitting data twice
 * - Ensures that even if someone clicks multiple times, only one submission goes through
 * - Tests that our JavaScript protection works as expected
 */

import { test, expect } from '../../fixtures/imports';
import { ApiHelpers } from '../../helpers/ApiHelpers';
import * as fixtures from '../../fixtures/test-data';
import { getConnectorDetails, shouldContinueFurther, shouldIncludeConnector, CONNECTOR_LISTS } from '../configs/Payment/Utils';

test.describe.configure({ mode: 'parallel' });

test.describe('DDC Race Condition Tests', () => {
  test.skip(({ globalState }) => {
    const connector = globalState.get('connectorId');

    // Skip if connector is not in the inclusion list
    if (shouldIncludeConnector(connector, CONNECTOR_LISTS.DDC_RACE_CONDITION)) {
      return true;
    }

    // Check for required state keys
    const requiredKeys = ['merchantId', 'apiKey', 'publishableKey', 'baseUrl'];
    const missingKeys = requiredKeys.filter(key => !globalState.get(key));

    if (missingKeys.length > 0) {
      console.log(`Skipping DDC tests - missing critical state: ${missingKeys.join(', ')}`);
      return true;
    }

    const merchantConnectorId = globalState.get('merchantConnectorId');
    if (!merchantConnectorId) {
      console.log('Warning: merchantConnectorId missing - may indicate connector configuration issue');
    }

    return false;
  });

  test('Server-side DDC race condition handling', async ({ request, globalState }) => {
    const api = new ApiHelpers(request, globalState);
    const connectorId = globalState.get('connectorId');

    // Ensure customer exists
    if (!globalState.get('customerId')) {
      await api.createCustomerCall(fixtures.customerCreateBody);
    }

    // Ensure profile ID is set
    if (!globalState.get('profileId')) {
      const defaultProfileId = globalState.get('defaultProfileId');
      if (defaultProfileId) {
        globalState.set('profileId', defaultProfileId);
      }
    }

    // Create Payment Intent
    const createData = getConnectorDetails(connectorId)?.card_pm?.PaymentIntent;
    if (!createData) {
      test.skip();
      return;
    }

    await api.createPaymentIntent(
      {
        ...fixtures.createPaymentBody,
        ...createData,
        authentication_type: 'three_ds',
        capture_method: 'automatic'
      },
      'PublishableKey'
    );

    if (!shouldContinueFurther(createData)) {
      test.skip();
      return;
    }

    // Confirm Payment
    const confirmData = getConnectorDetails(connectorId)?.card_pm?.DDCRaceConditionServerSide;
    if (!confirmData) {
      test.skip();
      return;
    }

    await api.confirmPayment(
      {
        ...fixtures.confirmBody,
        ...confirmData
      },
      'PublishableKey'
    );

    if (!shouldContinueFurther(confirmData)) {
      test.skip();
      return;
    }

    // Test server-side race condition
    const paymentId = globalState.get('paymentId');
    await api.ddcServerSideRaceCondition(paymentId, confirmData);
  });

  test('Client-side DDC race condition handling', async ({ request, globalState }) => {
    const api = new ApiHelpers(request, globalState);
    const connectorId = globalState.get('connectorId');

    // Reset payment-specific state
    globalState.set('clientSecret', null);
    globalState.set('nextActionUrl', null);

    // Ensure customer exists
    if (!globalState.get('customerId')) {
      await api.createCustomerCall(fixtures.customerCreateBody);
    }

    // Ensure profile ID is set
    if (!globalState.get('profileId')) {
      const defaultProfileId = globalState.get('defaultProfileId');
      if (defaultProfileId) {
        globalState.set('profileId', defaultProfileId);
      }
    }

    // Create Payment Intent
    const createData = getConnectorDetails(connectorId)?.card_pm?.PaymentIntent;
    if (!createData) {
      test.skip();
      return;
    }

    await api.createPaymentIntent(
      {
        ...fixtures.createPaymentBody,
        ...createData,
        authentication_type: 'three_ds',
        capture_method: 'automatic'
      },
      'PublishableKey'
    );

    if (!shouldContinueFurther(createData)) {
      test.skip();
      return;
    }

    // Confirm Payment
    const confirmData = getConnectorDetails(connectorId)?.card_pm?.DDCRaceConditionClientSide;
    if (!confirmData) {
      test.skip();
      return;
    }

    await api.confirmPayment(
      {
        ...fixtures.confirmBody,
        ...confirmData
      },
      'PublishableKey'
    );

    if (!shouldContinueFurther(confirmData)) {
      test.skip();
      return;
    }

    // Test client-side race condition
    const paymentId = globalState.get('paymentId');
    await api.ddcClientSideRaceCondition(paymentId, confirmData);
  });
});
