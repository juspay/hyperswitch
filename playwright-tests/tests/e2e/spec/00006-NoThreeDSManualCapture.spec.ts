/**
 * Card - NoThreeDS Manual Capture Payment Flow Tests
 *
 * Converted from Cypress: 00006-NoThreeDSManualCapture.cy.js
 * Tests card payments without 3DS with manual capture (both full and partial)
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

  test.describe.serial('Card - NoThreeDS Manual Full Capture payment flow test', () => {

    test.describe('payment Create and Confirm', () => {
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
        expect(Array.isArray(body.payment_methods)).toBe(true);

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

        if (!connectorConfig?.card_pm?.No3DSManualCapture) {
          test.skip();
          return;
        }

        const data = connectorConfig.card_pm.No3DSManualCapture;
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

      test('capture-call-test', async ({ request, globalState }) => {
        const connectorId = globalState.get('connectorId');
        const connectorConfig = getConnectorConfig(connectorId);

        if (!connectorConfig?.card_pm?.Capture) {
          test.skip();
          return;
        }

        const data = connectorConfig.card_pm.Capture;
        const baseUrl = globalState.get('baseUrl');
        const apiKey = globalState.get('apiKey');
        const paymentId = globalState.get('paymentId');

        const requestBody = {
          ...fixtures.captureBody,
          ...data.Request,
        };

        const response = await request.post(`${baseUrl}/payments/${paymentId}/capture`, {
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

        console.log(`✓ Payment captured: ${body.status}`);
      });

      test('retrieve-payment-after-capture-call-test', async ({ request, globalState }) => {
        const connectorId = globalState.get('connectorId');
        const connectorConfig = getConnectorConfig(connectorId);

        if (!connectorConfig?.card_pm?.Capture) {
          test.skip();
          return;
        }

        const data = connectorConfig.card_pm.Capture;
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

        console.log('✓ Payment retrieved after capture');
      });
    });

    test.describe('Payment Create+Confirm', () => {
      let shouldContinue = true;

      test.beforeEach(async ({}, testInfo) => {
        if (!shouldContinue) {
          testInfo.skip();
        }
      });

      test('create+confirm-payment-call-test', async ({ request, globalState }) => {
        const connectorId = globalState.get('connectorId');
        const connectorConfig = getConnectorConfig(connectorId);

        if (!connectorConfig?.card_pm?.No3DSManualCapture) {
          test.skip();
          return;
        }

        const data = connectorConfig.card_pm.No3DSManualCapture;
        const baseUrl = globalState.get('baseUrl');
        const apiKey = globalState.get('apiKey');

        const requestBody = {
          ...fixtures.createConfirmPaymentBody,
          ...data.Request,
          profile_id: globalState.get('profileId'),  // Override placeholder with actual profileId
          customer_id: globalState.get('customerId'),
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

        if (data.Response.body) {
          Object.entries(data.Response.body).forEach(([key, value]) => {
            expect(body[key]).toEqual(value);
          });
        }

        console.log(`✓ Payment created and confirmed: ${body.payment_id}`);
      });

      test('retrieve-payment-after-confirm-call-test', async ({ request, globalState }) => {
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

      test('capture-call-test', async ({ request, globalState }) => {
        const connectorId = globalState.get('connectorId');
        const connectorConfig = getConnectorConfig(connectorId);

        if (!connectorConfig?.card_pm?.Capture) {
          test.skip();
          return;
        }

        const data = connectorConfig.card_pm.Capture;
        const baseUrl = globalState.get('baseUrl');
        const apiKey = globalState.get('apiKey');
        const paymentId = globalState.get('paymentId');

        const requestBody = {
          ...fixtures.captureBody,
          ...data.Request,
        };

        const response = await request.post(`${baseUrl}/payments/${paymentId}/capture`, {
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

        console.log(`✓ Payment captured: ${body.status}`);
      });

      test('retrieve-payment-after-capture-create-confirm-call-test', async ({ request, globalState }) => {
        const connectorId = globalState.get('connectorId');
        const connectorConfig = getConnectorConfig(connectorId);

        if (!connectorConfig?.card_pm?.Capture) {
          test.skip();
          return;
        }

        const data = connectorConfig.card_pm.Capture;
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

        console.log('✓ Payment retrieved after capture');
      });
    });
  });

  test.describe.serial('Card - NoThreeDS Manual Partial Capture payment flow test - Create and Confirm', () => {

    test.describe('payment Create and Payment Confirm', () => {
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

        const response = await request.get(`${baseUrl}/payments/${paymentId}`, {
          headers: {
            'Content-Type': 'application/json',
            'api-key': apiKey,
          },
        });

        const body = await response.json();
        expect(response.status()).toBe(200);

        console.log('✓ Payment retrieved');
      });

      test('capture-call-test', async ({ request, globalState }) => {
        const connectorId = globalState.get('connectorId');
        const connectorConfig = getConnectorConfig(connectorId);

        if (!connectorConfig?.card_pm?.PartialCapture) {
          test.skip();
          return;
        }

        const data = connectorConfig.card_pm.PartialCapture;
        const baseUrl = globalState.get('baseUrl');
        const apiKey = globalState.get('apiKey');
        const paymentId = globalState.get('paymentId');

        const requestBody = {
          ...fixtures.captureBody,
          ...data.Request,
        };

        const response = await request.post(`${baseUrl}/payments/${paymentId}/capture`, {
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

        console.log(`✓ Payment partially captured: ${body.status}`);
      });

      test('retrieve-payment-after-partial-capture-call-test', async ({ request, globalState }) => {
        const connectorId = globalState.get('connectorId');
        const connectorConfig = getConnectorConfig(connectorId);

        if (!connectorConfig?.card_pm?.PartialCapture) {
          test.skip();
          return;
        }

        const data = connectorConfig.card_pm.PartialCapture;
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

        console.log('✓ Payment retrieved after partial capture');
      });
    });

    test.describe('payment + Confirm', () => {
      let shouldContinue = true;

      test.beforeEach(async ({}, testInfo) => {
        if (!shouldContinue) {
          testInfo.skip();
        }
      });

      test('create+confirm-payment-call-test', async ({ request, globalState }) => {
        const connectorId = globalState.get('connectorId');
        const connectorConfig = getConnectorConfig(connectorId);

        if (!connectorConfig?.card_pm?.No3DSManualCapture) {
          test.skip();
          return;
        }

        const data = connectorConfig.card_pm.No3DSManualCapture;
        const baseUrl = globalState.get('baseUrl');
        const apiKey = globalState.get('apiKey');

        const requestBody = {
          ...fixtures.createConfirmPaymentBody,
          ...data.Request,
          profile_id: globalState.get('profileId'),  // Override placeholder with actual profileId
          customer_id: globalState.get('customerId'),
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

        console.log(`✓ Payment created and confirmed: ${body.payment_id}`);
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

        const response = await request.get(`${baseUrl}/payments/${paymentId}`, {
          headers: {
            'Content-Type': 'application/json',
            'api-key': apiKey,
          },
        });

        const body = await response.json();
        expect(response.status()).toBe(200);

        console.log('✓ Payment retrieved');
      });

      test('capture-call-test', async ({ request, globalState }) => {
        const connectorId = globalState.get('connectorId');
        const connectorConfig = getConnectorConfig(connectorId);

        if (!connectorConfig?.card_pm?.PartialCapture) {
          test.skip();
          return;
        }

        const data = connectorConfig.card_pm.PartialCapture;
        const baseUrl = globalState.get('baseUrl');
        const apiKey = globalState.get('apiKey');
        const paymentId = globalState.get('paymentId');

        const requestBody = {
          ...fixtures.captureBody,
          ...data.Request,
        };

        const response = await request.post(`${baseUrl}/payments/${paymentId}/capture`, {
          headers: {
            'Content-Type': 'application/json',
            'api-key': apiKey,
          },
          data: requestBody,
        });

        const body = await response.json();

        expect(response.status()).toBe(data.Response.status);

        console.log(`✓ Payment partially captured: ${body.status}`);
      });

      test('retrieve-payment-after-partial-capture-create-confirm-call-test', async ({ request, globalState }) => {
        const connectorId = globalState.get('connectorId');
        const connectorConfig = getConnectorConfig(connectorId);

        if (!connectorConfig?.card_pm?.PartialCapture) {
          test.skip();
          return;
        }

        const data = connectorConfig.card_pm.PartialCapture;
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

        console.log('✓ Payment retrieved after partial capture');
      });
    });
  });
});
