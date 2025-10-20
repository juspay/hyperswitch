/**
 * Card - Void Payment Flow Tests
 *
 * Converted from Cypress: 00007-VoidPayment.cy.js
 * Tests payment void/cancellation in different payment states
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

test.describe('Card - NoThreeDS Manual payment flow test', () => {

  test.describe.serial('Card - void payment in Requires_capture state flow test', () => {
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
        capture_method: 'manual',
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

      if (!connectorConfig?.card_pm?.No3DSManualCapture) {
        test.skip();
        return;
      }

      const data = connectorConfig.card_pm.No3DSManualCapture;
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

      if (!connectorConfig?.card_pm?.No3DSManualCapture) {
        test.skip();
        return;
      }

      const data = connectorConfig.card_pm.No3DSManualCapture;
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

    test('void-call-test', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const connectorConfig = getConnectorConfig(connectorId);

      if (!connectorConfig?.card_pm?.VoidAfterConfirm) {
        test.skip();
        return;
      }

      const data = connectorConfig.card_pm.VoidAfterConfirm;
      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');
      const paymentId = globalState.get('paymentId');

      const requestBody = {
        ...fixtures.voidBody,
        ...data.Request,
      };

      const response = await request.post(`${baseUrl}/payments/${paymentId}/cancel`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
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

      console.log(`✓ Payment voided: ${body.status}`);
    });
  });

  test.describe.serial('Card - void payment in Requires_payment_method state flow test', () => {
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
        capture_method: 'manual',
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

    test('void-call-test', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const connectorConfig = getConnectorConfig(connectorId);

      if (!connectorConfig?.card_pm?.Void) {
        test.skip();
        return;
      }

      const data = connectorConfig.card_pm.Void;
      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');
      const paymentId = globalState.get('paymentId');

      const requestBody = {
        ...fixtures.voidBody,
        ...data.Request,
      };

      const response = await request.post(`${baseUrl}/payments/${paymentId}/cancel`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
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

      console.log(`✓ Payment voided: ${body.status}`);
    });
  });

  test.describe.serial('Card - void payment in success state flow test', () => {
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
        capture_method: 'manual',
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

      if (!connectorConfig?.card_pm?.No3DSManualCapture) {
        test.skip();
        return;
      }

      const data = connectorConfig.card_pm.No3DSManualCapture;
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

      if (!connectorConfig?.card_pm?.No3DSManualCapture) {
        test.skip();
        return;
      }

      const data = connectorConfig.card_pm.No3DSManualCapture;
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

    test('void-call-test', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const connectorConfig = getConnectorConfig(connectorId);

      if (!connectorConfig?.card_pm?.VoidAfterConfirm) {
        test.skip();
        return;
      }

      const data = connectorConfig.card_pm.VoidAfterConfirm;
      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');
      const paymentId = globalState.get('paymentId');

      const requestBody = {
        ...fixtures.voidBody,
        ...data.Request,
      };

      const response = await request.post(`${baseUrl}/payments/${paymentId}/cancel`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
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

      console.log(`✓ Payment voided: ${body.status}`);
    });
  });
});
