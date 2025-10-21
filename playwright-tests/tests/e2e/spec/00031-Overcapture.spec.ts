/**
 * Overcapture Tests
 *
 * Tests overcapture functionality where more than the authorized amount can be captured.
 * This is useful for scenarios where additional charges (like tips, shipping fees) are
 * added after authorization.
 *
 * Flow:
 * 1. Create payment intent with enable_overcapture flag
 * 2. Confirm payment (manual capture)
 * 3. Capture more than the authorized amount
 * 4. Verify payment status
 */

import { test, expect } from '../../fixtures/imports';
import { ApiHelpers } from '../../helpers/ApiHelpers';
import * as fixtures from '../../fixtures/test-data';
import { getConnectorDetails, shouldContinueFurther, shouldIncludeConnector, CONNECTOR_LISTS } from '../configs/Payment/Utils';

test.describe.configure({ mode: 'parallel' });

test.describe.serial('Overcapture Pre-Auth Tests', () => {
  test.skip(({ globalState }) => {
    const connector = globalState.get('connectorId');
    // Skip if connector is not in the inclusion list
    return shouldIncludeConnector(connector, CONNECTOR_LISTS.OVERCAPTURE);
  });

  test('Overcapture Pre-Auth - Create, Confirm, Overcapture, Retrieve', async ({ request, globalState }) => {
    const api = new ApiHelpers(request, globalState);
    const connectorId = globalState.get('connectorId');

    // Create Payment Intent with overcapture enabled
    const createData = getConnectorDetails(connectorId)?.card_pm?.PaymentIntent;
    if (!createData) {
      test.skip();
      return;
    }

    const newData = {
      ...createData,
      Request: {
        ...createData.Request,
        enable_overcapture: true,
      },
    };

    await api.createPaymentIntent(
      {
        ...fixtures.createPaymentBody,
        ...newData,
        authentication_type: 'no_three_ds',
        capture_method: 'manual'
      },
      'PublishableKey'
    );

    if (!shouldContinueFurther(createData)) {
      test.skip();
      return;
    }

    // Confirm Payment with manual capture
    const confirmData = getConnectorDetails(connectorId)?.card_pm?.No3DSManualCapture;
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

    // Capture with overcapture amount
    const captureData = getConnectorDetails(connectorId)?.card_pm?.Overcapture;
    if (!captureData) {
      test.skip();
      return;
    }

    await api.capturePayment(fixtures.captureBody, captureData);

    if (!shouldContinueFurther(captureData)) {
      test.skip();
      return;
    }

    // Retrieve payment to verify overcapture
    const retrieveData = getConnectorDetails(connectorId)?.card_pm?.Overcapture;
    if (!retrieveData) {
      test.skip();
      return;
    }

    await api.retrievePayment(retrieveData);
  });
});
