/**
 * Card - Mandates using Payment Method Id flow test
 *
 * Converted from Cypress test: 00019-MandatesUsingPMID.cy.js
 * Tests mandate flows using Payment Method ID (CIT and MIT)
 */

import { test, expect } from '../../fixtures/imports';
import { getConnectorDetails, shouldContinueFurther } from '../configs/Payment/Utils';
import * as fixtures from '../../fixtures/imports';
import { handle3DSRedirection } from '../../helpers/RedirectionHelper';

test.describe.configure({ mode: 'parallel' });

test.describe('Card - Mandates using Payment Method Id flow test', () => {
  test.describe.serial('Card - NoThreeDS Create and Confirm Automatic CIT and MIT payment flow test', () => {
    test('customer-create-call-test', async ({ request, globalState }) => {
      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');

      const response = await request.post(`${baseUrl}/customers`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
        data: fixtures.customerCreateBody,
      });

      const body = await response.json();

      if (response.status() === 200) {
        globalState.set('customerId', body.customer_id);
        console.log(`✓ Customer created: ${body.customer_id}`);
      }
    });

    test('Create No 3DS Payment Intent', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm']['PaymentIntentOffSession'];

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
        authentication_type: 'no_three_ds',
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

    test('Confirm No 3DS CIT', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm'][
        'PaymentMethodIdMandateNo3DSAutoCapture'
      ];

      if (!shouldContinueFurther(data)) {
        test.skip();
        return;
      }

      const baseUrl = globalState.get('baseUrl');
      const publishableKey = globalState.get('publishableKey');
      const paymentId = globalState.get('paymentID');

      const confirmBody = {
        ...fixtures.citConfirmBody,
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

      if (body.payment_method_id) {
        globalState.set('paymentMethodId', body.payment_method_id);
      }
      if (body.mandate_id) {
        globalState.set('mandateId', body.mandate_id);
      }

      console.log(`✓ CIT confirmed: ${body.status}`);
    });

    test('retrieve-payment-call-test', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm'][
        'PaymentMethodIdMandateNo3DSAutoCapture'
      ];

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

    test('Confirm No 3DS MIT', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm']['MITAutoCapture'];

      if (!shouldContinueFurther(data)) {
        test.skip();
        return;
      }

      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');
      const customerId = globalState.get('customerId');
      const paymentMethodId = globalState.get('paymentMethodId');

      const mitBody = {
        ...fixtures.pmIdConfirmBody,
        ...data.Request,
        customer_id: customerId,
        payment_method_id: paymentMethodId,
        confirm: true,
      };

      const response = await request.post(`${baseUrl}/payments`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
        data: mitBody,
      });

      const body = await response.json();
      globalState.set('paymentID', body.payment_id);

      console.log(`✓ MIT payment confirmed: ${body.status}`);
    });

    test('retrieve-payment-call-test after MIT', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm']['MITAutoCapture'];

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
      console.log(`✓ MIT Payment retrieved: ${body.status}`);
    });
  });

  test.describe.serial('Card - NoThreeDS Create and Confirm Manual CIT and MIT payment flow test', () => {
    test('Create No 3DS Payment Intent', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm']['PaymentIntentOffSession'];

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
        authentication_type: 'no_three_ds',
        capture_method: 'manual',
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

    test('Confirm No 3DS CIT', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm'][
        'PaymentMethodIdMandateNo3DSManualCapture'
      ];

      if (!shouldContinueFurther(data)) {
        test.skip();
        return;
      }

      const baseUrl = globalState.get('baseUrl');
      const publishableKey = globalState.get('publishableKey');
      const paymentId = globalState.get('paymentID');

      const confirmBody = {
        ...fixtures.citConfirmBody,
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

      if (body.payment_method_id) {
        globalState.set('paymentMethodId', body.payment_method_id);
      }
      if (body.mandate_id) {
        globalState.set('mandateId', body.mandate_id);
      }

      console.log(`✓ CIT confirmed: ${body.status}`);
    });

    test('cit-capture-call-test', async ({ request, globalState }) => {
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
      console.log(`✓ CIT Payment captured: ${body.status}`);
    });

    test('retrieve-payment-call-test', async ({ request, globalState }) => {
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
      console.log(`✓ Payment retrieved: ${body.status}`);
    });

    test('Confirm No 3DS MIT', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm']['MITAutoCapture'];

      if (!shouldContinueFurther(data)) {
        test.skip();
        return;
      }

      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');
      const customerId = globalState.get('customerId');
      const paymentMethodId = globalState.get('paymentMethodId');

      const mitBody = {
        ...fixtures.pmIdConfirmBody,
        ...data.Request,
        customer_id: customerId,
        payment_method_id: paymentMethodId,
        confirm: true,
        capture_method: 'automatic',
      };

      const response = await request.post(`${baseUrl}/payments`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
        data: mitBody,
      });

      const body = await response.json();
      globalState.set('paymentID', body.payment_id);

      console.log(`✓ MIT payment confirmed: ${body.status}`);
    });

    test('retrieve-payment-call-test after MIT', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm']['MITAutoCapture'];

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
      console.log(`✓ MIT Payment retrieved: ${body.status}`);
    });
  });

  test.describe.serial('Card - NoThreeDS Create + Confirm Automatic CIT and MIT payment flow test', () => {
    test('Confirm No 3DS CIT', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm'][
        'PaymentMethodIdMandateNo3DSAutoCapture'
      ];

      if (!shouldContinueFurther(data)) {
        test.skip();
        return;
      }

      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');
      const customerId = globalState.get('customerId');

      const citBody = {
        ...fixtures.citConfirmBody,
        ...data.Request,
        customer_id: customerId,
        confirm: true,
      };

      const response = await request.post(`${baseUrl}/payments`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
        data: citBody,
      });

      const body = await response.json();

      globalState.set('paymentID', body.payment_id);

      if (body.payment_method_id) {
        globalState.set('paymentMethodId', body.payment_method_id);
      }
      if (body.mandate_id) {
        globalState.set('mandateId', body.mandate_id);
      }

      console.log(`✓ CIT payment confirmed: ${body.status}`);
    });

    test('retrieve-payment-call-test', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm'][
        'PaymentMethodIdMandateNo3DSAutoCapture'
      ];

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

    test('Confirm No 3DS MIT - first', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm']['MITAutoCapture'];

      if (!shouldContinueFurther(data)) {
        test.skip();
        return;
      }

      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');
      const customerId = globalState.get('customerId');
      const paymentMethodId = globalState.get('paymentMethodId');

      const mitBody = {
        ...fixtures.pmIdConfirmBody,
        ...data.Request,
        customer_id: customerId,
        payment_method_id: paymentMethodId,
        confirm: true,
        capture_method: 'automatic',
      };

      const response = await request.post(`${baseUrl}/payments`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
        data: mitBody,
      });

      const body = await response.json();
      globalState.set('paymentID', body.payment_id);

      console.log(`✓ MIT payment 1 confirmed: ${body.status}`);
    });

    test('retrieve-payment-call-test after first MIT', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm']['MITAutoCapture'];

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
      console.log(`✓ MIT Payment 1 retrieved: ${body.status}`);
    });

    test('Confirm No 3DS MIT - second', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm']['MITAutoCapture'];

      if (!shouldContinueFurther(data)) {
        test.skip();
        return;
      }

      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');
      const customerId = globalState.get('customerId');
      const paymentMethodId = globalState.get('paymentMethodId');

      const mitBody = {
        ...fixtures.pmIdConfirmBody,
        ...data.Request,
        customer_id: customerId,
        payment_method_id: paymentMethodId,
        confirm: true,
        capture_method: 'automatic',
      };

      const response = await request.post(`${baseUrl}/payments`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
        data: mitBody,
      });

      const body = await response.json();
      globalState.set('paymentID', body.payment_id);

      console.log(`✓ MIT payment 2 confirmed: ${body.status}`);
    });

    test('retrieve-payment-call-test after second MIT', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm']['MITAutoCapture'];

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
      console.log(`✓ MIT Payment 2 retrieved: ${body.status}`);
    });
  });

  test.describe.serial('Card - NoThreeDS Create + Confirm Manual CIT and MIT payment flow test', () => {
    test('Confirm No 3DS CIT', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm'][
        'PaymentMethodIdMandateNo3DSManualCapture'
      ];

      if (!shouldContinueFurther(data)) {
        test.skip();
        return;
      }

      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');
      const customerId = globalState.get('customerId');

      const citBody = {
        ...fixtures.citConfirmBody,
        ...data.Request,
        customer_id: customerId,
        confirm: true,
        capture_method: 'manual',
      };

      const response = await request.post(`${baseUrl}/payments`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
        data: citBody,
      });

      const body = await response.json();

      globalState.set('paymentID', body.payment_id);
      globalState.set('paymentAmount', citBody.amount);

      if (body.payment_method_id) {
        globalState.set('paymentMethodId', body.payment_method_id);
      }

      console.log(`✓ CIT payment confirmed: ${body.status}`);
    });

    test('cit-capture-call-test', async ({ request, globalState }) => {
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
      console.log(`✓ CIT Payment captured: ${body.status}`);
    });

    test('retrieve-payment-call-test', async ({ request, globalState }) => {
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
      console.log(`✓ Payment retrieved: ${body.status}`);
    });

    test('Confirm No 3DS MIT 1', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm']['MITManualCapture'];

      if (!shouldContinueFurther(data)) {
        test.skip();
        return;
      }

      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');
      const customerId = globalState.get('customerId');
      const paymentMethodId = globalState.get('paymentMethodId');

      const mitBody = {
        ...fixtures.pmIdConfirmBody,
        ...data.Request,
        customer_id: customerId,
        payment_method_id: paymentMethodId,
        confirm: true,
        capture_method: 'manual',
      };

      const response = await request.post(`${baseUrl}/payments`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
        data: mitBody,
      });

      const body = await response.json();
      globalState.set('paymentID', body.payment_id);
      globalState.set('paymentAmount', mitBody.amount);

      console.log(`✓ MIT payment 1 confirmed: ${body.status}`);
    });

    test('mit-capture-call-test 1', async ({ request, globalState }) => {
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
      console.log(`✓ MIT Payment 1 captured: ${body.status}`);
    });

    test('retrieve-payment-call-test after MIT 1', async ({ request, globalState }) => {
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
      console.log(`✓ MIT Payment 1 retrieved: ${body.status}`);
    });

    test('Confirm No 3DS MIT 2', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm']['MITManualCapture'];

      if (!shouldContinueFurther(data)) {
        test.skip();
        return;
      }

      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');
      const customerId = globalState.get('customerId');
      const paymentMethodId = globalState.get('paymentMethodId');

      const mitBody = {
        ...fixtures.pmIdConfirmBody,
        ...data.Request,
        customer_id: customerId,
        payment_method_id: paymentMethodId,
        confirm: true,
        capture_method: 'manual',
      };

      const response = await request.post(`${baseUrl}/payments`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
        data: mitBody,
      });

      const body = await response.json();
      globalState.set('paymentID', body.payment_id);
      globalState.set('paymentAmount', mitBody.amount);

      console.log(`✓ MIT payment 2 confirmed: ${body.status}`);
    });

    test('mit-capture-call-test 2', async ({ request, globalState }) => {
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
      console.log(`✓ MIT Payment 2 captured: ${body.status}`);
    });

    test('retrieve-payment-call-test after MIT 2', async ({ request, globalState }) => {
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
      console.log(`✓ MIT Payment 2 retrieved: ${body.status}`);
    });
  });

  test.describe.serial('Card - MIT without billing address', () => {
    test('Create No 3DS Payment Intent', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm']['PaymentIntentOffSession'];

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
        authentication_type: 'no_three_ds',
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

    test('Confirm No 3DS CIT', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm'][
        'PaymentMethodIdMandateNo3DSAutoCapture'
      ];

      if (!shouldContinueFurther(data)) {
        test.skip();
        return;
      }

      const baseUrl = globalState.get('baseUrl');
      const publishableKey = globalState.get('publishableKey');
      const paymentId = globalState.get('paymentID');

      const confirmBody = {
        ...fixtures.citConfirmBody,
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

      if (body.payment_method_id) {
        globalState.set('paymentMethodId', body.payment_method_id);
      }

      console.log(`✓ CIT confirmed: ${body.status}`);
    });

    test('Confirm No 3DS MIT', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm']['MITWithoutBillingAddress'];

      if (!shouldContinueFurther(data)) {
        test.skip();
        return;
      }

      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');
      const customerId = globalState.get('customerId');
      const paymentMethodId = globalState.get('paymentMethodId');

      const mitBody = {
        ...fixtures.pmIdConfirmBody,
        ...data.Request,
        customer_id: customerId,
        payment_method_id: paymentMethodId,
        confirm: true,
        capture_method: 'automatic',
      };

      const response = await request.post(`${baseUrl}/payments`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
        data: mitBody,
      });

      const body = await response.json();
      console.log(`✓ MIT without billing address confirmed: ${body.status}`);
    });
  });

  test.describe.serial('Card - ThreeDS Create + Confirm Automatic CIT and MIT payment flow test', () => {
    test('Confirm 3DS CIT', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm'][
        'PaymentMethodIdMandate3DSAutoCapture'
      ];

      if (!shouldContinueFurther(data)) {
        test.skip();
        return;
      }

      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');
      const customerId = globalState.get('customerId');

      const citBody = {
        ...fixtures.citConfirmBody,
        ...data.Request,
        customer_id: customerId,
        confirm: true,
        authentication_type: 'three_ds',
      };

      const response = await request.post(`${baseUrl}/payments`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
        data: citBody,
      });

      const body = await response.json();

      globalState.set('paymentID', body.payment_id);

      if (body.payment_method_id) {
        globalState.set('paymentMethodId', body.payment_method_id);
      }

      if (body.next_action?.redirect_to_url) {
        globalState.set('nextActionUrl', body.next_action.redirect_to_url);
      }

      console.log(`✓ 3DS CIT payment confirmed: ${body.status}`);
    });

    test('Handle redirection', async ({ page, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const expectedRedirection = fixtures.citConfirmBody.return_url;
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
      const data = getConnectorDetails(connectorId)['card_pm'][
        'PaymentMethodIdMandate3DSAutoCapture'
      ];

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

    test('Confirm No 3DS MIT - first', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm']['MITAutoCapture'];

      if (!shouldContinueFurther(data)) {
        test.skip();
        return;
      }

      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');
      const customerId = globalState.get('customerId');
      const paymentMethodId = globalState.get('paymentMethodId');

      const mitBody = {
        ...fixtures.pmIdConfirmBody,
        ...data.Request,
        customer_id: customerId,
        payment_method_id: paymentMethodId,
        confirm: true,
        capture_method: 'automatic',
      };

      const response = await request.post(`${baseUrl}/payments`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
        data: mitBody,
      });

      const body = await response.json();
      console.log(`✓ MIT payment 1 confirmed: ${body.status}`);
    });

    test('Confirm No 3DS MIT - second', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm']['MITAutoCapture'];

      if (!shouldContinueFurther(data)) {
        test.skip();
        return;
      }

      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');
      const customerId = globalState.get('customerId');
      const paymentMethodId = globalState.get('paymentMethodId');

      const mitBody = {
        ...fixtures.pmIdConfirmBody,
        ...data.Request,
        customer_id: customerId,
        payment_method_id: paymentMethodId,
        confirm: true,
        capture_method: 'automatic',
      };

      const response = await request.post(`${baseUrl}/payments`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
        data: mitBody,
      });

      const body = await response.json();
      console.log(`✓ MIT payment 2 confirmed: ${body.status}`);
    });
  });

  test.describe.serial('Card - ThreeDS Create + Confirm Manual CIT and MIT payment flow', () => {
    test('Confirm 3DS CIT', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm'][
        'PaymentMethodIdMandate3DSManualCapture'
      ];

      if (!shouldContinueFurther(data)) {
        test.skip();
        return;
      }

      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');
      const customerId = globalState.get('customerId');

      const citBody = {
        ...fixtures.citConfirmBody,
        ...data.Request,
        customer_id: customerId,
        confirm: true,
        authentication_type: 'three_ds',
        capture_method: 'manual',
      };

      const response = await request.post(`${baseUrl}/payments`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
        data: citBody,
      });

      const body = await response.json();

      globalState.set('paymentID', body.payment_id);
      globalState.set('paymentAmount', citBody.amount);

      if (body.payment_method_id) {
        globalState.set('paymentMethodId', body.payment_method_id);
      }

      if (body.next_action?.redirect_to_url) {
        globalState.set('nextActionUrl', body.next_action.redirect_to_url);
      }

      console.log(`✓ 3DS CIT payment confirmed: ${body.status}`);
    });

    test('Handle redirection', async ({ page, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const expectedRedirection = fixtures.citConfirmBody.return_url;
      const nextActionUrl = globalState.get('nextActionUrl');

      // Verify URLs exist
      expect(nextActionUrl).toBeTruthy();
      expect(expectedRedirection).toBeTruthy();

      // Handle 3DS authentication based on connector
      await handle3DSRedirection(page, connectorId, nextActionUrl, expectedRedirection);

      console.log('✓ 3DS authentication completed successfully');
    });

    test('cit-capture-call-test', async ({ request, globalState }) => {
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
      console.log(`✓ CIT Payment captured: ${body.status}`);
    });

    test('retrieve-payment-call-test', async ({ request, globalState }) => {
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
      console.log(`✓ Payment retrieved: ${body.status}`);
    });

    test('Confirm No 3DS MIT', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm']['MITAutoCapture'];

      if (!shouldContinueFurther(data)) {
        test.skip();
        return;
      }

      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');
      const customerId = globalState.get('customerId');
      const paymentMethodId = globalState.get('paymentMethodId');

      const mitBody = {
        ...fixtures.pmIdConfirmBody,
        ...data.Request,
        customer_id: customerId,
        payment_method_id: paymentMethodId,
        confirm: true,
        capture_method: 'automatic',
      };

      const response = await request.post(`${baseUrl}/payments`, {
        headers: {
          'Content-Type': 'application/json',
          'api-key': apiKey,
        },
        data: mitBody,
      });

      const body = await response.json();
      console.log(`✓ MIT payment confirmed: ${body.status}`);
    });
  });
});
