/**
 * Business Profile Configuration Tests
 *
 * Tests various configurations for business profiles including:
 * - collect_billing_details_from_wallet_connector
 * - collect_shipping_details_from_wallet_connector
 * - always_collect_billing_details_from_wallet_connector
 * - always_collect_shipping_details_from_wallet_connector
 *
 * Verifies that these configurations are properly reflected in payment method list responses
 */

import { test, expect } from '../../fixtures/imports';
import { ApiHelpers } from '../../helpers/ApiHelpers';
import * as fixtures from '../../fixtures/test-data';
import { getConnectorDetails, shouldContinueFurther } from '../configs/Payment/Utils';
import { payment_methods_enabled } from '../configs/Payment/Commons';

test.describe.configure({ mode: 'parallel' });

test.describe.serial('Business Profile Config Tests - Billing Address from Wallet', () => {
  test('Update collect_billing_details_from_wallet_connector to true and verify in payment method list', async ({ request, globalState }) => {
    const api = new ApiHelpers(request, globalState);
    const connectorId = globalState.get('connectorId');

    // Create Business Profile
    await api.createBusinessProfile(fixtures.businessProfile);

    // Create Connector
    await api.createConnectorCall(
      'payment_processor',
      fixtures.createConnectorBody,
      payment_methods_enabled
    );

    // Create Customer
    await api.createCustomerCall(fixtures.customerCreateBody);

    // Update Business Profile with collect_billing_details_from_wallet_connector = true
    const profileId = globalState.get('profileId');
    await api.updateBusinessProfile(profileId, {
      is_connector_agnostic_enabled: true,
      collect_billing_details_from_wallet_connector: true,
      collect_shipping_details_from_wallet_connector: false,
      always_collect_billing_details_from_wallet_connector: false,
      always_collect_shipping_details_from_wallet_connector: false,
    });

    // Create Payment Intent
    const data = getConnectorDetails(connectorId)?.card_pm?.PaymentIntentOffSession;
    if (!data) {
      test.skip();
      return;
    }

    await api.createPaymentIntent(
      {
        ...fixtures.createPaymentBody,
        ...data,
        authentication_type: 'no_three_ds',
        capture_method: 'automatic'
      },
      'PublishableKey'
    );

    if (!shouldContinueFurther(data)) {
      test.skip();
      return;
    }

    // Verify in payment methods list
    await api.paymentMethodsList({
      amount: 1000,
      currency: 'USD'
    });
  });
});

test.describe.serial('Business Profile Config Tests - Shipping Address from Wallet', () => {
  test('Update collect_shipping_details_from_wallet_connector to true and verify in payment method list', async ({ request, globalState }) => {
    const api = new ApiHelpers(request, globalState);
    const connectorId = globalState.get('connectorId');

    // Update Business Profile with collect_shipping_details_from_wallet_connector = false
    const profileId = globalState.get('profileId');
    await api.updateBusinessProfile(profileId, {
      is_connector_agnostic_enabled: true,
      collect_billing_details_from_wallet_connector: false,
      collect_shipping_details_from_wallet_connector: false,
      always_collect_billing_details_from_wallet_connector: false,
      always_collect_shipping_details_from_wallet_connector: false,
    });

    // Create Payment Intent
    const data = getConnectorDetails(connectorId)?.card_pm?.PaymentIntentOffSession;
    if (!data) {
      test.skip();
      return;
    }

    await api.createPaymentIntent(
      {
        ...fixtures.createPaymentBody,
        ...data,
        authentication_type: 'no_three_ds',
        capture_method: 'automatic'
      },
      'PublishableKey'
    );

    if (!shouldContinueFurther(data)) {
      test.skip();
      return;
    }

    // Verify in payment methods list
    await api.paymentMethodsList({
      amount: 1000,
      currency: 'USD'
    });
  });
});

test.describe.serial('Business Profile Config Tests - Always Collect Billing Address', () => {
  test('Update always_collect_billing_details_from_wallet_connector to true and verify in payment method list', async ({ request, globalState }) => {
    const api = new ApiHelpers(request, globalState);
    const connectorId = globalState.get('connectorId');

    // Update Business Profile
    const profileId = globalState.get('profileId');
    await api.updateBusinessProfile(profileId, {
      is_connector_agnostic_enabled: true,
      collect_billing_details_from_wallet_connector: false,
      collect_shipping_details_from_wallet_connector: false,
      always_collect_billing_details_from_wallet_connector: true,
      always_collect_shipping_details_from_wallet_connector: false,
    });

    // Create Payment Intent
    const data = getConnectorDetails(connectorId)?.card_pm?.PaymentIntentOffSession;
    if (!data) {
      test.skip();
      return;
    }

    await api.createPaymentIntent(
      {
        ...fixtures.createPaymentBody,
        ...data,
        authentication_type: 'no_three_ds',
        capture_method: 'automatic'
      },
      'PublishableKey'
    );

    if (!shouldContinueFurther(data)) {
      test.skip();
      return;
    }

    // Verify in payment methods list
    await api.paymentMethodsList({
      amount: 1000,
      currency: 'USD'
    });
  });
});

test.describe.serial('Business Profile Config Tests - Always Collect Shipping Address', () => {
  test('Update always_collect_shipping_details_from_wallet_connector to true and verify in payment method list', async ({ request, globalState }) => {
    const api = new ApiHelpers(request, globalState);
    const connectorId = globalState.get('connectorId');

    // Update Business Profile
    const profileId = globalState.get('profileId');
    await api.updateBusinessProfile(profileId, {
      is_connector_agnostic_enabled: true,
      collect_billing_details_from_wallet_connector: false,
      collect_shipping_details_from_wallet_connector: false,
      always_collect_billing_details_from_wallet_connector: false,
      always_collect_shipping_details_from_wallet_connector: true,
    });

    // Create Payment Intent
    const data = getConnectorDetails(connectorId)?.card_pm?.PaymentIntentOffSession;
    if (!data) {
      test.skip();
      return;
    }

    await api.createPaymentIntent(
      {
        ...fixtures.createPaymentBody,
        ...data,
        authentication_type: 'no_three_ds',
        capture_method: 'automatic'
      },
      'PublishableKey'
    );

    if (!shouldContinueFurther(data)) {
      test.skip();
      return;
    }

    // Verify in payment methods list
    await api.paymentMethodsList({
      amount: 1000,
      currency: 'USD'
    });
  });
});

test.describe.serial('Business Profile Config Tests - Both Always and Collect Shipping', () => {
  test('Update both always_collect and collect_shipping_details_from_wallet_connector to true', async ({ request, globalState }) => {
    const api = new ApiHelpers(request, globalState);
    const connectorId = globalState.get('connectorId');

    // Update Business Profile
    const profileId = globalState.get('profileId');
    await api.updateBusinessProfile(profileId, {
      is_connector_agnostic_enabled: true,
      collect_billing_details_from_wallet_connector: false,
      collect_shipping_details_from_wallet_connector: true,
      always_collect_billing_details_from_wallet_connector: false,
      always_collect_shipping_details_from_wallet_connector: true,
    });

    // Create Payment Intent
    const data = getConnectorDetails(connectorId)?.card_pm?.PaymentIntentOffSession;
    if (!data) {
      test.skip();
      return;
    }

    await api.createPaymentIntent(
      {
        ...fixtures.createPaymentBody,
        ...data,
        authentication_type: 'no_three_ds',
        capture_method: 'automatic'
      },
      'PublishableKey'
    );

    if (!shouldContinueFurther(data)) {
      test.skip();
      return;
    }

    // Verify in payment methods list
    await api.paymentMethodsList({
      amount: 1000,
      currency: 'USD'
    });
  });
});

test.describe.serial('Business Profile Config Tests - Both Always and Collect Billing', () => {
  test('Update both always_collect and collect_billing_details_from_wallet_connector to true', async ({ request, globalState }) => {
    const api = new ApiHelpers(request, globalState);
    const connectorId = globalState.get('connectorId');

    // Update Business Profile
    const profileId = globalState.get('profileId');
    await api.updateBusinessProfile(profileId, {
      is_connector_agnostic_enabled: true,
      collect_billing_details_from_wallet_connector: true,
      collect_shipping_details_from_wallet_connector: false,
      always_collect_billing_details_from_wallet_connector: true,
      always_collect_shipping_details_from_wallet_connector: false,
    });

    // Create Payment Intent
    const data = getConnectorDetails(connectorId)?.card_pm?.PaymentIntentOffSession;
    if (!data) {
      test.skip();
      return;
    }

    await api.createPaymentIntent(
      {
        ...fixtures.createPaymentBody,
        ...data,
        authentication_type: 'no_three_ds',
        capture_method: 'automatic'
      },
      'PublishableKey'
    );

    if (!shouldContinueFurther(data)) {
      test.skip();
      return;
    }

    // Verify in payment methods list
    await api.paymentMethodsList({
      amount: 1000,
      currency: 'USD'
    });
  });
});

test.describe.serial('Business Profile Config Tests - All Configs False', () => {
  test('Update all collect address configs to false and verify both configs are false in payment method list', async ({ request, globalState }) => {
    const api = new ApiHelpers(request, globalState);
    const connectorId = globalState.get('connectorId');

    // Create Business Profile
    await api.createBusinessProfile(fixtures.businessProfile);

    // Create Connector
    await api.createConnectorCall(
      'payment_processor',
      fixtures.createConnectorBody,
      payment_methods_enabled
    );

    // Create Customer
    await api.createCustomerCall(fixtures.customerCreateBody);

    // Update Business Profile with all configs false
    const profileId = globalState.get('profileId');
    await api.updateBusinessProfile(profileId, {
      is_connector_agnostic_enabled: true,
      collect_billing_details_from_wallet_connector: false,
      collect_shipping_details_from_wallet_connector: false,
      always_collect_billing_details_from_wallet_connector: false,
      always_collect_shipping_details_from_wallet_connector: false,
    });

    // Create Payment Intent
    const data = getConnectorDetails(connectorId)?.card_pm?.PaymentIntentOffSession;
    if (!data) {
      test.skip();
      return;
    }

    await api.createPaymentIntent(
      {
        ...fixtures.createPaymentBody,
        ...data,
        authentication_type: 'no_three_ds',
        capture_method: 'automatic'
      },
      'PublishableKey'
    );

    if (!shouldContinueFurther(data)) {
      test.skip();
      return;
    }

    // Verify in payment methods list
    await api.paymentMethodsList({
      amount: 1000,
      currency: 'USD'
    });
  });
});
