/**
 * Card - ThreeDS Manual payment flow test
 *
 * Converted from Cypress test: 00016-ThreeDSManualCapture.cy.js
 * Tests 3DS manual capture flows including full and partial capture scenarios
 */

import { test, expect } from '../../fixtures/imports';
import { getConnectorDetails, shouldContinueFurther } from '../configs/Payment/Utils';
import * as fixtures from '../../fixtures/imports';
import { handle3DSRedirection } from '../../helpers/RedirectionHelper';

test.describe.configure({ mode: 'parallel' });

test.describe.serial('Card - ThreeDS Manual Full Capture payment flow test', () => {
  test.describe('payment Create and Confirm', () => {
    test('create-payment-call-test', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm']['PaymentIntent'];

      if (!shouldContinueFurther(data)) {
        test.skip();
        return;
      }

      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');
      const customerId = globalState.get('customerId');
      const profileId = globalState.get('profileId');

      const createPaymentBody = {
        ...fixtures.createPaymentBody,
        ...data.Request,
        authentication_type: 'three_ds',
        capture_method: 'manual',
        customer_id: customerId,
        profile_id: profileId,
      };

      const response = await request.post(`${baseUrl}/payments`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
        data: createPaymentBody,
      });

      const body = await response.json();

      globalState.set('clientSecret', body.client_secret);
      globalState.set('paymentID', body.payment_id);
      globalState.set('paymentAmount', createPaymentBody.amount);

      console.log(`✓ Payment created: ${body.payment_id}`);
    });

    test('payment_methods-call-test', async ({ request, globalState }) => {
      const baseUrl = globalState.get('baseUrl');
      const publishableKey = globalState.get('publishableKey');
      const clientSecret = globalState.get('clientSecret');

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
      console.log('✓ Payment methods retrieved');
    });

    test('confirm-call-test', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm']['3DSManualCapture'];

      if (!shouldContinueFurther(data)) {
        test.skip();
        return;
      }

      const baseUrl = globalState.get('baseUrl');
      const publishableKey = globalState.get('publishableKey');
      const paymentId = globalState.get('paymentID');

      const confirmBody = {
        ...fixtures.confirmBody,
        ...data.Request,
        client_secret: globalState.get('clientSecret'),
      };

      const response = await request.post(`${baseUrl}/payments/${paymentId}/confirm`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': publishableKey,
        },
        data: confirmBody,
      });

      const body = await response.json();

      if (body.next_action?.redirect_to_url) {
        globalState.set('nextActionUrl', body.next_action.redirect_to_url);
      }

      console.log(`✓ Payment confirmed: ${body.status}`);
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

    test('retrieve-payment-call-test', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm']['3DSManualCapture'];

      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');
      const paymentId = globalState.get('paymentID');

      const response = await request.get(`${baseUrl}/payments/${paymentId}?force_sync=true`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
      });

      const body = await response.json();
      console.log(`✓ Payment retrieved: ${body.status}`);
    });

    test('capture-call-test', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm']['Capture'];

      if (!shouldContinueFurther(data)) {
        test.skip();
        return;
      }

      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');
      const paymentId = globalState.get('paymentID');

      const captureBody = {
        ...fixtures.captureBody,
        amount_to_capture: globalState.get('paymentAmount'),
      };

      const response = await request.post(`${baseUrl}/payments/${paymentId}/capture`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
        data: captureBody,
      });

      const body = await response.json();
      console.log(`✓ Payment captured: ${body.status}`);
    });

    test('retrieve-payment-call-test after capture', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm']['Capture'];

      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');
      const paymentId = globalState.get('paymentID');

      const response = await request.get(`${baseUrl}/payments/${paymentId}?force_sync=true`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
      });

      const body = await response.json();
      console.log(`✓ Payment retrieved after capture: ${body.status}`);
    });
  });

  test.describe('Payment Create+Confirm', () => {
    test('create+confirm-payment-call-test', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm']['3DSManualCapture'];

      if (!shouldContinueFurther(data)) {
        test.skip();
        return;
      }

      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');
      const customerId = globalState.get('customerId');
      const profileId = globalState.get('profileId');

      const createConfirmBody = {
        ...fixtures.createConfirmPaymentBody,
        ...data.Request,
        authentication_type: 'three_ds',
        capture_method: 'manual',
        customer_id: customerId,
        profile_id: profileId,
        confirm: true,
      };

      const response = await request.post(`${baseUrl}/payments`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
        data: createConfirmBody,
      });

      const body = await response.json();

      globalState.set('paymentID', body.payment_id);
      globalState.set('paymentAmount', createConfirmBody.amount);

      if (body.next_action?.redirect_to_url) {
        globalState.set('nextActionUrl', body.next_action.redirect_to_url);
      }

      console.log(`✓ Payment created and confirmed: ${body.payment_id}`);
    });

    test('Handle redirection', async ({ page, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const expectedRedirection = fixtures.createConfirmPaymentBody.return_url;
      const nextActionUrl = globalState.get('nextActionUrl');

      // Verify URLs exist
      expect(nextActionUrl).toBeTruthy();
      expect(expectedRedirection).toBeTruthy();

      // Handle 3DS authentication based on connector
      await handle3DSRedirection(page, connectorId, nextActionUrl, expectedRedirection);

      console.log('✓ 3DS authentication completed successfully');
    });

    test('retrieve-payment-call-test', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm']['3DSManualCapture'];

      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');
      const paymentId = globalState.get('paymentID');

      const response = await request.get(`${baseUrl}/payments/${paymentId}?force_sync=true`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
      });

      const body = await response.json();
      console.log(`✓ Payment retrieved: ${body.status}`);
    });

    test('capture-call-test', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm']['Capture'];

      if (!shouldContinueFurther(data)) {
        test.skip();
        return;
      }

      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');
      const paymentId = globalState.get('paymentID');

      const captureBody = {
        ...fixtures.captureBody,
        amount_to_capture: globalState.get('paymentAmount'),
      };

      const response = await request.post(`${baseUrl}/payments/${paymentId}/capture`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
        data: captureBody,
      });

      const body = await response.json();
      console.log(`✓ Payment captured: ${body.status}`);
    });

    test('retrieve-payment-call-test after capture', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm']['Capture'];

      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');
      const paymentId = globalState.get('paymentID');

      const response = await request.get(`${baseUrl}/payments/${paymentId}?force_sync=true`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
      });

      const body = await response.json();
      console.log(`✓ Payment retrieved after capture: ${body.status}`);
    });
  });
});

test.describe.serial('Card - ThreeDS Manual Partial Capture payment flow test - Create and Confirm', () => {
  test.describe('payment Create and Payment Confirm', () => {
    test('create-payment-call-test', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm']['PaymentIntent'];

      if (!shouldContinueFurther(data)) {
        test.skip();
        return;
      }

      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');
      const customerId = globalState.get('customerId');
      const profileId = globalState.get('profileId');

      const createPaymentBody = {
        ...fixtures.createPaymentBody,
        ...data.Request,
        authentication_type: 'three_ds',
        capture_method: 'manual',
        customer_id: customerId,
        profile_id: profileId,
      };

      const response = await request.post(`${baseUrl}/payments`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
        data: createPaymentBody,
      });

      const body = await response.json();

      globalState.set('clientSecret', body.client_secret);
      globalState.set('paymentID', body.payment_id);
      globalState.set('paymentAmount', createPaymentBody.amount);

      console.log(`✓ Payment created: ${body.payment_id}`);
    });

    test('payment_methods-call-test', async ({ request, globalState }) => {
      const baseUrl = globalState.get('baseUrl');
      const publishableKey = globalState.get('publishableKey');
      const clientSecret = globalState.get('clientSecret');

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
      console.log('✓ Payment methods retrieved');
    });

    test('confirm-call-test', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm']['3DSManualCapture'];

      if (!shouldContinueFurther(data)) {
        test.skip();
        return;
      }

      const baseUrl = globalState.get('baseUrl');
      const publishableKey = globalState.get('publishableKey');
      const paymentId = globalState.get('paymentID');

      const confirmBody = {
        ...fixtures.confirmBody,
        ...data.Request,
        client_secret: globalState.get('clientSecret'),
      };

      const response = await request.post(`${baseUrl}/payments/${paymentId}/confirm`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': publishableKey,
        },
        data: confirmBody,
      });

      const body = await response.json();

      if (body.next_action?.redirect_to_url) {
        globalState.set('nextActionUrl', body.next_action.redirect_to_url);
      }

      console.log(`✓ Payment confirmed: ${body.status}`);
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

    test('retrieve-payment-call-test', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm']['3DSManualCapture'];

      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');
      const paymentId = globalState.get('paymentID');

      const response = await request.get(`${baseUrl}/payments/${paymentId}?force_sync=true`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
      });

      const body = await response.json();
      console.log(`✓ Payment retrieved: ${body.status}`);
    });

    test('capture-call-test (partial)', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm']['PartialCapture'];

      if (!shouldContinueFurther(data)) {
        test.skip();
        return;
      }

      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');
      const paymentId = globalState.get('paymentID');

      const captureBody = {
        ...fixtures.captureBody,
        ...data.Request,
      };

      const response = await request.post(`${baseUrl}/payments/${paymentId}/capture`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
        data: captureBody,
      });

      const body = await response.json();
      console.log(`✓ Payment partially captured: ${body.status}`);
    });

    test('retrieve-payment-call-test after partial capture', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm']['PartialCapture'];

      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');
      const paymentId = globalState.get('paymentID');

      const response = await request.get(`${baseUrl}/payments/${paymentId}?force_sync=true`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
      });

      const body = await response.json();
      console.log(`✓ Payment retrieved after partial capture: ${body.status}`);
    });
  });

  test.describe('payment + Confirm', () => {
    test('create+confirm-payment-call-test', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm']['3DSManualCapture'];

      if (!shouldContinueFurther(data)) {
        test.skip();
        return;
      }

      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');
      const customerId = globalState.get('customerId');
      const profileId = globalState.get('profileId');

      const createConfirmBody = {
        ...fixtures.createConfirmPaymentBody,
        ...data.Request,
        authentication_type: 'three_ds',
        capture_method: 'manual',
        customer_id: customerId,
        profile_id: profileId,
        confirm: true,
      };

      const response = await request.post(`${baseUrl}/payments`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
        data: createConfirmBody,
      });

      const body = await response.json();

      globalState.set('paymentID', body.payment_id);
      globalState.set('paymentAmount', createConfirmBody.amount);

      if (body.next_action?.redirect_to_url) {
        globalState.set('nextActionUrl', body.next_action.redirect_to_url);
      }

      console.log(`✓ Payment created and confirmed: ${body.payment_id}`);
    });

    test('Handle redirection', async ({ page, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const expectedRedirection = fixtures.createConfirmPaymentBody.return_url;
      const nextActionUrl = globalState.get('nextActionUrl');

      // Verify URLs exist
      expect(nextActionUrl).toBeTruthy();
      expect(expectedRedirection).toBeTruthy();

      // Handle 3DS authentication based on connector
      await handle3DSRedirection(page, connectorId, nextActionUrl, expectedRedirection);

      console.log('✓ 3DS authentication completed successfully');
    });

    test('retrieve-payment-call-test', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm']['3DSManualCapture'];

      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');
      const paymentId = globalState.get('paymentID');

      const response = await request.get(`${baseUrl}/payments/${paymentId}?force_sync=true`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
      });

      const body = await response.json();
      console.log(`✓ Payment retrieved: ${body.status}`);
    });

    test('capture-call-test (partial)', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm']['PartialCapture'];

      if (!shouldContinueFurther(data)) {
        test.skip();
        return;
      }

      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');
      const paymentId = globalState.get('paymentID');

      const captureBody = {
        ...fixtures.captureBody,
        ...data.Request,
      };

      const response = await request.post(`${baseUrl}/payments/${paymentId}/capture`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
        data: captureBody,
      });

      const body = await response.json();
      console.log(`✓ Payment partially captured: ${body.status}`);
    });

    test('retrieve-payment-call-test after partial capture', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm']['PartialCapture'];

      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');
      const paymentId = globalState.get('paymentID');

      const response = await request.get(`${baseUrl}/payments/${paymentId}?force_sync=true`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
      });

      const body = await response.json();
      console.log(`✓ Payment retrieved after partial capture: ${body.status}`);
    });
  });
});
