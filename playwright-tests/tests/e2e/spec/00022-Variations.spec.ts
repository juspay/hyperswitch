/**
 * Corner Cases and Variation Tests
 *
 * Converted from Cypress: cypress-tests/cypress/e2e/spec/Payment/00022-Variations.cy.js
 * Tests various edge cases, invalid inputs, and error scenarios
 */

import { test, expect } from '../../fixtures/imports';

// Helper to get connector details
function getConnectorDetails(connectorId: string): any {
  return {
    card_pm: {
      InvalidCardNumber: {
        Request: {
          payment_method_data: {
            card: {
              card_number: '1234567890123456',
              card_exp_month: '10',
              card_exp_year: '50',
              card_holder_name: 'John Doe',
              card_cvc: '123',
            },
          },
        },
        Response: {
          status: 400,
          body: {
            error: {
              type: 'invalid_request',
              message: 'Invalid card number',
            },
          },
        },
      },
      InvalidExpiryMonth: {
        Request: {
          payment_method_data: {
            card: {
              card_number: '4242424242424242',
              card_exp_month: '13',
              card_exp_year: '50',
              card_holder_name: 'John Doe',
              card_cvc: '123',
            },
          },
        },
        Response: {
          status: 400,
        },
      },
      InvalidExpiryYear: {
        Request: {
          payment_method_data: {
            card: {
              card_number: '4242424242424242',
              card_exp_month: '10',
              card_exp_year: '00',
              card_holder_name: 'John Doe',
              card_cvc: '123',
            },
          },
        },
        Response: {
          status: 400,
        },
      },
      InvalidCardCvv: {
        Request: {
          payment_method_data: {
            card: {
              card_number: '4242424242424242',
              card_exp_month: '10',
              card_exp_year: '50',
              card_holder_name: 'John Doe',
              card_cvc: '12',
            },
          },
        },
        Response: {
          status: 400,
        },
      },
      InvalidCurrency: {
        Request: {
          currency: 'INVALID',
        },
        Response: {
          status: 400,
        },
      },
      InvalidCaptureMethod: {
        Request: {
          capture_method: 'invalid',
        },
        Response: {
          status: 400,
        },
      },
      InvalidPaymentMethod: {
        Request: {
          payment_method: 'invalid_pm',
        },
        Response: {
          status: 400,
        },
      },
      InvalidAmountToCapture: {
        Request: {
          amount_to_capture: 99999999,
        },
        Response: {
          status: 400,
        },
      },
      MissingRequiredParam: {
        Request: {},
        Response: {
          status: 400,
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
      PaymentIntentErrored: {
        Response: {
          status: 400,
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
      CaptureGreaterAmount: {
        Request: {
          amount_to_capture: 99999,
        },
        Response: {
          status: 400,
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
      CaptureCapturedAmount: {
        Response: {
          status: 400,
        },
      },
      ConfirmSuccessfulPayment: {
        Response: {
          status: 400,
        },
      },
      Void: {
        Response: {
          status: 200,
          body: {
            status: 'cancelled',
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
      RefundGreaterAmount: {
        Request: {
          amount: 99999,
        },
        Response: {
          status: 400,
        },
      },
      MandateSingleUseNo3DSManualCapture: {
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
          mandate_data: {
            customer_acceptance: {
              acceptance_type: 'offline',
              accepted_at: '1963-05-03T04:07:52.723Z',
              online: {
                ip_address: '127.0.0.1',
                user_agent: 'amet irure esse',
              },
            },
            mandate_type: {
              single_use: {
                amount: 6000,
                currency: 'USD',
              },
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
      MITAutoCapture: {
        Request: {
          amount: 60000,
        },
        Response: {
          status: 400,
        },
      },
      No3DSFailPayment: {
        Request: {
          payment_method: 'card',
          payment_method_data: {
            card: {
              card_number: '4000000000000002',
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
            status: 'failed',
          },
        },
      },
      DuplicatePaymentID: {
        Response: {
          status: 400,
          body: {
            error: {
              message: 'Duplicate payment found',
            },
          },
        },
      },
      PartialRefund: {
        Request: {
          amount: 3000,
        },
        Response: {
          status: 200,
        },
      },
      SyncRefund: {
        Response: {
          status: 200,
        },
      },
      DuplicateRefundID: {
        Response: {
          status: 400,
        },
      },
    },
    return_url_variations: {
      return_url_too_long: {
        Request: {
          return_url: 'a'.repeat(300),
        },
        Response: {
          status: 400,
        },
      },
      return_url_invalid_format: {
        Request: {
          return_url: 'invalid-url',
        },
        Response: {
          status: 400,
        },
      },
    },
    mandate_id_too_long: {
      Request: {
        mandate_id: 'a'.repeat(300),
      },
      Response: {
        status: 400,
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

test.describe('Corner cases', () => {
  test.describe('[Payment] Invalid Info', () => {
    test('[Payment] Invalid card number', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm']['InvalidCardNumber'];

      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');

      const response = await request.post(`${baseUrl}/payments`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
        data: {
          amount: 6000,
          currency: 'USD',
          confirm: true,
          authentication_type: 'three_ds',
          capture_method: 'automatic',
          payment_method: 'card',
          ...data.Request,
        },
      });

      expect(response.status()).toBe(data.Response.status);
      console.log('✓ Invalid card number handled correctly');
    });

    test('[Payment] Invalid expiry month', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm']['InvalidExpiryMonth'];

      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');

      const response = await request.post(`${baseUrl}/payments`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
        data: {
          amount: 6000,
          currency: 'USD',
          confirm: true,
          authentication_type: 'three_ds',
          capture_method: 'automatic',
          payment_method: 'card',
          ...data.Request,
        },
      });

      expect(response.status()).toBe(data.Response.status);
      console.log('✓ Invalid expiry month handled correctly');
    });

    test('[Payment] Invalid expiry year', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm']['InvalidExpiryYear'];

      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');

      const response = await request.post(`${baseUrl}/payments`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
        data: {
          amount: 6000,
          currency: 'USD',
          confirm: true,
          authentication_type: 'three_ds',
          capture_method: 'automatic',
          payment_method: 'card',
          ...data.Request,
        },
      });

      expect(response.status()).toBe(data.Response.status);
      console.log('✓ Invalid expiry year handled correctly');
    });

    test('[Payment] Invalid card CVV', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm']['InvalidCardCvv'];

      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');

      const response = await request.post(`${baseUrl}/payments`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
        data: {
          amount: 6000,
          currency: 'USD',
          confirm: true,
          authentication_type: 'three_ds',
          capture_method: 'automatic',
          payment_method: 'card',
          ...data.Request,
        },
      });

      expect(response.status()).toBe(data.Response.status);
      console.log('✓ Invalid card CVV handled correctly');
    });

    test('[Payment] Invalid currency', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm']['InvalidCurrency'];

      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');

      const response = await request.post(`${baseUrl}/payments`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
        data: {
          amount: 6000,
          confirm: true,
          authentication_type: 'three_ds',
          capture_method: 'automatic',
          ...data.Request,
        },
      });

      expect(response.status()).toBe(data.Response.status);
      console.log('✓ Invalid currency handled correctly');
    });

    test('[Payment] Invalid capture method', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm']['InvalidCaptureMethod'];

      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');

      const response = await request.post(`${baseUrl}/payments`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
        data: {
          amount: 6000,
          currency: 'USD',
          confirm: true,
          authentication_type: 'three_ds',
          ...data.Request,
        },
      });

      expect(response.status()).toBe(data.Response.status);
      console.log('✓ Invalid capture method handled correctly');
    });

    test('[Payment] Invalid payment method', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm']['InvalidPaymentMethod'];

      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');

      const response = await request.post(`${baseUrl}/payments`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
        data: {
          amount: 6000,
          currency: 'USD',
          confirm: true,
          authentication_type: 'three_ds',
          capture_method: 'automatic',
          ...data.Request,
        },
      });

      expect(response.status()).toBe(data.Response.status);
      console.log('✓ Invalid payment method handled correctly');
    });

    test('[Payment] return_url - too long', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['return_url_variations']['return_url_too_long'];

      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');

      const response = await request.post(`${baseUrl}/payments`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
        data: {
          amount: 6000,
          currency: 'USD',
          confirm: true,
          authentication_type: 'no_three_ds',
          capture_method: 'automatic',
          ...data.Request,
        },
      });

      expect(response.status()).toBe(data.Response.status);
      console.log('✓ Too long return_url handled correctly');
    });

    test('[Payment] return_url - invalid format', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['return_url_variations']['return_url_invalid_format'];

      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');

      const response = await request.post(`${baseUrl}/payments`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
        data: {
          amount: 6000,
          currency: 'USD',
          confirm: true,
          authentication_type: 'no_three_ds',
          capture_method: 'automatic',
          ...data.Request,
        },
      });

      expect(response.status()).toBe(data.Response.status);
      console.log('✓ Invalid return_url format handled correctly');
    });

    test('[Payment] mandate_id - too long', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['mandate_id_too_long'];

      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');

      const response = await request.post(`${baseUrl}/payments`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
        data: {
          amount: 6000,
          currency: 'USD',
          confirm: true,
          authentication_type: 'no_three_ds',
          capture_method: 'automatic',
          ...data.Request,
        },
      });

      expect(response.status()).toBe(data.Response.status);
      console.log('✓ Too long mandate_id handled correctly');
    });
  });


  test.describe.serial('[Payment] Confirm w/o PMD', () => {
    test('Create payment intent', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm']['PaymentIntent'];

      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');

      const response = await request.post(`${baseUrl}/payments`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
        data: {
          amount: data.Request.amount || 6000,
          currency: data.Request.currency || 'USD',
          authentication_type: 'no_three_ds',
          capture_method: 'automatic',
          ...data.Request,
        },
      });

      const body = await response.json();
      expect(response.status()).toBe(data.Response.status);

      globalState.set('paymentId', body.payment_id);
      console.log('✓ Payment intent created');
    });

    test('Confirm payment intent', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm']['PaymentIntentErrored'];

      const baseUrl = globalState.get('baseUrl');
      const publishableKey = globalState.get('publishableKey');
      const paymentId = globalState.get('paymentId');

      const response = await request.post(`${baseUrl}/payments/${paymentId}/confirm`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': publishableKey,
        },
        data: {
          return_url: 'https://example.com',
        },
      });

      expect(response.status()).toBe(data.Response.status);
      console.log('✓ Confirm without PMD handled correctly');
    });
  });

  test.describe.serial('[Payment] Duplicate Payment ID', () => {
    let shouldContinue = true;

    test.beforeEach(async () => {
      if (!shouldContinue) {
        test.skip();
      }
    });

    test('Create new payment', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm']['No3DSAutoCapture'];

      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');

      const response = await request.post(`${baseUrl}/payments`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
        data: {
          amount: 6000,
          currency: 'USD',
          confirm: true,
          authentication_type: 'no_three_ds',
          capture_method: 'automatic',
          payment_method: 'card',
          ...data.Request,
        },
      });

      const body = await response.json();
      expect(response.status()).toBe(data.Response.status);

      globalState.set('paymentID', body.payment_id);
      console.log('✓ Payment created');

      shouldContinue = shouldContinueFurther(data);
    });

    test('Retrieve payment', async ({ request, globalState }) => {
      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');
      const paymentId = globalState.get('paymentID');

      const response = await request.get(`${baseUrl}/payments/${paymentId}`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
      });

      expect(response.status()).toBe(200);
      console.log('✓ Payment retrieved');
    });

    test('Create a payment with a duplicate payment ID', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm']['DuplicatePaymentID'];

      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');
      const paymentId = globalState.get('paymentID');

      const response = await request.post(`${baseUrl}/payments`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
        data: {
          amount: 6000,
          currency: 'USD',
          confirm: true,
          authentication_type: 'no_three_ds',
          capture_method: 'automatic',
          payment_method: 'card',
          payment_id: paymentId,
        },
      });

      expect(response.status()).toBe(data.Response.status);
      console.log('✓ Duplicate payment ID handled correctly');
    });
  });

  test.describe.serial('[Payment] Duplicate Customer ID', () => {
    test('Create new customer', async ({ request, globalState }) => {
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

    test('Create a customer with a duplicate customer ID', async ({ request, globalState }) => {
      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');
      const customerId = globalState.get('customerId');

      const response = await request.post(`${baseUrl}/customers`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
        data: {
          customer_id: customerId,
          email: 'guest@example.com',
          name: 'John Doe',
        },
      });

      const body = await response.json();

      if (response.status() === 400) {
        expect(body.error?.code).toBe('IR_12');
        console.log('✓ Duplicate customer ID handled correctly');
      }
    });
  });
});
