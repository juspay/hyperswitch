/**
 * 00015 - Card Zero Auth Mandate Flow Test
 *
 * Ported from Cypress: cypress-tests/cypress/e2e/spec/Payment/00015-ZeroAuthMandate.cy.js
 *
 * Test scenarios:
 * - Card - NoThreeDS Create + Confirm Automatic CIT and Single use MIT payment flow
 * - Card - NoThreeDS Create + Confirm Automatic CIT and Multi use MIT payment flow
 * - Card - Zero Auth Payment flow
 */

import { test } from '../../fixtures/imports';
import { getConnectorDetails } from '../configs/Commons';

test.describe('Card - SingleUse Mandates flow test', () => {
  test.describe('Card - NoThreeDS Create + Confirm Automatic CIT and Single use MIT payment flow test', () => {
    test('customer-create-call-test', async ({ request, globalState }) => {
      // TODO: Implement createCustomerCallTest helper
      // cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);

      console.log('TODO: Implement createCustomerCallTest');
    });

    test('Confirm No 3DS CIT', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.ZeroAuthMandate;

      if (!data) {
        test.skip();
        return;
      }

      // TODO: Implement citForMandatesCallTest helper
      // cy.citForMandatesCallTest(
      //   fixtures.citConfirmBody,
      //   data,
      //   0,
      //   true,
      //   "automatic",
      //   "setup_mandate",
      //   globalState
      // );

      console.log('TODO: Implement citForMandatesCallTest for Zero auth CIT');
    });

    test('retrieve-payment-call-test', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.ZeroAuthMandate;

      if (!data) {
        test.skip();
        return;
      }

      // TODO: Implement retrievePaymentCallTest helper
      // cy.retrievePaymentCallTest(globalState, data);

      console.log('TODO: Implement retrievePaymentCallTest for Zero auth CIT');
    });

    test('Confirm No 3DS MIT', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.MITAutoCapture;

      if (!data) {
        test.skip();
        return;
      }

      // TODO: Implement mitForMandatesCallTest helper
      // cy.mitForMandatesCallTest(
      //   fixtures.mitConfirmBody,
      //   data,
      //   6000,
      //   true,
      //   "automatic",
      //   globalState
      // );

      console.log('TODO: Implement mitForMandatesCallTest for No 3DS MIT with automatic capture');
    });

    test('retrieve-payment-call-test (after MIT)', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.MITAutoCapture;

      if (!data) {
        test.skip();
        return;
      }

      // TODO: Implement retrievePaymentCallTest helper
      // cy.retrievePaymentCallTest(globalState, data);

      console.log('TODO: Implement retrievePaymentCallTest for MIT');
    });
  });

  test.describe('Card - NoThreeDS Create + Confirm Automatic CIT and Multi use MIT payment flow test', () => {
    test('Confirm No 3DS CIT', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.ZeroAuthMandate;

      if (!data) {
        test.skip();
        return;
      }

      // TODO: Implement citForMandatesCallTest helper
      // cy.citForMandatesCallTest(
      //   fixtures.citConfirmBody,
      //   data,
      //   0,
      //   true,
      //   "automatic",
      //   "setup_mandate",
      //   globalState
      // );

      console.log('TODO: Implement citForMandatesCallTest for Zero auth CIT');
    });

    test('retrieve-payment-call-test', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.ZeroAuthMandate;

      if (!data) {
        test.skip();
        return;
      }

      // TODO: Implement retrievePaymentCallTest helper
      // cy.retrievePaymentCallTest(globalState, data);

      console.log('TODO: Implement retrievePaymentCallTest for Zero auth CIT');
    });

    test('Confirm No 3DS MIT (first)', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.MITAutoCapture;

      if (!data) {
        test.skip();
        return;
      }

      // TODO: Implement mitForMandatesCallTest helper
      // cy.mitForMandatesCallTest(
      //   fixtures.mitConfirmBody,
      //   data,
      //   6000,
      //   true,
      //   "automatic",
      //   globalState
      // );

      console.log('TODO: Implement mitForMandatesCallTest for first No 3DS MIT with automatic capture');
    });

    test('retrieve-payment-call-test (after first MIT)', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.MITAutoCapture;

      if (!data) {
        test.skip();
        return;
      }

      // TODO: Implement retrievePaymentCallTest helper
      // cy.retrievePaymentCallTest(globalState, data);

      console.log('TODO: Implement retrievePaymentCallTest for first MIT');
    });

    test('Confirm No 3DS MIT (second)', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.MITAutoCapture;

      if (!data) {
        test.skip();
        return;
      }

      // TODO: Implement mitForMandatesCallTest helper
      // cy.mitForMandatesCallTest(
      //   fixtures.mitConfirmBody,
      //   data,
      //   6000,
      //   true,
      //   "automatic",
      //   globalState
      // );

      console.log('TODO: Implement mitForMandatesCallTest for second No 3DS MIT with automatic capture');
    });

    test('retrieve-payment-call-test (after second MIT)', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.MITAutoCapture;

      if (!data) {
        test.skip();
        return;
      }

      // TODO: Implement retrievePaymentCallTest helper
      // cy.retrievePaymentCallTest(globalState, data);

      console.log('TODO: Implement retrievePaymentCallTest for second MIT');
    });
  });

  test.describe('Card - Zero Auth Payment', () => {
    test('Create No 3DS Payment Intent', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.ZeroAuthPaymentIntent;

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

      console.log('TODO: Implement createPaymentIntentTest for Zero auth payment intent');
    });

    test('Confirm No 3DS payment', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.ZeroAuthConfirmPayment;

      if (!data) {
        test.skip();
        return;
      }

      // TODO: Implement confirmCallTest helper
      // cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

      console.log('TODO: Implement confirmCallTest for Zero auth payment');
    });

    test('Retrieve Payment Call Test', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.ZeroAuthConfirmPayment;

      if (!data) {
        test.skip();
        return;
      }

      // TODO: Implement retrievePaymentCallTest helper
      // cy.retrievePaymentCallTest(globalState, data);

      console.log('TODO: Implement retrievePaymentCallTest for Zero auth payment');
    });

    test('Retrieve CustomerPM Call Test', async ({ request, globalState }) => {
      // TODO: Implement listCustomerPMCallTest helper
      // cy.listCustomerPMCallTest(globalState);

      console.log('TODO: Implement listCustomerPMCallTest');
    });

    test('Create Recurring Payment Intent', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.PaymentIntentOffSession;

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

      console.log('TODO: Implement createPaymentIntentTest for recurring payment');
    });

    test('Confirm Recurring Payment', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.SaveCardConfirmAutoCaptureOffSession;

      if (!data) {
        test.skip();
        return;
      }

      // TODO: Implement saveCardConfirmCallTest helper
      // cy.saveCardConfirmCallTest(
      //   fixtures.saveCardConfirmBody,
      //   data,
      //   globalState
      // );

      console.log('TODO: Implement saveCardConfirmCallTest for recurring payment');
    });

    test('retrieve-payment-call-test (after recurring)', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.SaveCardConfirmAutoCaptureOffSession;

      if (!data) {
        test.skip();
        return;
      }

      // TODO: Implement retrievePaymentCallTest helper
      // cy.retrievePaymentCallTest(globalState, data);

      console.log('TODO: Implement retrievePaymentCallTest for recurring payment');
    });
  });
});
