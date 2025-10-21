/**
 * 00012 - Card Multi-Use Mandates Flow Test
 *
 * Ported from Cypress: cypress-tests/cypress/e2e/spec/Payment/00012-CreateMultiuseMandate.cy.js
 *
 * Test scenarios:
 * - Card - NoThreeDS Create + Confirm Automatic CIT and MIT payment flow
 * - Card - NoThreeDS Create + Confirm Manual CIT and MIT payment flow (with multiple MIT transactions)
 * - Card - ThreeDS Create + Confirm Manual CIT and MIT payment flow
 */

import { test } from '../../fixtures/imports';
import { getConnectorDetails } from '../configs/Commons';

test.describe.serial('Card - MultiUse Mandates flow test', () => {
  test.describe.serial('Card - NoThreeDS Create + Confirm Automatic CIT and MIT payment flow test', () => {
    test('Confirm No 3DS CIT', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.MandateMultiUseNo3DSAutoCapture;

      if (!data) {
        test.skip();
        return;
      }

      // TODO: Implement citForMandatesCallTest helper
      // cy.citForMandatesCallTest(
      //   fixtures.citConfirmBody,
      //   data,
      //   6000,
      //   true,
      //   "automatic",
      //   "new_mandate",
      //   globalState
      // );

      console.log('TODO: Implement citForMandatesCallTest for No 3DS CIT with automatic capture');
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
  });

  test.describe.serial('Card - NoThreeDS Create + Confirm Manual CIT and MIT payment flow test', () => {
    test('Confirm No 3DS CIT', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.MandateMultiUseNo3DSManualCapture;

      if (!data) {
        test.skip();
        return;
      }

      // TODO: Implement citForMandatesCallTest helper
      // cy.citForMandatesCallTest(
      //   fixtures.citConfirmBody,
      //   data,
      //   6000,
      //   true,
      //   "manual",
      //   "new_mandate",
      //   globalState
      // );

      console.log('TODO: Implement citForMandatesCallTest for No 3DS CIT with manual capture');
    });

    test('cit-capture-call-test', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.Capture;

      if (!data) {
        test.skip();
        return;
      }

      // TODO: Implement captureCallTest helper
      // cy.captureCallTest(fixtures.captureBody, data, globalState);

      console.log('TODO: Implement captureCallTest for CIT capture');
    });

    test('Confirm No 3DS MIT 1', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.MITManualCapture;

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
      //   "manual",
      //   globalState
      // );

      console.log('TODO: Implement mitForMandatesCallTest for first No 3DS MIT with manual capture');
    });

    test('mit-capture-call-test (first)', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.Capture;

      if (!data) {
        test.skip();
        return;
      }

      // TODO: Implement captureCallTest helper
      // cy.captureCallTest(fixtures.captureBody, data, globalState);

      console.log('TODO: Implement captureCallTest for first MIT capture');
    });

    test('Confirm No 3DS MIT 2', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.MITManualCapture;

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
      //   "manual",
      //   globalState
      // );

      console.log('TODO: Implement mitForMandatesCallTest for second No 3DS MIT with manual capture');
    });

    test('mit-capture-call-test (second)', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.Capture;

      if (!data) {
        test.skip();
        return;
      }

      // TODO: Implement captureCallTest helper
      // cy.captureCallTest(fixtures.captureBody, data, globalState);

      console.log('TODO: Implement captureCallTest for second MIT capture');
    });
  });

  test.describe.serial('Card - ThreeDS Create + Confirm Manual CIT and MIT payment flow test', () => {
    test('Confirm No 3DS CIT', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.MandateMultiUseNo3DSManualCapture;

      if (!data) {
        test.skip();
        return;
      }

      // TODO: Implement citForMandatesCallTest helper
      // cy.citForMandatesCallTest(
      //   fixtures.citConfirmBody,
      //   data,
      //   6000,
      //   true,
      //   "manual",
      //   "new_mandate",
      //   globalState
      // );

      console.log('TODO: Implement citForMandatesCallTest for No 3DS CIT with manual capture');
    });

    test('cit-capture-call-test', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.Capture;

      if (!data) {
        test.skip();
        return;
      }

      // TODO: Implement captureCallTest helper
      // cy.captureCallTest(fixtures.captureBody, data, globalState);

      console.log('TODO: Implement captureCallTest for CIT capture');
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
  });
});
