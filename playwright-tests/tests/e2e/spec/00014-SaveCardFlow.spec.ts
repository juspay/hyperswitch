/**
 * 00014 - Card SaveCard Payment Flow Test
 *
 * Ported from Cypress: cypress-tests/cypress/e2e/spec/Payment/00014-SaveCardFlow.cy.js
 *
 * Test scenarios:
 * - Save card for NoThreeDS automatic capture payment - Create+Confirm [on_session]
 * - Save card for NoThreeDS manual full capture payment - Create+Confirm [on_session]
 * - Save card for NoThreeDS manual partial capture payment - Create + Confirm [on_session]
 * - Save card for NoThreeDS automatic capture payment [off_session]
 * - Save card for NoThreeDS manual capture payment - Create+Confirm [off_session]
 * - Save card for NoThreeDS automatic capture payment - create and confirm [off_session]
 * - Use billing address from payment method during subsequent payment [off_session]
 * - Check if card fields are populated when saving card again after a metadata update
 */

import { test } from '../../fixtures/imports';
import { getConnectorDetails } from '../configs/Commons';

test.describe('Card - SaveCard payment flow test', () => {
  test.describe('Save card for NoThreeDS automatic capture payment - Create+Confirm [on_session]', () => {
    test('customer-create-call-test', async ({ request, globalState }) => {
      // TODO: Implement createCustomerCallTest helper
      // cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);

      console.log('TODO: Implement createCustomerCallTest');
    });

    test('create+confirm-payment-call-test', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.SaveCardUseNo3DSAutoCapture;

      if (!data) {
        test.skip();
        return;
      }

      // TODO: Implement createConfirmPaymentTest helper
      // cy.createConfirmPaymentTest(
      //   fixtures.createConfirmPaymentBody,
      //   data,
      //   "no_three_ds",
      //   "automatic",
      //   globalState
      // );

      console.log('TODO: Implement createConfirmPaymentTest for SaveCard with automatic capture');
    });

    test('retrieve-payment-call-test', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.SaveCardUseNo3DSAutoCapture;

      if (!data) {
        test.skip();
        return;
      }

      // TODO: Implement retrievePaymentCallTest helper
      // cy.retrievePaymentCallTest(globalState, data);

      console.log('TODO: Implement retrievePaymentCallTest');
    });

    test('retrieve-customerPM-call-test', async ({ request, globalState }) => {
      // TODO: Implement listCustomerPMCallTest helper
      // cy.listCustomerPMCallTest(globalState);

      console.log('TODO: Implement listCustomerPMCallTest');
    });

    test('create-payment-call-test', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.PaymentIntent;

      if (!data) {
        test.skip();
        return;
      }

      // TODO: Implement createPaymentIntentTest helper
      // cy.createPaymentIntentTest(
      //   fixtures.createPaymentBody,
      //   data,
      //   "no_three_ds",
      //   "automatic",
      //   globalState
      // );

      console.log('TODO: Implement createPaymentIntentTest');
    });

    test('confirm-save-card-payment-call-test', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.SaveCardUseNo3DSAutoCapture;

      if (!data) {
        test.skip();
        return;
      }

      // TODO: Implement saveCardConfirmCallTest helper
      // cy.saveCardConfirmCallTest(saveCardBody, data, globalState);

      console.log('TODO: Implement saveCardConfirmCallTest');
    });
  });

  test.describe('Save card for NoThreeDS manual full capture payment - Create+Confirm [on_session]', () => {
    test('customer-create-call-test', async ({ request, globalState }) => {
      // TODO: Implement createCustomerCallTest helper
      console.log('TODO: Implement createCustomerCallTest');
    });

    test('create+confirm-payment-call-test', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.SaveCardUseNo3DSAutoCapture;

      if (!data) {
        test.skip();
        return;
      }

      console.log('TODO: Implement createConfirmPaymentTest for SaveCard with automatic capture');
    });

    test('retrieve-payment-call-test', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.SaveCardUseNo3DSAutoCapture;

      if (!data) {
        test.skip();
        return;
      }

      console.log('TODO: Implement retrievePaymentCallTest');
    });

    test('retrieve-customerPM-call-test', async ({ request, globalState }) => {
      console.log('TODO: Implement listCustomerPMCallTest');
    });

    test('create-payment-call-test', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.PaymentIntent;

      if (!data) {
        test.skip();
        return;
      }

      console.log('TODO: Implement createPaymentIntentTest with manual capture');
    });

    test('confirm-save-card-payment-call-test', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.SaveCardUseNo3DSManualCapture;

      if (!data) {
        test.skip();
        return;
      }

      console.log('TODO: Implement saveCardConfirmCallTest with manual capture');
    });

    test('retrieve-payment-call-test (after confirm)', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.SaveCardUseNo3DSManualCapture;

      if (!data) {
        test.skip();
        return;
      }

      console.log('TODO: Implement retrievePaymentCallTest after confirm');
    });

    test('capture-call-test', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.Capture;

      if (!data) {
        test.skip();
        return;
      }

      console.log('TODO: Implement captureCallTest');
    });

    test('retrieve-payment-call-test (after capture)', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.Capture;

      if (!data) {
        test.skip();
        return;
      }

      console.log('TODO: Implement retrievePaymentCallTest after capture');
    });
  });

  test.describe('Save card for NoThreeDS manual partial capture payment - Create + Confirm [on_session]', () => {
    test('customer-create-call-test', async ({ request, globalState }) => {
      console.log('TODO: Implement createCustomerCallTest');
    });

    test('create+confirm-payment-call-test', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.SaveCardUseNo3DSAutoCapture;

      if (!data) {
        test.skip();
        return;
      }

      console.log('TODO: Implement createConfirmPaymentTest for SaveCard with automatic capture');
    });

    test('retrieve-payment-call-test', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.SaveCardUseNo3DSAutoCapture;

      if (!data) {
        test.skip();
        return;
      }

      console.log('TODO: Implement retrievePaymentCallTest');
    });

    test('retrieve-customerPM-call-test', async ({ request, globalState }) => {
      console.log('TODO: Implement listCustomerPMCallTest');
    });

    test('create-payment-call-test', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.PaymentIntent;

      if (!data) {
        test.skip();
        return;
      }

      console.log('TODO: Implement createPaymentIntentTest with manual capture');
    });

    test('confirm-save-card-payment-call-test', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.SaveCardUseNo3DSManualCapture;

      if (!data) {
        test.skip();
        return;
      }

      console.log('TODO: Implement saveCardConfirmCallTest with manual capture');
    });

    test('retrieve-payment-call-test (after confirm)', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.SaveCardUseNo3DSManualCapture;

      if (!data) {
        test.skip();
        return;
      }

      console.log('TODO: Implement retrievePaymentCallTest after confirm');
    });

    test('capture-call-test (partial)', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.PartialCapture;

      if (!data) {
        test.skip();
        return;
      }

      console.log('TODO: Implement captureCallTest for partial capture');
    });

    test('retrieve-payment-call-test (after partial capture)', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.PartialCapture;

      if (!data) {
        test.skip();
        return;
      }

      console.log('TODO: Implement retrievePaymentCallTest after partial capture');
    });
  });

  test.describe('Save card for NoThreeDS automatic capture payment [off_session]', () => {
    test('customer-create-call-test', async ({ request, globalState }) => {
      console.log('TODO: Implement createCustomerCallTest');
    });

    test('create+confirm-payment-call-test', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.SaveCardUseNo3DSAutoCaptureOffSession;

      if (!data) {
        test.skip();
        return;
      }

      console.log('TODO: Implement createConfirmPaymentTest for off_session automatic capture');
    });

    test('retrieve-payment-call-test', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.SaveCardUseNo3DSAutoCaptureOffSession;

      if (!data) {
        test.skip();
        return;
      }

      console.log('TODO: Implement retrievePaymentCallTest for off_session');
    });

    test('retrieve-customerPM-call-test', async ({ request, globalState }) => {
      console.log('TODO: Implement listCustomerPMCallTest');
    });

    test('create-payment-call-test', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.PaymentIntentOffSession;

      if (!data) {
        test.skip();
        return;
      }

      console.log('TODO: Implement createPaymentIntentTest for off_session');
    });

    test('confirm-save-card-payment-call-test', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.SaveCardConfirmAutoCaptureOffSession;

      if (!data) {
        test.skip();
        return;
      }

      console.log('TODO: Implement saveCardConfirmCallTest for off_session automatic capture');
    });
  });

  test.describe('Save card for NoThreeDS manual capture payment - Create+Confirm [off_session]', () => {
    test('customer-create-call-test', async ({ request, globalState }) => {
      console.log('TODO: Implement createCustomerCallTest');
    });

    test('create+confirm-payment-call-test', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.SaveCardUseNo3DSManualCaptureOffSession;

      if (!data) {
        test.skip();
        return;
      }

      console.log('TODO: Implement createConfirmPaymentTest for off_session manual capture');
    });

    test('retrieve-payment-call-test', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.SaveCardUseNo3DSManualCaptureOffSession;

      if (!data) {
        test.skip();
        return;
      }

      console.log('TODO: Implement retrievePaymentCallTest for off_session');
    });

    test('capture-call-test', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.Capture;

      if (!data) {
        test.skip();
        return;
      }

      console.log('TODO: Implement captureCallTest');
    });

    test('retrieve-payment-call-test (after capture)', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.Capture;

      if (!data) {
        test.skip();
        return;
      }

      console.log('TODO: Implement retrievePaymentCallTest after capture');
    });

    test('retrieve-customerPM-call-test', async ({ request, globalState }) => {
      console.log('TODO: Implement listCustomerPMCallTest');
    });

    test('create-payment-call-test', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.PaymentIntentOffSession;

      if (!data) {
        test.skip();
        return;
      }

      console.log('TODO: Implement createPaymentIntentTest for off_session');
    });

    test('confirm-save-card-payment-call-test', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.SaveCardConfirmManualCaptureOffSession;

      if (!data) {
        test.skip();
        return;
      }

      console.log('TODO: Implement saveCardConfirmCallTest for off_session manual capture');
    });

    test('retrieve-payment-call-test (after confirm)', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.SaveCardConfirmManualCaptureOffSession;

      if (!data) {
        test.skip();
        return;
      }

      console.log('TODO: Implement retrievePaymentCallTest after confirm');
    });

    test('capture-call-test (second)', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.Capture;

      if (!data) {
        test.skip();
        return;
      }

      console.log('TODO: Implement captureCallTest for second capture');
    });

    test('retrieve-payment-call-test (after second capture)', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.Capture;

      if (!data) {
        test.skip();
        return;
      }

      console.log('TODO: Implement retrievePaymentCallTest after second capture');
    });
  });

  test.describe('Save card for NoThreeDS automatic capture payment - create and confirm [off_session]', () => {
    test('customer-create-call-test', async ({ request, globalState }) => {
      console.log('TODO: Implement createCustomerCallTest');
    });

    test('create-payment-call-test', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.PaymentIntentOffSession;

      if (!data) {
        test.skip();
        return;
      }

      console.log('TODO: Implement createPaymentIntentTest for off_session');
    });

    test('confirm-payment-call-test', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.SaveCardUseNo3DSAutoCaptureOffSession;

      if (!data) {
        test.skip();
        return;
      }

      console.log('TODO: Implement confirmCallTest for off_session');
    });

    test('retrieve-payment-call-test', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.SaveCardUseNo3DSAutoCaptureOffSession;

      if (!data) {
        test.skip();
        return;
      }

      console.log('TODO: Implement retrievePaymentCallTest');
    });

    test('retrieve-customerPM-call-test', async ({ request, globalState }) => {
      console.log('TODO: Implement listCustomerPMCallTest');
    });

    test('create-payment-call-test (second)', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.PaymentIntentOffSession;

      if (!data) {
        test.skip();
        return;
      }

      console.log('TODO: Implement createPaymentIntentTest for second payment');
    });

    test('confirm-save-card-payment-call-test', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.SaveCardConfirmAutoCaptureOffSession;

      if (!data) {
        test.skip();
        return;
      }

      console.log('TODO: Implement saveCardConfirmCallTest for off_session automatic capture');
    });
  });

  test.describe('Use billing address from payment method during subsequent payment [off_session]', () => {
    test('customer-create-call-test', async ({ request, globalState }) => {
      console.log('TODO: Implement createCustomerCallTest');
    });

    test('create-payment-call-test', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.PaymentIntentOffSession;

      if (!data) {
        test.skip();
        return;
      }

      console.log('TODO: Implement createPaymentIntentTest for off_session');
    });

    test('confirm-payment-call-test', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.SaveCardUseNo3DSAutoCaptureOffSession;

      if (!data) {
        test.skip();
        return;
      }

      console.log('TODO: Implement confirmCallTest for off_session');
    });

    test('retrieve-customerPM-call-test', async ({ request, globalState }) => {
      console.log('TODO: Implement listCustomerPMCallTest');
    });

    test('create-payment-call-test (second)', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.PaymentIntentOffSession;

      if (!data) {
        test.skip();
        return;
      }

      console.log('TODO: Implement createPaymentIntentTest for second payment');
    });

    test('confirm-save-card-payment-call-test-without-billing', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.SaveCardConfirmAutoCaptureOffSessionWithoutBilling;

      if (!data) {
        test.skip();
        return;
      }

      console.log('TODO: Implement saveCardConfirmCallTest without billing address');
    });
  });

  test.describe('Check if card fields are populated when saving card again after a metadata update', () => {
    test('customer-create-call-test', async ({ request, globalState }) => {
      console.log('TODO: Implement createCustomerCallTest');
    });

    test('create+confirm-payment-call-test', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.SaveCardUseNo3DSAutoCapture;

      if (!data) {
        test.skip();
        return;
      }

      console.log('TODO: Implement createConfirmPaymentTest for SaveCard with automatic capture');
    });

    test('retrieve-customerPM-call-test', async ({ request, globalState }) => {
      console.log('TODO: Implement listCustomerPMCallTest');
    });

    test('create+confirm-payment-call-test (with metadata update)', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.SaveCardUseNo3DSAutoCapture;

      if (!data) {
        test.skip();
        return;
      }

      // TODO: Implement createConfirmPaymentTest with modified card_holder_name
      // const card_holder_name = generateRandomName();
      // const newData = {
      //   ...data,
      //   Request: {
      //     ...data.Request,
      //     payment_method_data: {
      //       card: {
      //         ...data.Request.payment_method_data.card,
      //         card_holder_name: card_holder_name,
      //       },
      //     },
      //   },
      // };

      console.log('TODO: Implement createConfirmPaymentTest with metadata update');
    });

    test('retrieve-customerPM-call-test (after update)', async ({ request, globalState }) => {
      console.log('TODO: Implement listCustomerPMCallTest to verify card fields');
    });
  });
});
