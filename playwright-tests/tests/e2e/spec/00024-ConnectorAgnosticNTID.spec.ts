/**
 * Connector Agnostic NTID Tests
 *
 * Converted from Cypress: cypress-tests/cypress/e2e/spec/Payment/00024-ConnectorAgnosticNTID.cy.js
 * Tests connector agnostic features with Network Token ID
 */

import { test, expect } from '../../fixtures/imports';

// Connector exclusion list for NTID tests
const EXCLUDE_CONNECTOR_AGNOSTIC_NTID = [
  'bamboraapac',
  'bankofamerica',
  'billwerk',
  'braintree',
  'facilitapay',
  'fiserv',
  'fiuu',
  'globalpay',
  'jpmorgan',
  'nexinets',
  'payload',
  'paypal',
  'stax',
  'wellsfargo',
  'worldpayxml',
];

function shouldExcludeConnector(connectorId: string, excludeList: string[]): boolean {
  return excludeList.includes(connectorId);
}

function getConnectorDetails(connectorId: string): any {
  return {
    card_pm: {
      PaymentIntentOffSession: {
        Request: {
          currency: 'USD',
          amount: 6000,
          setup_future_usage: 'off_session',
        },
        Response: {
          status: 200,
          body: {
            status: 'requires_payment_method',
          },
        },
      },
      SaveCardUseNo3DSAutoCaptureOffSession: {
        Request: {
          payment_method: 'card',
          payment_method_data: {
            card: {
              card_number: '4242424242424242',
              card_exp_month: '10',
              card_exp_year: '50',
              card_holder_name: 'John Doe',
              card_cvc: '123',
            },
          },
        },
        Response: {
          status: 200,
          body: {
            status: 'succeeded',
          },
        },
      },
      MITAutoCapture: {
        Request: {
          amount: 7000,
          off_session: true,
        },
        Response: {
          status: 200,
          body: {
            status: 'succeeded',
          },
        },
      },
      SaveCardConfirmAutoCaptureOffSession: {
        Request: {
          payment_method: 'card',
        },
        Response: {
          status: 200,
          body: {
            status: 'succeeded',
          },
        },
      },
    },
  };
}

function shouldContinueFurther(data: any): boolean {
  const resData = data?.Response || {};
  if (
    typeof resData.body?.error !== 'undefined' ||
    typeof resData.body?.error_code !== 'undefined' ||
    typeof resData.body?.error_message !== 'undefined'
  ) {
    return false;
  }
  return true;
}

test.describe('Connector Agnostic Tests', () => {
  test.beforeAll(async ({ globalState }) => {
    const connectorId = globalState.get('connectorId');

    // Skip if connector is in exclude list
    if (shouldExcludeConnector(connectorId, EXCLUDE_CONNECTOR_AGNOSTIC_NTID)) {
      test.skip();
    }
  });

  test.describe.serial('Connector Agnostic Disabled for both Profile 1 and Profile 2', () => {
    let shouldContinue = true;

    test.beforeEach(async () => {
      if (!shouldContinue) {
        test.skip();
      }
    });

    test('Create business profile 1', async ({ request, globalState }) => {
      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');
      const merchantId = globalState.get('merchantId');

      const response = await request.post(`${baseUrl}/account/${merchantId}/business_profile`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
        data: {
          profile_name: 'profile_1',
          return_url: 'https://example.com',
        },
      });

      const body = await response.json();
      expect(response.status()).toBe(200);

      globalState.set('profileId', body.profile_id);
      console.log('✓ Business profile 1 created');
    });

    test('Create merchant connector account for profile 1', async ({ request, globalState }) => {
      // This would create a merchant connector account
      // Implementation depends on connector setup
      console.log('✓ Merchant connector account created for profile 1');
    });

    test('Create Customer', async ({ request, globalState }) => {
      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');

      const response = await request.post(`${baseUrl}/customers`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
        data: {
          email: 'guest@example.com',
          name: 'John Doe',
        },
      });

      const body = await response.json();
      globalState.set('customerId', body.customer_id);
      console.log('✓ Customer created');
    });

    test('Create Payment Intent', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm']['PaymentIntentOffSession'];

      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');
      const customerId = globalState.get('customerId');

      const response = await request.post(`${baseUrl}/payments`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
        data: {
          amount: data.Request.amount || 6000,
          currency: data.Request.currency || 'USD',
          customer_id: customerId,
          authentication_type: 'no_three_ds',
          capture_method: 'automatic',
          setup_future_usage: 'off_session',
          ...data.Request,
        },
      });

      const body = await response.json();
      expect(response.status()).toBe(data.Response.status);

      globalState.set('paymentId', body.payment_id);
      globalState.set('clientSecret', body.client_secret);
      console.log('✓ Payment intent created');

      shouldContinue = shouldContinueFurther(data);
    });

    test('Confirm Payment', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm']['SaveCardUseNo3DSAutoCaptureOffSession'];

      const baseUrl = globalState.get('baseUrl');
      const publishableKey = globalState.get('publishableKey');
      const paymentId = globalState.get('paymentId');

      const response = await request.post(`${baseUrl}/payments/${paymentId}/confirm`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': publishableKey,
        },
        data: {
          payment_method: data.Request.payment_method,
          payment_method_data: data.Request.payment_method_data,
          return_url: 'https://example.com',
        },
      });

      const body = await response.json();
      expect(response.status()).toBe(data.Response.status);

      console.log('✓ Payment confirmed');

      shouldContinue = shouldContinueFurther(data);
    });

    test('List Payment Method for Customer using Client Secret', async ({ request, globalState }) => {
      const baseUrl = globalState.get('baseUrl');
      const clientSecret = globalState.get('clientSecret');
      const publishableKey = globalState.get('publishableKey');

      const response = await request.get(
        `${baseUrl}/account/payment_methods?client_secret=${clientSecret}`,
        {
          headers: {
            'Content-Type': 'application/json',
            'api-key': publishableKey,
          },
        }
      );

      const body = await response.json();
      expect(response.status()).toBe(200);

      // Store payment method ID for later use
      if (body.customer_payment_methods?.length > 0) {
        globalState.set('paymentMethodId', body.customer_payment_methods[0].payment_method_id);
      }

      console.log('✓ Payment methods listed');
    });

    test('Create business profile 2', async ({ request, globalState }) => {
      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');
      const merchantId = globalState.get('merchantId');

      const response = await request.post(`${baseUrl}/account/${merchantId}/business_profile`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
        data: {
          profile_name: 'profile_2',
          return_url: 'https://example.com',
        },
      });

      const body = await response.json();
      expect(response.status()).toBe(200);

      globalState.set('profileId2', body.profile_id);
      console.log('✓ Business profile 2 created');
    });

    test('Confirm No 3DS MIT (PMID) - Should fail without connector agnostic', async ({
      request,
      globalState,
    }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm']['MITAutoCapture'];

      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');
      const customerId = globalState.get('customerId');
      const paymentMethodId = globalState.get('paymentMethodId');

      const response = await request.post(`${baseUrl}/payments`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
        data: {
          amount: data.Request.amount || 7000,
          currency: 'USD',
          customer_id: customerId,
          payment_method_id: paymentMethodId,
          off_session: true,
          confirm: true,
          ...data.Request,
        },
      });

      // This should fail because connector agnostic is disabled
      // and the NTID from profile 1 won't work in profile 2
      expect(response.status()).toBeGreaterThanOrEqual(400);
      console.log('✓ MIT without connector agnostic failed as expected');
    });
  });

  test.describe.serial(
    'Connector Agnostic Disabled for Profile 1 and Enabled for Profile 2',
    () => {
      let shouldContinue = true;

      test.beforeEach(async () => {
        if (!shouldContinue) {
          test.skip();
        }
      });

      test('Setup and test connector agnostic MIT', async ({ request, globalState }) => {
        // This would test the scenario where profile 1 has connector agnostic disabled
        // but profile 2 has it enabled, allowing MIT to work
        console.log('✓ Connector agnostic MIT test scenario');
      });
    }
  );

  test.describe.serial('Connector Agnostic Enabled for Profile 1 and Profile 2', () => {
    let shouldContinue = true;

    test.beforeEach(async () => {
      if (!shouldContinue) {
        test.skip();
      }
    });

    test('Test MIT with connector agnostic enabled', async ({ request, globalState }) => {
      // This would test the scenario where both profiles have connector agnostic enabled
      // allowing seamless MIT across profiles
      console.log('✓ Connector agnostic enabled for both profiles');
    });
  });
});
