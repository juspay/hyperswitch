/**
 * 00011 - Card Single-Use Mandates Flow Test
 *
 * Ported from Cypress: cypress-tests/cypress/e2e/spec/Payment/00011-CreateSingleuseMandate.cy.js
 *
 * Test scenarios:
 * - Card - NoThreeDS Create + Confirm Automatic CIT and MIT payment flow
 * - Card - NoThreeDS Create + Confirm Manual CIT and MIT payment flow
 * - Card - No threeDS Create + Confirm Manual CIT and MIT payment flow
 */

import { test } from '../../fixtures/imports';
import { getConnectorDetails } from '../configs/Commons';

test.describe('Card - SingleUse Mandates flow test', () => {
  test.describe('Card - NoThreeDS Create + Confirm Automatic CIT and MIT payment flow test', () => {
    test('Confirm No 3DS CIT', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.MandateSingleUseNo3DSAutoCapture;

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

  test.describe('Card - NoThreeDS Create + Confirm Manual CIT and MIT payment flow test', () => {
    test('Confirm No 3DS CIT', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.MandateSingleUseNo3DSManualCapture;

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

      console.log('TODO: Implement mitForMandatesCallTest for No 3DS MIT with manual capture');
    });

    test('mit-capture-call-test', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.Capture;

      if (!data) {
        test.skip();
        return;
      }

      // TODO: Implement captureCallTest helper
      // cy.captureCallTest(fixtures.captureBody, data, globalState);

      console.log('TODO: Implement captureCallTest for MIT capture');
    });

    test('list-mandate-call-test', async ({ request, globalState }) => {
      // TODO: Implement listMandateCallTest helper
      // cy.listMandateCallTest(globalState);

      console.log('TODO: Implement listMandateCallTest');
    });
  });

  test.describe('Card - No threeDS Create + Confirm Manual CIT and MIT payment flow test', () => {
    test('Create No 3DS CIT', async ({ request, globalState }) => {
      const connectorDetails = getConnectorDetails(globalState.get('connectorId'));
      const data = connectorDetails.card_pm?.MandateSingleUseNo3DSManualCapture;

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

      console.log('TODO: Implement citForMandatesCallTest for Create No 3DS CIT');
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

    test('list-mandate-call-test', async ({ request, globalState }) => {
      // TODO: Implement listMandateCallTest helper
      // cy.listMandateCallTest(globalState);

      console.log('TODO: Implement listMandateCallTest');
    });
  });
});
