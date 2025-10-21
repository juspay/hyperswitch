/**
 * Incremental Authorization Tests
 *
 * Tests incremental authorization feature where additional amounts can be authorized
 * after the initial authorization.
 *
 * Currently known to be supported by: Cybersource
 */

import { test, expect } from '../../fixtures/imports';
import { ApiHelpers } from '../../helpers/ApiHelpers';
import * as fixtures from '../../fixtures/test-data';
import { getConnectorDetails, shouldContinueFurther, shouldIncludeConnector, CONNECTOR_LISTS } from '../configs/Payment/Utils';

test.describe.configure({ mode: 'parallel' });

test.describe.serial('Incremental Authorization - Payment Tests', () => {
  test.skip(({ globalState }) => {
    const connector = globalState.get('connectorId');
    // Skip if connector is not in the inclusion list
    return shouldIncludeConnector(connector, CONNECTOR_LISTS.INCREMENTAL_AUTH);
  });

  test('Incremental Pre-Auth - Create, Confirm, Increment, Capture', async ({ request, globalState }) => {
    const api = new ApiHelpers(request, globalState);
    const connectorId = globalState.get('connectorId');

    // Create Payment Intent with incremental authorization enabled
    const createData = getConnectorDetails(connectorId)?.card_pm?.PaymentIntentOffSession;
    if (!createData) {
      test.skip();
      return;
    }

    const newData = {
      ...createData,
      Request: {
        ...createData.Request,
        request_incremental_authorization: true,
      },
    };

    await api.createPaymentIntent(
      {
        ...fixtures.createPaymentBody,
        ...newData,
        customer_id: globalState.get('customerId'),
        authentication_type: 'no_three_ds',
        capture_method: 'manual'
      },
      'PublishableKey'
    );

    if (!shouldContinueFurther(createData)) {
      test.skip();
      return;
    }

    // Confirm Payment Intent
    const confirmData = getConnectorDetails(connectorId)?.card_pm?.SaveCardUseNo3DSManualCaptureOffSession;
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

    // Incremental Authorization
    const incrementalData = getConnectorDetails(connectorId)?.card_pm?.IncrementalAuth;
    if (!incrementalData) {
      test.skip();
      return;
    }

    const paymentId = globalState.get('paymentId');
    await api.incrementalAuth(paymentId, incrementalData);

    if (!shouldContinueFurther(incrementalData)) {
      test.skip();
      return;
    }

    // Capture Payment with increased amount
    const captureData = getConnectorDetails(connectorId)?.card_pm?.Capture;
    if (!captureData) {
      test.skip();
      return;
    }

    const newCaptureData = {
      ...captureData,
      Request: {
        amount_to_capture: (captureData.Request?.amount_to_capture || 0) + 2000,
      },
      Response: captureData.ResponseCustom || captureData.Response,
    };

    await api.capturePayment(fixtures.captureBody, newCaptureData);

    if (!shouldContinueFurther(captureData)) {
      test.skip();
      return;
    }
  });
});

test.describe.serial('Incremental Authorization - Saved Card Tests', () => {
  test.skip(({ globalState }) => {
    const connector = globalState.get('connectorId');
    // Skip if connector is not cybersource (only known to support saved card incremental auth)
    return connector !== 'cybersource';
  });

  test('Saved Card Incremental Pre-Auth - List PM, Create, Confirm, Increment, Capture', async ({ request, globalState }) => {
    const api = new ApiHelpers(request, globalState);
    const connectorId = globalState.get('connectorId');

    // List customer payment methods
    await api.listCustomerPaymentMethods();

    // Create Payment Intent
    const createData = getConnectorDetails(connectorId)?.card_pm?.PaymentIntentOffSession;
    if (!createData) {
      test.skip();
      return;
    }

    await api.createPaymentIntent(
      {
        ...fixtures.createPaymentBody,
        ...createData,
        customer_id: globalState.get('customerId'),
        authentication_type: 'no_three_ds',
        capture_method: 'manual'
      },
      'PublishableKey'
    );

    if (!shouldContinueFurther(createData)) {
      test.skip();
      return;
    }

    // Confirm Payment with saved card
    const confirmData = getConnectorDetails(connectorId)?.card_pm?.SaveCardUseNo3DSManualCaptureOffSession;
    if (!confirmData) {
      test.skip();
      return;
    }

    await api.saveCardConfirm(fixtures.confirmBody, confirmData);

    if (!shouldContinueFurther(confirmData)) {
      test.skip();
      return;
    }

    // Incremental Authorization
    const incrementalData = getConnectorDetails(connectorId)?.card_pm?.IncrementalAuth;
    if (!incrementalData) {
      test.skip();
      return;
    }

    const paymentId = globalState.get('paymentId');
    await api.incrementalAuth(paymentId, incrementalData);

    if (!shouldContinueFurther(incrementalData)) {
      test.skip();
      return;
    }

    // Capture Payment
    const captureData = getConnectorDetails(connectorId)?.card_pm?.Capture;
    if (!captureData) {
      test.skip();
      return;
    }

    await api.capturePayment(fixtures.captureBody, captureData);

    if (!shouldContinueFurther(captureData)) {
      test.skip();
      return;
    }
  });
});
