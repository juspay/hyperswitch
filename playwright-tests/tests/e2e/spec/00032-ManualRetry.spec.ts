/**
 * Manual Retry Tests
 *
 * Tests manual retry functionality which allows retrying failed payments.
 * Includes tests for:
 * - Manual retry disabled (should throw error on retry)
 * - Manual retry enabled (should allow retry)
 * - Manual retry cutoff (should reject retry after expiration)
 * - First confirm after cutoff (should succeed as it's not a retry)
 */

import { test, expect } from '../../fixtures/imports';
import { ApiHelpers } from '../../helpers/ApiHelpers';
import * as fixtures from '../../fixtures/test-data';
import { getConnectorDetails, shouldContinueFurther, shouldIncludeConnector, CONNECTOR_LISTS } from '../configs/Payment/Utils';

const MANUAL_RETRY_EXPIRATION = 35000; // 35 seconds

test.describe.configure({ mode: 'parallel' });

test.describe.serial('Manual Retry Disabled Tests', () => {
  test.skip(({ globalState }) => {
    const connectorId = globalState.get('connectorId');
    return shouldIncludeConnector(connectorId, CONNECTOR_LISTS.MANUAL_RETRY);
  });

  test('Manual retry disabled - First confirm fails, second confirm throws error', async ({ request, globalState }) => {
    const api = new ApiHelpers(request, globalState);
    const connectorId = globalState.get('connectorId');

    // Update Profile with is_manual_retry_enabled = false
    const updateBusinessProfileBody = {
      is_manual_retry_enabled: false,
    };

    const profileId = globalState.get('profileId');
    await api.updateBusinessProfile(profileId, {
      ...updateBusinessProfileBody,
      is_connector_agnostic_enabled: false,
      collect_billing_details_from_wallet_connector: false,
      collect_shipping_details_from_wallet_connector: false,
      always_collect_billing_details_from_wallet_connector: false,
      always_collect_shipping_details_from_wallet_connector: false,
    });

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
        authentication_type: 'no_three_ds',
        capture_method: 'automatic'
      },
      'PublishableKey'
    );

    if (!shouldContinueFurther(createData)) {
      test.skip();
      return;
    }

    // First Confirm with Failed Status
    const failData = getConnectorDetails(connectorId)?.card_pm?.No3DSFailPayment;
    if (!failData) {
      test.skip();
      return;
    }

    await api.confirmPayment(
      {
        ...fixtures.confirmBody,
        ...failData
      },
      'PublishableKey'
    );

    // Second Confirm should throw error (manual retry disabled)
    const retryData = getConnectorDetails(connectorId)?.card_pm?.ManualRetryPaymentDisabled;
    if (!retryData) {
      test.skip();
      return;
    }

    await api.confirmPayment(
      {
        ...fixtures.confirmBody,
        ...retryData
      },
      'PublishableKey'
    );

    if (!shouldContinueFurther(retryData)) {
      test.skip();
      return;
    }
  });
});

test.describe.serial('Manual Retry Enabled Tests', () => {
  test.skip(({ globalState }) => {
    const connectorId = globalState.get('connectorId');
    return shouldIncludeConnector(connectorId, CONNECTOR_LISTS.MANUAL_RETRY);
  });

  test('Manual retry enabled - First confirm fails, retry confirm succeeds', async ({ request, globalState }) => {
    const api = new ApiHelpers(request, globalState);
    const connectorId = globalState.get('connectorId');

    // Update Profile with is_manual_retry_enabled = true
    const updateBusinessProfileBody = {
      is_manual_retry_enabled: true,
    };

    const profileId = globalState.get('profileId');
    await api.updateBusinessProfile(profileId, {
      ...updateBusinessProfileBody,
      is_connector_agnostic_enabled: false,
      collect_billing_details_from_wallet_connector: false,
      collect_shipping_details_from_wallet_connector: false,
      always_collect_billing_details_from_wallet_connector: false,
      always_collect_shipping_details_from_wallet_connector: false,
    });

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
        authentication_type: 'no_three_ds',
        capture_method: 'automatic'
      },
      'PublishableKey'
    );

    if (!shouldContinueFurther(createData)) {
      test.skip();
      return;
    }

    // First Confirm with Failed Status
    const failData = getConnectorDetails(connectorId)?.card_pm?.No3DSFailPayment;
    if (!failData) {
      test.skip();
      return;
    }

    await api.confirmPayment(
      {
        ...fixtures.confirmBody,
        ...failData
      },
      'PublishableKey'
    );

    // Retry Confirm with Successful Status
    const retryData = getConnectorDetails(connectorId)?.card_pm?.ManualRetryPaymentEnabled;
    if (!retryData) {
      test.skip();
      return;
    }

    await api.confirmPayment(
      {
        ...fixtures.confirmBody,
        ...retryData
      },
      'PublishableKey'
    );

    if (!shouldContinueFurther(retryData)) {
      test.skip();
      return;
    }
  });
});

test.describe.serial('Manual Retry Cutoff Tests', () => {
  test.skip(({ globalState }) => {
    const connectorId = globalState.get('connectorId');
    return shouldIncludeConnector(connectorId, CONNECTOR_LISTS.MANUAL_RETRY);
  });

  test('Manual retry cutoff - Retry after expiration should throw error', async ({ request, globalState }) => {
    const api = new ApiHelpers(request, globalState);
    const connectorId = globalState.get('connectorId');

    // Update Profile with is_manual_retry_enabled = true and session_expiry = 60
    const updateBusinessProfileBody = {
      is_manual_retry_enabled: true,
      session_expiry: 60,
    };

    const profileId = globalState.get('profileId');
    await api.updateBusinessProfile(profileId, {
      ...updateBusinessProfileBody,
      is_connector_agnostic_enabled: false,
      collect_billing_details_from_wallet_connector: false,
      collect_shipping_details_from_wallet_connector: false,
      always_collect_billing_details_from_wallet_connector: false,
      always_collect_shipping_details_from_wallet_connector: false,
    });

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
        authentication_type: 'no_three_ds',
        capture_method: 'automatic'
      },
      'PublishableKey'
    );

    if (!shouldContinueFurther(createData)) {
      test.skip();
      return;
    }

    // First Confirm with Failed Status
    const failData = getConnectorDetails(connectorId)?.card_pm?.No3DSFailPayment;
    if (!failData) {
      test.skip();
      return;
    }

    await api.confirmPayment(
      {
        ...fixtures.confirmBody,
        ...failData
      },
      'PublishableKey'
    );

    // Wait for manual retry cutoff to expire
    await new Promise(resolve => setTimeout(resolve, MANUAL_RETRY_EXPIRATION));

    // Retry Confirm after cutoff (should throw error)
    const retryData = getConnectorDetails(connectorId)?.card_pm?.ManualRetryPaymentCutoffExpired;
    if (!retryData) {
      test.skip();
      return;
    }

    await api.confirmPayment(
      {
        ...fixtures.confirmBody,
        ...retryData
      },
      'PublishableKey'
    );

    if (!shouldContinueFurther(retryData)) {
      test.skip();
      return;
    }
  });
});

test.describe.serial('First Confirm After Cutoff Tests', () => {
  test.skip(({ globalState }) => {
    const connectorId = globalState.get('connectorId');
    return shouldIncludeConnector(connectorId, CONNECTOR_LISTS.MANUAL_RETRY);
  });

  test('First confirm after manual retry cutoff should succeed', async ({ request, globalState }) => {
    const api = new ApiHelpers(request, globalState);
    const connectorId = globalState.get('connectorId');

    // Update Profile with is_manual_retry_enabled = true and session_expiry = 60
    const updateBusinessProfileBody = {
      is_manual_retry_enabled: true,
      session_expiry: 60,
    };

    const profileId = globalState.get('profileId');
    await api.updateBusinessProfile(profileId, {
      ...updateBusinessProfileBody,
      is_connector_agnostic_enabled: false,
      collect_billing_details_from_wallet_connector: false,
      collect_shipping_details_from_wallet_connector: false,
      always_collect_billing_details_from_wallet_connector: false,
      always_collect_shipping_details_from_wallet_connector: false,
    });

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
        authentication_type: 'no_three_ds',
        capture_method: 'automatic'
      },
      'PublishableKey'
    );

    if (!shouldContinueFurther(createData)) {
      test.skip();
      return;
    }

    // Wait for manual retry cutoff to expire
    await new Promise(resolve => setTimeout(resolve, MANUAL_RETRY_EXPIRATION));

    // First Confirm after Manual Retry Cutoff (should succeed)
    const confirmData = getConnectorDetails(connectorId)?.card_pm?.No3DSAutoCapture;
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
  });
});
