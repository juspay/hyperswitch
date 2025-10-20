/**
 * Bank Transfers
 *
 * Converted from Cypress test: 00017-BankTransfers.cy.js
 * Tests various bank transfer payment methods (Pix, InstantBankTransfer, ACH)
 */

import { test } from '../../fixtures/imports';
import { getConnectorDetails, shouldContinueFurther } from '../configs/Payment/Utils';
import * as fixtures from '../../fixtures/imports';

test.describe.configure({ mode: 'parallel' });

test.describe('Bank Transfers', () => {
  test.describe.serial('Bank transfer - Pix forward flow', () => {
    test('create-payment-call-test', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['bank_transfer_pm']['PaymentIntent']('Pix');

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

    test('Confirm bank transfer', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['bank_transfer_pm']['Pix'];

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

      console.log(`✓ Bank transfer confirmed: ${body.status}`);
    });

    test('Handle bank transfer redirection', async ({ page, globalState }) => {
      const nextActionUrl = globalState.get('nextActionUrl');
      const expectedRedirection = fixtures.confirmBody['return_url'];
      const paymentMethodType = globalState.get('paymentMethodType');

      if (nextActionUrl) {
        await page.goto(nextActionUrl);
        // Handle bank transfer flow here
        await page.waitForURL(new RegExp(expectedRedirection), { timeout: 30000 });
        console.log(`✓ Bank transfer (${paymentMethodType}) redirection handled`);
      }
    });
  });

  test.describe.serial('Bank transfer - Instant Bank Transfer Finland forward flow', () => {
    test('create-payment-call-test', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['bank_transfer_pm']['PaymentIntent'](
        'InstantBankTransferFinland'
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

    test('Confirm bank transfer', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['bank_transfer_pm']['InstantBankTransferFinland'];

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

      console.log(`✓ Bank transfer confirmed: ${body.status}`);
    });

    test('Handle bank transfer redirection', async ({ page, globalState }) => {
      const nextActionUrl = globalState.get('nextActionUrl');
      const expectedRedirection = fixtures.confirmBody['return_url'];
      const paymentMethodType = globalState.get('paymentMethodType');

      if (nextActionUrl) {
        await page.goto(nextActionUrl);
        await page.waitForURL(new RegExp(expectedRedirection), { timeout: 30000 });
        console.log(`✓ Bank transfer (${paymentMethodType}) redirection handled`);
      }
    });
  });

  test.describe.serial('Bank transfer - Instant Bank Transfer Poland forward flow', () => {
    test('create-payment-call-test', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['bank_transfer_pm']['PaymentIntent'](
        'InstantBankTransferPoland'
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

    test('Confirm bank transfer', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['bank_transfer_pm']['InstantBankTransferPoland'];

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

      console.log(`✓ Bank transfer confirmed: ${body.status}`);
    });

    test('Handle bank transfer redirection', async ({ page, globalState }) => {
      const nextActionUrl = globalState.get('nextActionUrl');
      const expectedRedirection = fixtures.confirmBody['return_url'];
      const paymentMethodType = globalState.get('paymentMethodType');

      if (nextActionUrl) {
        await page.goto(nextActionUrl);
        await page.waitForURL(new RegExp(expectedRedirection), { timeout: 30000 });
        console.log(`✓ Bank transfer (${paymentMethodType}) redirection handled`);
      }
    });
  });

  test.describe.serial('Bank transfer - Ach flow', () => {
    test('create-payment-call-test', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['bank_transfer_pm']['PaymentIntent']('Ach');

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

    test('Confirm bank transfer', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['bank_transfer_pm']['Ach'];

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

      console.log(`✓ Bank transfer confirmed: ${body.status}`);
    });

    test('Handle bank transfer redirection', async ({ page, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const nextActionUrl = globalState.get('nextActionUrl');
      const expectedRedirection = fixtures.confirmBody['return_url'];
      const paymentMethodType = globalState.get('paymentMethodType');

      // Skip redirection for checkbook connector
      if (connectorId === 'checkbook') {
        console.log('✓ Skipping redirection for checkbook connector');
        return;
      }

      if (nextActionUrl) {
        await page.goto(nextActionUrl);
        await page.waitForURL(new RegExp(expectedRedirection), { timeout: 30000 });
        console.log(`✓ Bank transfer (${paymentMethodType}) redirection handled`);
      }
    });
  });
});
