/**
 * Bank Redirect tests
 *
 * Converted from Cypress test: 00018-BankRedirect.cy.js
 * Tests various bank redirect payment methods (Blik, EPS, iDEAL, Sofort, Przelewy24)
 */

import { test } from '../../fixtures/imports';
import { getConnectorDetails, shouldContinueFurther } from '../configs/Payment/Utils';
import * as fixtures from '../../fixtures/imports';

test.describe.configure({ mode: 'parallel' });

test.describe('Bank Redirect tests', () => {
  test.describe.serial('Blik Create and Confirm flow test', () => {
    test('create-payment-call-test', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['bank_redirect_pm']['PaymentIntent']('Blik');

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
        profile_id: profileId,  // Override placeholder with actual profileId
        authentication_type: 'three_ds',
        capture_method: 'automatic',
        customer_id: customerId,
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

    test('Confirm bank redirect', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['bank_redirect_pm']['Blik'];

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

      if (body.payment_method_type) {
        globalState.set('paymentMethodType', body.payment_method_type);
      }

      console.log(`✓ Bank redirect confirmed: ${body.status}`);
    });
  });

  test.describe.serial('EPS Create and Confirm flow test', () => {
    test('create-payment-call-test', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['bank_redirect_pm']['PaymentIntent']('Eps');

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
        profile_id: profileId,  // Override placeholder with actual profileId
        authentication_type: 'three_ds',
        capture_method: 'automatic',
        customer_id: customerId,
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

    test('Confirm bank redirect', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['bank_redirect_pm']['Eps'];

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

      if (body.payment_method_type) {
        globalState.set('paymentMethodType', body.payment_method_type);
      }

      if (body.next_action?.redirect_to_url) {
        globalState.set('nextActionUrl', body.next_action.redirect_to_url);
      }

      console.log(`✓ Bank redirect confirmed: ${body.status}`);
    });

    test('Handle bank redirect redirection', async ({ page, globalState }) => {
      const nextActionUrl = globalState.get('nextActionUrl');
      const expectedRedirection = fixtures.confirmBody['return_url'];
      const paymentMethodType = globalState.get('paymentMethodType');

      if (nextActionUrl) {
        await page.goto(nextActionUrl);
        await page.waitForURL(new RegExp(expectedRedirection), { timeout: 30000 });
        console.log(`✓ Bank redirect (${paymentMethodType}) redirection handled`);
      }
    });
  });

  test.describe.serial('iDEAL Create and Confirm flow test', () => {
    test('create-payment-call-test', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['bank_redirect_pm']['PaymentIntent']('Ideal');

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
        profile_id: profileId,  // Override placeholder with actual profileId
        authentication_type: 'three_ds',
        capture_method: 'automatic',
        customer_id: customerId,
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

    test('Confirm bank redirect', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['bank_redirect_pm']['Ideal'];

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

      if (body.payment_method_type) {
        globalState.set('paymentMethodType', body.payment_method_type);
      }

      if (body.next_action?.redirect_to_url) {
        globalState.set('nextActionUrl', body.next_action.redirect_to_url);
      }

      console.log(`✓ Bank redirect confirmed: ${body.status}`);
    });

    test('Handle bank redirect redirection', async ({ page, globalState }) => {
      const nextActionUrl = globalState.get('nextActionUrl');
      const expectedRedirection = fixtures.confirmBody['return_url'];
      const paymentMethodType = globalState.get('paymentMethodType');

      if (nextActionUrl) {
        await page.goto(nextActionUrl);
        await page.waitForURL(new RegExp(expectedRedirection), { timeout: 30000 });
        console.log(`✓ Bank redirect (${paymentMethodType}) redirection handled`);
      }
    });
  });

  test.describe.serial('Sofort Create and Confirm flow test', () => {
    test('create-payment-call-test', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['bank_redirect_pm']['PaymentIntent']('Sofort');

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
        profile_id: profileId,  // Override placeholder with actual profileId
        authentication_type: 'three_ds',
        capture_method: 'automatic',
        customer_id: customerId,
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

    test('Confirm bank redirect', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['bank_redirect_pm']['Sofort'];

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

      if (body.payment_method_type) {
        globalState.set('paymentMethodType', body.payment_method_type);
      }

      if (body.next_action?.redirect_to_url) {
        globalState.set('nextActionUrl', body.next_action.redirect_to_url);
      }

      console.log(`✓ Bank redirect confirmed: ${body.status}`);
    });

    test('Handle bank redirect redirection', async ({ page, globalState }) => {
      const nextActionUrl = globalState.get('nextActionUrl');
      const expectedRedirection = fixtures.confirmBody['return_url'];
      const paymentMethodType = globalState.get('paymentMethodType');

      if (nextActionUrl) {
        await page.goto(nextActionUrl);
        await page.waitForURL(new RegExp(expectedRedirection), { timeout: 30000 });
        console.log(`✓ Bank redirect (${paymentMethodType}) redirection handled`);
      }
    });
  });

  test.describe.serial('Przelewy24 Create and Confirm flow test', () => {
    test('create-payment-call-test', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['bank_redirect_pm']['PaymentIntent'](
        'Przelewy24'
      );

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
        profile_id: profileId,  // Override placeholder with actual profileId
        authentication_type: 'three_ds',
        capture_method: 'automatic',
        customer_id: customerId,
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

    test('Confirm bank redirect', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['bank_redirect_pm']['Przelewy24'];

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

      if (body.payment_method_type) {
        globalState.set('paymentMethodType', body.payment_method_type);
      }

      if (body.next_action?.redirect_to_url) {
        globalState.set('nextActionUrl', body.next_action.redirect_to_url);
      }

      console.log(`✓ Bank redirect confirmed: ${body.status}`);
    });

    test('Handle bank redirect redirection', async ({ page, globalState }) => {
      const nextActionUrl = globalState.get('nextActionUrl');
      const expectedRedirection = fixtures.confirmBody['return_url'];
      const paymentMethodType = globalState.get('paymentMethodType');

      if (nextActionUrl) {
        await page.goto(nextActionUrl);
        await page.waitForURL(new RegExp(expectedRedirection), { timeout: 30000 });
        console.log(`✓ Bank redirect (${paymentMethodType}) redirection handled`);
      }
    });
  });
});
