/**
 * Card - Sync Refund Flow Tests
 *
 * Converted from Cypress: 00010-SyncRefund.cy.js
 * Tests refund retrieval and synchronization
 */

import { test, expect } from '../../fixtures/imports';
import * as fixtures from '../../fixtures/test-data';
import { connectorDetails as stripeConfig } from '../configs/Stripe';
import { connectorDetails as cybersourceConfig } from '../configs/Cybersource';

// Get connector config based on connector ID
const getConnectorConfig = (connectorId: string) => {
  const configs: Record<string, typeof stripeConfig> = {
    stripe: stripeConfig,
    cybersource: cybersourceConfig,
  };
  return configs[connectorId.toLowerCase()] || null;
};

test.describe.serial('Card - Sync Refund flow test', () => {
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
      customer_id: globalState.get('customerId'),
      authentication_type: 'no_three_ds',
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

  test('confirm-call-test', async ({ request, globalState }) => {
    const connectorId = globalState.get('connectorId');
    const connectorConfig = getConnectorConfig(connectorId);

    if (!connectorConfig?.card_pm?.No3DSAutoCapture) {
      test.skip();
      return;
    }

    const data = connectorConfig.card_pm.No3DSAutoCapture;
    const baseUrl = globalState.get('baseUrl');
    const publishableKey = globalState.get('publishableKey');
    const paymentId = globalState.get('paymentId');
    const clientSecret = globalState.get('clientSecret');

    const requestBody = {
      ...fixtures.confirmBody,
      ...data.Request,
      client_secret: clientSecret,
    };

    const response = await request.post(`${baseUrl}/payments/${paymentId}/confirm`, {
      headers: {
        'Content-Type': 'application/json',
        'api-key': publishableKey,
      },
      data: requestBody,
    });

    const body = await response.json();

    expect(response.status()).toBe(data.Response.status);

    if (data.Response.body) {
      Object.entries(data.Response.body).forEach(([key, value]) => {
        expect(body[key]).toEqual(value);
      });
    }

    console.log(`✓ Payment confirmed: ${body.status}`);
  });

  test('retrieve-payment-call-test', async ({ request, globalState }) => {
    const connectorId = globalState.get('connectorId');
    const connectorConfig = getConnectorConfig(connectorId);

    if (!connectorConfig?.card_pm?.No3DSAutoCapture) {
      test.skip();
      return;
    }

    const data = connectorConfig.card_pm.No3DSAutoCapture;
    const baseUrl = globalState.get('baseUrl');
    const apiKey = globalState.get('apiKey');
    const paymentId = globalState.get('paymentId');
    const clientSecret = globalState.get('clientSecret');

    const response = await request.get(`${baseUrl}/payments/${paymentId}`, {
      headers: {
        'Content-Type': 'application/json',
        'api-key': apiKey,
      },
    });

    const body = await response.json();
    expect(response.status()).toBe(200);

    if (data.Response.body) {
      Object.entries(data.Response.body).forEach(([key, value]) => {
        expect(body[key]).toEqual(value);
      });
    }

    console.log('✓ Payment retrieved');
  });

  test('refund-call-test', async ({ request, globalState }) => {
    const connectorId = globalState.get('connectorId');
    const connectorConfig = getConnectorConfig(connectorId);

    if (!connectorConfig?.card_pm?.Refund) {
      test.skip();
      return;
    }

    const data = connectorConfig.card_pm.Refund;
    const baseUrl = globalState.get('baseUrl');
    const apiKey = globalState.get('apiKey');
    const paymentId = globalState.get('paymentId');

    const requestBody = {
      ...fixtures.refundBody,
      ...data.Request,
      payment_id: paymentId,
    };

    const response = await request.post(`${baseUrl}/refunds`, {
      headers: {
        'Content-Type': 'application/json',
        'api-key': apiKey,
      },
      data: requestBody,
    });

    const body = await response.json();

    expect(response.status()).toBe(data.Response.status);

    globalState.set('refundId', body.refund_id);

    if (data.Response.body) {
      Object.entries(data.Response.body).forEach(([key, value]) => {
        expect(body[key]).toEqual(value);
      });
    }

    console.log(`✓ Refund created: ${body.refund_id}`);
  });

  test('sync-refund-call-test', async ({ request, globalState }) => {
    const connectorId = globalState.get('connectorId');
    const connectorConfig = getConnectorConfig(connectorId);

    if (!connectorConfig?.card_pm?.SyncRefund) {
      test.skip();
      return;
    }

    const data = connectorConfig.card_pm.SyncRefund;
    const baseUrl = globalState.get('baseUrl');
    const apiKey = globalState.get('apiKey');
    const refundId = globalState.get('refundId');

    const response = await request.get(`${baseUrl}/refunds/${refundId}`, {
      headers: {
        'Content-Type': 'application/json',
        'api-key': apiKey,
      },
    });

    const body = await response.json();

    expect(response.status()).toBe(data.Response.status);

    if (data.Response.body) {
      Object.entries(data.Response.body).forEach(([key, value]) => {
        expect(body[key]).toEqual(value);
      });
    }

    console.log('✓ Refund retrieved and synced');
  });
});
