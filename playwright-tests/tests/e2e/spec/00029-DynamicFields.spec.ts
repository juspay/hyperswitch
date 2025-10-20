/**
 * Dynamic Fields Verification Tests
 *
 * Tests verification of dynamic fields for card payments based on
 * the presence or absence of billing information:
 * - Payment without billing address
 * - Payment with billing address
 * - Payment with billing first and last name
 * - Payment with billing email
 *
 * Verifies that payment method list returns appropriate required fields
 */

import { test, expect } from '../../fixtures/imports';
import { ApiHelpers } from '../../helpers/ApiHelpers';
import * as fixtures from '../../fixtures/test-data';
import { getConnectorDetails, shouldContinueFurther } from '../configs/Payment/Utils';
import { cardCreditEnabled } from '../configs/PaymentMethodList/Commons';

test.describe.configure({ mode: 'parallel' });

test.describe.serial('Dynamic Fields - Payment without Billing Address', () => {
  test('Verify dynamic fields when payment is created without billing address', async ({ request, globalState }) => {
    const api = new ApiHelpers(request, globalState);
    const connectorId = globalState.get('connectorId');

    // Create Business Profile
    await api.createBusinessProfile(fixtures.businessProfile);

    // Create Connector
    await api.createConnectorCall(
      'payment_processor',
      fixtures.createConnectorBody,
      cardCreditEnabled
    );

    // Create Payment Intent without billing address
    const data = getConnectorDetails(connectorId)?.card_pm?.PaymentWithoutBilling;
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

    // Verify dynamic fields in payment method list
    const pmListData = getConnectorDetails(connectorId)?.pm_list?.PmListResponse?.pmListDynamicFieldWithoutBilling;
    if (!pmListData) {
      test.skip();
      return;
    }

    await api.paymentMethodsListWithRequiredFields(pmListData);
  });
});

test.describe.serial('Dynamic Fields - Payment with Billing Address', () => {
  test('Verify dynamic fields when payment is created with billing address', async ({ request, globalState }) => {
    const api = new ApiHelpers(request, globalState);
    const connectorId = globalState.get('connectorId');

    // Create Payment Intent with billing address
    const data = getConnectorDetails(connectorId)?.card_pm?.PaymentWithBilling;
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

    // Verify dynamic fields in payment method list
    const pmListData = getConnectorDetails(connectorId)?.pm_list?.PmListResponse?.pmListDynamicFieldWithBilling;
    if (!pmListData) {
      test.skip();
      return;
    }

    await api.paymentMethodsListWithRequiredFields(pmListData);
  });
});

test.describe.serial('Dynamic Fields - Payment with Billing First and Last Name', () => {
  test('Verify dynamic fields when payment is created with billing first and last name', async ({ request, globalState }) => {
    const api = new ApiHelpers(request, globalState);
    const connectorId = globalState.get('connectorId');

    // Create Payment Intent with full name
    const data = getConnectorDetails(connectorId)?.card_pm?.PaymentWithFullName;
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

    // Verify dynamic fields in payment method list
    const pmListData = getConnectorDetails(connectorId)?.pm_list?.PmListResponse?.pmListDynamicFieldWithNames;
    if (!pmListData) {
      test.skip();
      return;
    }

    await api.paymentMethodsListWithRequiredFields(pmListData);
  });
});

test.describe.serial('Dynamic Fields - Payment with Billing Email', () => {
  test('Verify dynamic fields when payment is created with billing email', async ({ request, globalState }) => {
    const api = new ApiHelpers(request, globalState);
    const connectorId = globalState.get('connectorId');

    // Create Payment Intent with billing email
    const data = getConnectorDetails(connectorId)?.card_pm?.PaymentWithBillingEmail;
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

    // Verify dynamic fields in payment method list
    const pmListData = getConnectorDetails(connectorId)?.pm_list?.PmListResponse?.pmListDynamicFieldWithEmail;
    if (!pmListData) {
      test.skip();
      return;
    }

    await api.paymentMethodsListWithRequiredFields(pmListData);
  });
});
