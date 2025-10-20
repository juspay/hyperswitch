/**
 * Card - ThreeDS Auto Capture Payment Flow Tests
 *
 * Converted from Cypress: 00005-ThreeDSAutoCapture.cy.js
 * Tests card payments with 3DS authentication and automatic capture
 */

import { test, expect } from '../../fixtures/imports';
import * as fixtures from '../../fixtures/test-data';
import { connectorDetails as stripeConfig } from '../configs/Stripe';
import { connectorDetails as cybersourceConfig } from '../configs/Cybersource';
import { handle3DSRedirection } from '../../helpers/RedirectionHelper';

// Get connector config based on connector ID
const getConnectorConfig = (connectorId: string) => {
  const configs: Record<string, typeof stripeConfig> = {
    stripe: stripeConfig,
    cybersource: cybersourceConfig,
  };
  return configs[connectorId.toLowerCase()] || null;
};

test.describe('Card - ThreeDS payment flow test', () => {
  let shouldContinue = true;

  test.beforeEach(async ({}, testInfo) => {
    if (!shouldContinue) {
      testInfo.skip();
    }
  });

  test('create-payment-call-test', async ({ request, globalState }) => {
    const connectorId = globalState.get('connectorId');
    const connectorConfig = getConnectorConfig(connectorId);

    if (!connectorConfig?.card_pm?.PaymentIntent) {
      test.skip();
      return;
    }

    const data = connectorConfig.card_pm.PaymentIntent;
    const baseUrl = globalState.get('baseUrl');
    const apiKey = globalState.get('apiKey');

    const requestBody = {
      ...fixtures.createPaymentBody,
      ...data.Request,
      profile_id: globalState.get('profileId'),  // Override placeholder with actual profileId
      authentication_type: 'three_ds',
      capture_method: 'automatic',
    };

    const response = await request.post(`${baseUrl}/payments`, {
      headers: {
        'Content-Type': 'application/json',
        'api-key': apiKey,
      },
      data: requestBody,
    });

    const body = await response.json();

    expect(response.status()).toBe(data.Response.status);

    globalState.set('paymentId', body.payment_id);
    globalState.set('clientSecret', body.client_secret);

    if (data.Response.body) {
      Object.entries(data.Response.body).forEach(([key, value]) => {
        expect(body[key]).toEqual(value);
      });
    }

    console.log(`✓ Payment created: ${body.payment_id}`);
  });

  test('payment_methods-call-test', async ({ request, globalState }) => {
    const baseUrl = globalState.get('baseUrl');
    const clientSecret = globalState.get('clientSecret');
    const publishableKey = globalState.get('publishableKey');

    const response = await request.get(`${baseUrl}/account/payment_methods?client_secret=${clientSecret}`, {
      headers: {
        'Content-Type': 'application/json',
        'api-key': publishableKey,
      },
    });

    const body = await response.json();
    expect(response.status()).toBe(200);
    expect(body.currency).toBeTruthy();
    expect(Array.isArray(body.payment_methods)).toBe(true);

    console.log('✓ Payment methods retrieved');
  });

  test('Confirm 3DS', async ({ request, globalState }) => {
    const connectorId = globalState.get('connectorId');
    const connectorConfig = getConnectorConfig(connectorId);

    if (!connectorConfig?.card_pm?.['3DSAutoCapture']) {
      test.skip();
      return;
    }

    const data = connectorConfig.card_pm['3DSAutoCapture'];
    const baseUrl = globalState.get('baseUrl');
    const publishableKey = globalState.get('publishableKey');  // Use publishable key like Cypress
    const paymentId = globalState.get('paymentId');
    const clientSecret = globalState.get('clientSecret');

    const requestBody = {
      ...fixtures.confirmBody,
      ...data.Request,
      client_secret: clientSecret,  // Required when using publishable key
    };

    const response = await request.post(`${baseUrl}/payments/${paymentId}/confirm`, {
      headers: {
        'Content-Type': 'application/json',
        'api-key': publishableKey,  // Use publishable key for client-side confirm
      },
      data: requestBody,
    });

    const body = await response.json();

    if (response.status() !== data.Response.status) {
      console.log(`❌ 3DS Confirm Error (${response.status()}):`, JSON.stringify(body, null, 2));
    }

    expect(response.status()).toBe(data.Response.status);

    if (data.Response.body) {
      Object.entries(data.Response.body).forEach(([key, value]) => {
        expect(body[key]).toEqual(value);
      });
    }

    // Store next_action URL for redirection handling
    if (body.next_action) {
      globalState.set('nextActionUrl', body.next_action.redirect_to_url);
    }

    console.log(`✓ Payment confirmed with 3DS: ${body.status}`);
  });

  test('Handle redirection', async ({ page, globalState }) => {
    const connectorId = globalState.get('connectorId');
    const expectedRedirection = fixtures.confirmBody.return_url;
    const nextActionUrl = globalState.get('nextActionUrl');

    // Verify URLs exist
    expect(nextActionUrl).toBeTruthy();
    expect(expectedRedirection).toBeTruthy();

    // Handle 3DS authentication based on connector
    await handle3DSRedirection(page, connectorId, nextActionUrl, expectedRedirection);

    console.log('✓ 3DS authentication completed successfully');
  });
});
