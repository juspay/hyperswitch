/**
 * Session Token Tests
 *
 * Converted from Cypress: cypress-tests/cypress/e2e/spec/Payment/00025-SessionCall.cy.js
 * Tests for creating session tokens
 */

import { test, expect } from '../../fixtures/imports';

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
      SessionToken: {
        Request: {
          wallets: ['apple_pay', 'google_pay'],
        },
        Response: {
          status: 200,
          body: {
            session_token: [],
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

test.describe.serial('Customer Create flow test', () => {
  let shouldContinue = true;

  test.beforeEach(async () => {
    if (!shouldContinue) {
      test.skip();
    }
  });

  test('create-payment-call-test', async ({ request, globalState }) => {
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
    expect(body.status).toBe(data.Response.body.status);

    globalState.set('paymentId', body.payment_id);
    globalState.set('clientSecret', body.client_secret);

    console.log('✓ Payment created for session call');

    shouldContinue = shouldContinueFurther(data);
  });

  test('session-call-test', async ({ request, globalState }) => {
    const connectorId = globalState.get('connectorId');
    const data = getConnectorDetails(connectorId)['card_pm']['SessionToken'];

    const baseUrl = globalState.get('baseUrl');
    const paymentId = globalState.get('paymentId');

    const response = await request.get(`${baseUrl}/payments/${paymentId}/session_tokens`, {
      headers: {
        'Content-Type': 'application/json',
      },
    });

    const body = await response.json();
    expect(response.status()).toBe(data.Response.status);

    // Validate session token structure
    if (body.session_token) {
      expect(Array.isArray(body.session_token)).toBe(true);
      console.log(`✓ Session tokens retrieved: ${body.session_token.length} tokens`);
    } else {
      console.log('✓ Session tokens endpoint called successfully');
    }
  });
});
