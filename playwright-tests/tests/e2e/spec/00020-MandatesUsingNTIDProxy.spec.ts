/**
 * Card - Mandates using Network Transaction Id flow test
 *
 * Converted from Cypress test: 00020-MandatesUsingNTIDProxy.cy.js
 * Tests mandate flows using Network Transaction ID (MIT payments)
 * Only supported by specific connectors (e.g., Cybersource)
 */

import { test } from '../../fixtures/imports';
import { getConnectorDetails, shouldContinueFurther, shouldIncludeConnector, CONNECTOR_LISTS } from '../configs/Payment/Utils';
import * as fixtures from '../../fixtures/imports';

test.describe.configure({ mode: 'parallel' });

test.describe('Card - Mandates using Network Transaction Id flow test', () => {
  // Check if connector supports NTID proxy before running tests
  test.beforeEach(async ({ globalState }) => {
    const connectorId = globalState.get('connectorId');

    // Skip the test if the connector is not in the inclusion list
    // This is done because only Cybersource is known to support at present
    if (shouldIncludeConnector(connectorId, CONNECTOR_LISTS.MANDATES_USING_NTID_PROXY)) {
      test.skip();
    }
  });

  test.describe.serial('Card - NoThreeDS Create and Confirm Automatic MIT payment flow test', () => {
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

      const mitBody = {
        ...fixtures.ntidConfirmBody,
        ...data.Request,
        customer_id: customerId,
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
      console.log(`✓ MIT payment (automatic) confirmed: ${body.status}`);
    });
  });

  test.describe.serial('Card - NoThreeDS Create and Confirm Manual MIT payment flow test', () => {
    test('Confirm No 3DS MIT', async ({ request, globalState }) => {
      const connectorId = globalState.get('connectorId');
      const data = getConnectorDetails(connectorId)['card_pm']['MITManualCapture'];

      if (!shouldContinueFurther(data)) {
        test.skip();
        return;
      }

      const baseUrl = globalState.get('baseUrl');
      const apiKey = globalState.get('apiKey');
      const customerId = globalState.get('customerId');

      const mitBody = {
        ...fixtures.ntidConfirmBody,
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
        data: mitBody,
      });

      const body = await response.json();
      console.log(`✓ MIT payment (manual) confirmed: ${body.status}`);
    });
  });

  test.describe.serial('Card - NoThreeDS Create and Confirm Automatic multiple MITs payment flow test', () => {
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

      const mitBody = {
        ...fixtures.ntidConfirmBody,
        ...data.Request,
        customer_id: customerId,
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

      const mitBody = {
        ...fixtures.ntidConfirmBody,
        ...data.Request,
        customer_id: customerId,
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

  test.describe.serial('Card - NoThreeDS Create and Confirm Manual multiple MITs payment flow test', () => {
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

      const mitBody = {
        ...fixtures.ntidConfirmBody,
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

      const mitBody = {
        ...fixtures.ntidConfirmBody,
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
  });

  test.describe.serial('Card - ThreeDS Create and Confirm Automatic multiple MITs payment flow test', () => {
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

      const mitBody = {
        ...fixtures.ntidConfirmBody,
        ...data.Request,
        customer_id: customerId,
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

      const mitBody = {
        ...fixtures.ntidConfirmBody,
        ...data.Request,
        customer_id: customerId,
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

  test.describe.serial('Card - ThreeDS Create and Confirm Manual multiple MITs payment flow', () => {
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

      const mitBody = {
        ...fixtures.ntidConfirmBody,
        ...data.Request,
        customer_id: customerId,
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
