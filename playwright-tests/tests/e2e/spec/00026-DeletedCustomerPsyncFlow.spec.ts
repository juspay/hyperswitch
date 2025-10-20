/**
 * Customer Deletion and Payment Sync Tests
 *
 * Converted from Cypress: cypress-tests/cypress/e2e/spec/Payment/00026-DeletedCustomerPsyncFlow.cy.js
 * Tests for payment sync after customer deletion with automatic and manual capture
 */

import { test, expect } from '../../fixtures/imports';
import { handle3DSRedirection } from '../../helpers/RedirectionHelper';

function getConnectorDetails(connectorId: string): any {
  return {
    card_pm: {
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
      No3DSAutoCapture: {
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
      '3DSAutoCapture': {
        Request: {
          payment_method: 'card',
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
      No3DSManualCapture: {
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
            status: 'requires_capture',
          },
        },
      },
      '3DSManualCapture': {
        Request: {
          payment_method: 'card',
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
      Capture: {
        Request: {
          amount_to_capture: 6000,
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

test.describe('Card - Customer Deletion and Psync', () => {
  test.describe.serial('Card - Psync after Customer Deletion for Automatic Capture', () => {
    test.describe('No3DS Card - Psync after Customer Deletion', () => {
      let shouldContinue = true;

      test.beforeEach(async () => {
        if (!shouldContinue) {
          test.skip();
        }
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
            phone: '999999999',
            phone_country_code: '+65',
          },
        });

        const body = await response.json();
        expect(response.status()).toBe(200);

        globalState.set('customerId', body.customer_id);
        console.log('✓ Customer created');
      });

      test('Create Payment Intent', async ({ request, globalState }) => {
        const connectorId = globalState.get('connectorId');
        const data = getConnectorDetails(connectorId)['card_pm']['PaymentIntent'];

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
            ...data.Request,
          },
        });

        const body = await response.json();
        expect(response.status()).toBe(data.Response.status);

        globalState.set('paymentId', body.payment_id);
        console.log('✓ Payment intent created');

        shouldContinue = shouldContinueFurther(data);
      });

      test('Confirm Payment', async ({ request, globalState }) => {
        const connectorId = globalState.get('connectorId');
        const data = getConnectorDetails(connectorId)['card_pm']['No3DSAutoCapture'];

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

      test('Retrieve Payment', async ({ request, globalState }) => {
        const baseUrl = globalState.get('baseUrl');
        const apiKey = globalState.get('apiKey');
        const paymentId = globalState.get('paymentId');

        const response = await request.get(`${baseUrl}/payments/${paymentId}`, {
          headers: {
            'Content-Type': 'application/json',
            'api-key': apiKey,
          },
        });

        expect(response.status()).toBe(200);
        console.log('✓ Payment retrieved before customer deletion');
      });

      test('Delete Customer', async ({ request, globalState }) => {
        const baseUrl = globalState.get('baseUrl');
        const apiKey = globalState.get('apiKey');
        const customerId = globalState.get('customerId');

        const response = await request.delete(`${baseUrl}/customers/${customerId}`, {
          headers: {
            'Content-Type': 'application/json',
            'api-key': apiKey,
          },
        });

        const body = await response.json();
        expect(response.status()).toBe(200);
        expect(body.deleted).toBe(true);
        expect(body.customer_id).toBe(customerId);

        console.log('✓ Customer deleted');
      });

      test('Retrieve Payment after customer deletion', async ({ request, globalState }) => {
        const baseUrl = globalState.get('baseUrl');
        const apiKey = globalState.get('apiKey');
        const paymentId = globalState.get('paymentId');

        const response = await request.get(`${baseUrl}/payments/${paymentId}`, {
          headers: {
            'Content-Type': 'application/json',
            'api-key': apiKey,
          },
        });

        const body = await response.json();
        expect(response.status()).toBe(200);
        expect(body.payment_id).toBe(paymentId);

        console.log('✓ Payment retrieved successfully after customer deletion');
      });
    });

    test.describe('3DS Card - Psync after Customer Deletion', () => {
      let shouldContinue = true;

      test.beforeEach(async () => {
        if (!shouldContinue) {
          test.skip();
        }
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
        console.log('✓ Customer created for 3DS');
      });

      test('Create Payment Intent', async ({ request, globalState }) => {
        const connectorId = globalState.get('connectorId');
        const data = getConnectorDetails(connectorId)['card_pm']['PaymentIntent'];

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
            authentication_type: 'three_ds',
            capture_method: 'automatic',
            ...data.Request,
          },
        });

        const body = await response.json();
        expect(response.status()).toBe(data.Response.status);

        globalState.set('paymentId', body.payment_id);
        console.log('✓ 3DS Payment intent created');

        shouldContinue = shouldContinueFurther(data);
      });

      test('Confirm Payment', async ({ request, globalState }) => {
        const connectorId = globalState.get('connectorId');
        const data = getConnectorDetails(connectorId)['card_pm']['3DSAutoCapture'];

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

        if (body.next_action?.redirect_to_url) {
          globalState.set('nextActionUrl', body.next_action.redirect_to_url);
        }

        console.log('✓ 3DS Payment confirmed');

        shouldContinue = shouldContinueFurther(data);
      });

      test('Handle redirection', async ({ page, globalState }) => {
        const connectorId = globalState.get('connectorId');
        const expectedRedirection = 'https://example.com';
        const nextActionUrl = globalState.get('nextActionUrl');

        // Verify URLs exist
        expect(nextActionUrl).toBeTruthy();
        expect(expectedRedirection).toBeTruthy();

        // Handle 3DS authentication based on connector
        await handle3DSRedirection(page, connectorId, nextActionUrl, expectedRedirection);

        console.log('✓ 3DS authentication completed successfully');
      });

      test('Retrieve Payment', async ({ request, globalState }) => {
        const baseUrl = globalState.get('baseUrl');
        const apiKey = globalState.get('apiKey');
        const paymentId = globalState.get('paymentId');

        const response = await request.get(`${baseUrl}/payments/${paymentId}`, {
          headers: {
            'Content-Type': 'application/json',
            'api-key': apiKey,
          },
        });

        expect(response.status()).toBe(200);
        console.log('✓ Payment retrieved');
      });

      test('Delete Customer', async ({ request, globalState }) => {
        const baseUrl = globalState.get('baseUrl');
        const apiKey = globalState.get('apiKey');
        const customerId = globalState.get('customerId');

        const response = await request.delete(`${baseUrl}/customers/${customerId}`, {
          headers: {
            'Content-Type': 'application/json',
            'api-key': apiKey,
          },
        });

        const body = await response.json();
        expect(body.deleted).toBe(true);

        console.log('✓ Customer deleted');
      });

      test('Retrieve Payment after deletion', async ({ request, globalState }) => {
        const baseUrl = globalState.get('baseUrl');
        const apiKey = globalState.get('apiKey');
        const paymentId = globalState.get('paymentId');

        const response = await request.get(`${baseUrl}/payments/${paymentId}`, {
          headers: {
            'Content-Type': 'application/json',
            'api-key': apiKey,
          },
        });

        expect(response.status()).toBe(200);
        console.log('✓ Payment retrieved after customer deletion');
      });
    });
  });

  test.describe.serial('Card - Psync after Customer Deletion for Manual Capture', () => {
    test.describe('No3DS Card - Psync after Customer Deletion', () => {
      let shouldContinue = true;

      test.beforeEach(async () => {
        if (!shouldContinue) {
          test.skip();
        }
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
        const data = getConnectorDetails(connectorId)['card_pm']['PaymentIntent'];

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
            capture_method: 'manual',
            ...data.Request,
          },
        });

        const body = await response.json();
        expect(response.status()).toBe(data.Response.status);

        globalState.set('paymentId', body.payment_id);
        console.log('✓ Payment intent created for manual capture');

        shouldContinue = shouldContinueFurther(data);
      });

      test('Confirm Payment', async ({ request, globalState }) => {
        const connectorId = globalState.get('connectorId');
        const data = getConnectorDetails(connectorId)['card_pm']['No3DSManualCapture'];

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

      test('Retrieve Payment', async ({ request, globalState }) => {
        const baseUrl = globalState.get('baseUrl');
        const apiKey = globalState.get('apiKey');
        const paymentId = globalState.get('paymentId');

        const response = await request.get(`${baseUrl}/payments/${paymentId}`, {
          headers: {
            'Content-Type': 'application/json',
            'api-key': apiKey,
          },
        });

        expect(response.status()).toBe(200);
        console.log('✓ Payment retrieved');
      });

      test('Capture Payment', async ({ request, globalState }) => {
        const connectorId = globalState.get('connectorId');
        const data = getConnectorDetails(connectorId)['card_pm']['Capture'];

        const baseUrl = globalState.get('baseUrl');
        const apiKey = globalState.get('apiKey');
        const paymentId = globalState.get('paymentId');

        const response = await request.post(`${baseUrl}/payments/${paymentId}/capture`, {
          headers: {
            'Content-Type': 'application/json',
            'api-key': apiKey,
          },
          data: {
            amount_to_capture: data.Request.amount_to_capture || 6000,
          },
        });

        const body = await response.json();
        expect(response.status()).toBe(data.Response.status);

        console.log('✓ Payment captured');

        shouldContinue = shouldContinueFurther(data);
      });

      test('Retrieve Payment after capture', async ({ request, globalState }) => {
        const baseUrl = globalState.get('baseUrl');
        const apiKey = globalState.get('apiKey');
        const paymentId = globalState.get('paymentId');

        const response = await request.get(`${baseUrl}/payments/${paymentId}`, {
          headers: {
            'Content-Type': 'application/json',
            'api-key': apiKey,
          },
        });

        expect(response.status()).toBe(200);
        console.log('✓ Payment retrieved after capture');
      });

      test('Delete Customer', async ({ request, globalState }) => {
        const baseUrl = globalState.get('baseUrl');
        const apiKey = globalState.get('apiKey');
        const customerId = globalState.get('customerId');

        const response = await request.delete(`${baseUrl}/customers/${customerId}`, {
          headers: {
            'Content-Type': 'application/json',
            'api-key': apiKey,
          },
        });

        const body = await response.json();
        expect(body.deleted).toBe(true);

        console.log('✓ Customer deleted');
      });

      test('Retrieve Payment after customer deletion', async ({ request, globalState }) => {
        const baseUrl = globalState.get('baseUrl');
        const apiKey = globalState.get('apiKey');
        const paymentId = globalState.get('paymentId');

        const response = await request.get(`${baseUrl}/payments/${paymentId}`, {
          headers: {
            'Content-Type': 'application/json',
            'api-key': apiKey,
          },
        });

        expect(response.status()).toBe(200);
        console.log('✓ Payment retrieved after customer deletion');
      });
    });

    test.describe('3DS Card - Psync after Customer Deletion', () => {
      let shouldContinue = true;

      test.beforeEach(async () => {
        if (!shouldContinue) {
          test.skip();
        }
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
        console.log('✓ Customer created for 3DS manual capture');
      });

      test('Create Payment Intent', async ({ request, globalState }) => {
        const connectorId = globalState.get('connectorId');
        const data = getConnectorDetails(connectorId)['card_pm']['PaymentIntent'];

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
            authentication_type: 'three_ds',
            capture_method: 'manual',
            ...data.Request,
          },
        });

        const body = await response.json();
        expect(response.status()).toBe(data.Response.status);

        globalState.set('paymentId', body.payment_id);
        console.log('✓ 3DS Payment intent created for manual capture');

        shouldContinue = shouldContinueFurther(data);
      });

      test('Confirm Payment', async ({ request, globalState }) => {
        const connectorId = globalState.get('connectorId');
        const data = getConnectorDetails(connectorId)['card_pm']['3DSManualCapture'];

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

        if (body.next_action?.redirect_to_url) {
          globalState.set('nextActionUrl', body.next_action.redirect_to_url);
        }

        console.log('✓ 3DS Payment confirmed for manual capture');

        shouldContinue = shouldContinueFurther(data);
      });

      test('Handle redirection', async ({ page, globalState }) => {
        const connectorId = globalState.get('connectorId');
        const expectedRedirection = 'https://example.com';
        const nextActionUrl = globalState.get('nextActionUrl');

        // Verify URLs exist
        expect(nextActionUrl).toBeTruthy();
        expect(expectedRedirection).toBeTruthy();

        // Handle 3DS authentication based on connector
        await handle3DSRedirection(page, connectorId, nextActionUrl, expectedRedirection);

        console.log('✓ 3DS authentication completed successfully');
      });

      test('Retrieve Payment', async ({ request, globalState }) => {
        const baseUrl = globalState.get('baseUrl');
        const apiKey = globalState.get('apiKey');
        const paymentId = globalState.get('paymentId');

        const response = await request.get(`${baseUrl}/payments/${paymentId}`, {
          headers: {
            'Content-Type': 'application/json',
            'api-key': apiKey,
          },
        });

        expect(response.status()).toBe(200);
        console.log('✓ Payment retrieved');
      });

      test('Capture Payment', async ({ request, globalState }) => {
        const connectorId = globalState.get('connectorId');
        const data = getConnectorDetails(connectorId)['card_pm']['Capture'];

        const baseUrl = globalState.get('baseUrl');
        const apiKey = globalState.get('apiKey');
        const paymentId = globalState.get('paymentId');

        const response = await request.post(`${baseUrl}/payments/${paymentId}/capture`, {
          headers: {
            'Content-Type': 'application/json',
            'api-key': apiKey,
          },
          data: {
            amount_to_capture: data.Request.amount_to_capture || 6000,
          },
        });

        const body = await response.json();
        expect(response.status()).toBe(data.Response.status);

        console.log('✓ Payment captured');

        shouldContinue = shouldContinueFurther(data);
      });

      test('Retrieve Payment after capture', async ({ request, globalState }) => {
        const baseUrl = globalState.get('baseUrl');
        const apiKey = globalState.get('apiKey');
        const paymentId = globalState.get('paymentId');

        const response = await request.get(`${baseUrl}/payments/${paymentId}`, {
          headers: {
            'Content-Type': 'application/json',
            'api-key': apiKey,
          },
        });

        expect(response.status()).toBe(200);
        console.log('✓ Payment retrieved after capture');
      });

      test('Delete Customer', async ({ request, globalState }) => {
        const baseUrl = globalState.get('baseUrl');
        const apiKey = globalState.get('apiKey');
        const customerId = globalState.get('customerId');

        const response = await request.delete(`${baseUrl}/customers/${customerId}`, {
          headers: {
            'Content-Type': 'application/json',
            'api-key': apiKey,
          },
        });

        const body = await response.json();
        expect(body.deleted).toBe(true);

        console.log('✓ Customer deleted');
      });

      test('Retrieve Payment after customer deletion', async ({ request, globalState }) => {
        const baseUrl = globalState.get('baseUrl');
        const apiKey = globalState.get('apiKey');
        const paymentId = globalState.get('paymentId');

        const response = await request.get(`${baseUrl}/payments/${paymentId}`, {
          headers: {
            'Content-Type': 'application/json',
            'api-key': apiKey,
          },
        });

        expect(response.status()).toBe(200);
        console.log('✓ Payment retrieved after customer deletion');
      });
    });
  });
});
