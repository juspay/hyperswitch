/**
 * Payment Methods Tests
 *
 * Converted from Cypress: cypress-tests/cypress/e2e/spec/Payment/00023-PaymentMethods.cy.js
 * Tests for creating, listing, setting default, and deleting payment methods
 */

import { test, expect } from '../../fixtures/imports';

// Helper to get connector details
function getConnectorDetails(connectorId: string): any {
  return {
    card_pm: {
      PaymentMethod: {
        Request: {
          payment_method: 'card',
          payment_method_type: 'debit',
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
        },
      },
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
          payment_method_type: 'debit',
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
      SaveCardUse3DSAutoCaptureOffSession: {
        Request: {
          payment_method: 'card',
          payment_method_type: 'debit',
          payment_method_data: {
            card: {
              card_number: '4000002500003155',
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
            status: 'requires_customer_action',
          },
        },
      },
      PaymentIntent: {
        Request: {
          currency: 'USD',
          amount: 6000,
        },
        Response: {
          status: 200,
          body: {
            status: 'requires_payment_method',
          },
        },
      },
      SaveCardUseNo3DSAutoCapture: {
        Request: {
          payment_method: 'card',
          payment_method_type: 'debit',
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

test.describe('Payment Methods Tests', () => {
  test.describe.serial('Create payment method for customer', () => {

    test('Create customer', async ({ request, globalState }) => {
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
          phone: '999999999',
          phone_country_code: '+65',
        },
      });

      const body = await response.json();
      expect(response.status()).toBe(200);
      expect(body.customer_id).toBeTruthy();

      globalState.set('customerId', body.customer_id);
      console.log(`✓ Customer created: ${body.customer_id}`);
    });

    test('Create Payment Method', async ({ request, globalState }) => {
      const data = getConnectorDetails('commons')['card_pm']['PaymentMethod'];

      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');
      const customerId = globalState.get('customerId');

      const response = await request.post(`${baseUrl}/payment_methods`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
        data: {
          customer_id: customerId,
          ...data.Request,
        },
      });

      const body = await response.json();
      expect(response.status()).toBe(data.Response.status);
      expect(body.payment_method_id).toBeTruthy();

      globalState.set('paymentMethodId', body.payment_method_id);
      console.log('✓ Payment method created');
    });

    test('List PM for customer', async ({ request, globalState }) => {
      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');
      const customerId = globalState.get('customerId');

      const response = await request.get(`${baseUrl}/customers/${customerId}/payment_methods`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
      });

      const body = await response.json();
      expect(response.status()).toBe(200);
      expect(body.customer_payment_methods).toBeDefined();
      expect(Array.isArray(body.customer_payment_methods)).toBe(true);

      console.log('✓ Payment methods listed');
    });
  });

  test.describe.serial('Set default payment method', () => {
    let shouldContinue = true;

    test.beforeEach(async () => {
      if (!shouldContinue) {
        test.skip();
      }
    });

    test('List PM for customer before payment', async ({ request, globalState }) => {
      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');
      const customerId = globalState.get('customerId');

      const response = await request.get(`${baseUrl}/customers/${customerId}/payment_methods`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
      });

      expect(response.status()).toBe(200);
      console.log('✓ Payment methods listed');
    });

    test('Create Payment Method', async ({ request, globalState }) => {
      const data = getConnectorDetails('commons')['card_pm']['PaymentMethod'];

      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');
      const customerId = globalState.get('customerId');

      const response = await request.post(`${baseUrl}/payment_methods`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
        data: {
          customer_id: customerId,
          ...data.Request,
        },
      });

      const body = await response.json();
      expect(response.status()).toBe(data.Response.status);

      globalState.set('paymentMethodId', body.payment_method_id);
      console.log('✓ Payment method created');
    });

    test('create-payment-call-test', async ({ request, globalState }) => {
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
      console.log('✓ Payment intent created');

      shouldContinue = shouldContinueFurther(data);
    });

    test('confirm-payment-call-test', async ({ request, globalState }) => {
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
          payment_method_type: data.Request.payment_method_type,
          payment_method_data: data.Request.payment_method_data,
          return_url: 'https://example.com',
        },
      });

      const body = await response.json();
      expect(response.status()).toBe(data.Response.status);

      console.log('✓ Payment confirmed');

      shouldContinue = shouldContinueFurther(data);
    });

    test('List PM for customer after payment', async ({ request, globalState }) => {
      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');
      const customerId = globalState.get('customerId');

      const response = await request.get(`${baseUrl}/customers/${customerId}/payment_methods`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
      });

      const body = await response.json();
      expect(response.status()).toBe(200);

      // Store the first payment method ID for setting as default
      if (body.customer_payment_methods?.length > 0) {
        globalState.set('paymentMethodIdToSetDefault', body.customer_payment_methods[0].payment_method_id);
      }

      console.log('✓ Payment methods listed');
    });

    test('Set default payment method', async ({ request, globalState }) => {
      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');
      const customerId = globalState.get('customerId');
      const paymentMethodId = globalState.get('paymentMethodIdToSetDefault');

      const response = await request.post(
        `${baseUrl}/customers/${customerId}/payment_methods/${paymentMethodId}/default`,
        {
          headers: {
            'Content-Type': 'application/json',
            'api-key': apiKey,
          },
        }
      );

      expect(response.status()).toBe(200);
      console.log('✓ Default payment method set');
    });
  });

  test.describe.serial('Delete payment method for customer', () => {

    test('Create customer', async ({ request, globalState }) => {
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
      expect(response.status()).toBe(200);

      globalState.set('customerId', body.customer_id);
      console.log('✓ Customer created');
    });

    test('Create Payment Method', async ({ request, globalState }) => {
      const data = getConnectorDetails('commons')['card_pm']['PaymentMethod'];

      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');
      const customerId = globalState.get('customerId');

      const response = await request.post(`${baseUrl}/payment_methods`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
        data: {
          customer_id: customerId,
          ...data.Request,
        },
      });

      const body = await response.json();
      expect(response.status()).toBe(data.Response.status);

      globalState.set('paymentMethodId', body.payment_method_id);
      console.log('✓ Payment method created');
    });

    test('List PM for customer', async ({ request, globalState }) => {
      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');
      const customerId = globalState.get('customerId');

      const response = await request.get(`${baseUrl}/customers/${customerId}/payment_methods`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
      });

      expect(response.status()).toBe(200);
      console.log('✓ Payment methods listed');
    });

    test('Delete Payment Method for a customer', async ({ request, globalState }) => {
      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');
      const paymentMethodId = globalState.get('paymentMethodId');

      const response = await request.delete(`${baseUrl}/payment_methods/${paymentMethodId}`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
      });

      const body = await response.json();
      expect(response.status()).toBe(200);
      expect(body.deleted).toBe(true);

      console.log('✓ Payment method deleted');
    });
  });

  test.describe.serial("'Last Used' off-session token payments", () => {
    let shouldContinue = true;

    test.beforeEach(async () => {
      if (!shouldContinue) {
        test.skip();
      }
    });

    test('Create customer', async ({ request, globalState }) => {
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

    test('Create No 3DS off session save card payment', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm']['SaveCardUseNo3DSAutoCaptureOffSession'];

      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');
      const customerId = globalState.get('customerId');

      const response = await request.post(`${baseUrl}/payments`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
        data: {
          amount: 6000,
          currency: 'USD',
          customer_id: customerId,
          confirm: true,
          authentication_type: 'no_three_ds',
          capture_method: 'automatic',
          setup_future_usage: 'off_session',
          ...data.Request,
        },
      });

      const body = await response.json();
      expect(response.status()).toBe(data.Response.status);

      console.log('✓ No 3DS off session payment created');

      shouldContinue = shouldContinueFurther(data);
    });

    test('List PM for customer', async ({ request, globalState }) => {
      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');
      const customerId = globalState.get('customerId');

      const response = await request.get(`${baseUrl}/customers/${customerId}/payment_methods`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
      });

      expect(response.status()).toBe(200);
      console.log('✓ Payment methods listed');
    });
  });
});
