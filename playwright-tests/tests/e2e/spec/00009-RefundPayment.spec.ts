/**
 * Card - Refund Payment Flow Tests
 *
 * Converted from Cypress: 00009-RefundPayment.cy.js
 * Tests payment refunds (full and partial) for both No3DS and 3DS payments
 * Note: This is a large test file - only implementing key scenarios for POC
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

test.describe('Card - Refund flow - No 3DS', () => {

  test.describe.serial('Card - Full Refund flow test for No-3DS', () => {
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

      const response = await request.get(`${baseUrl}/payments/${paymentId}?client_secret=${clientSecret}`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
      });

      const body = await response.json();
      expect(response.status()).toBe(200);

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

      console.log('✓ Refund synced');
    });
  });

  test.describe.serial('Card - Partial Refund flow test for No-3DS', () => {
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

      const response = await request.get(`${baseUrl}/payments/${paymentId}?client_secret=${clientSecret}`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
      });

      const body = await response.json();
      expect(response.status()).toBe(200);

      console.log('✓ Payment retrieved');
    });

    test('refund-call-test - first partial', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const connectorConfig = getConnectorConfig(connectorId);

      if (!connectorConfig?.card_pm?.PartialRefund) {
        test.skip();
        return;
      }

      const data = connectorConfig.card_pm.PartialRefund;
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

      console.log(`✓ Partial refund 1 created: ${body.refund_id}`);
    });

    test('refund-call-test - second partial', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const connectorConfig = getConnectorConfig(connectorId);

      if (!connectorConfig?.card_pm?.PartialRefund) {
        test.skip();
        return;
      }

      const data = connectorConfig.card_pm.PartialRefund;
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

      console.log(`✓ Partial refund 2 created: ${body.refund_id}`);
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

      console.log('✓ Refund synced');
    });
  });

  test.describe.serial('Fully Refund Card-NoThreeDS payment flow test Create+Confirm', () => {
    let shouldContinue = true;

    test.beforeEach(async ({}, testInfo) => {
      if (!shouldContinue) {
        testInfo.skip();
      }
    });

    test('create+confirm-payment-call-test', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const connectorConfig = getConnectorConfig(connectorId);

      if (!connectorConfig?.card_pm?.No3DSAutoCapture) {
        test.skip();
        return;
      }

      const data = connectorConfig.card_pm.No3DSAutoCapture;
      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');

      const requestBody = {
        ...fixtures.createConfirmPaymentBody,
        ...data.Request,
        profile_id: globalState.get('profileId'),  // Override placeholder with actual profileId
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

      console.log(`✓ Payment created and confirmed: ${body.payment_id}`);
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

      const response = await request.get(`${baseUrl}/payments/${paymentId}?client_secret=${clientSecret}`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
      });

      const body = await response.json();
      expect(response.status()).toBe(200);

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

      console.log('✓ Refund synced');
    });
  });
});

// TODO: Implement 3DS refund scenarios
// The Cypress test file includes extensive 3DS refund scenarios which follow
// similar patterns to the No3DS tests above. These can be added by duplicating
// the above test suites and:
// 1. Using '3DSAutoCapture' instead of 'No3DSAutoCapture'
// 2. Setting authentication_type to 'three_ds'
// 3. Adding redirection handling after confirm

test.describe('Card - Refund flow - 3DS', () => {
  test('TODO: Implement 3DS refund scenarios', async () => {
    // Placeholder for 3DS refund tests
    // Follow the same pattern as No3DS tests above
    test.skip();
  });
});
