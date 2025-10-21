/**
 * 00013 - Card List and Revoke Mandates Flow Test
 *
 * Ported from Cypress: cypress-tests/cypress/e2e/spec/Payment/00013-ListAndRevokeMandate.cy.js
 *
 * Test scenarios:
 * - Card - NoThreeDS Create + Confirm Automatic CIT and MIT payment flow with list and revoke
 * - Card - Zero auth CIT and MIT payment flow with list and revoke
 */

import { test } from '../../fixtures/imports';
import { getConnectorDetails } from '../configs/Commons';

test.describe.serial('Card - List and revoke Mandates flow test', () => {
  test.describe.serial('Card - NoThreeDS Create + Confirm Automatic CIT and MIT payment flow test', () => {
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

    test('list-mandate-call-test', async ({ request, globalState }) => {
      // TODO: Implement listMandateCallTest helper
      // cy.listMandateCallTest(globalState);

      console.log('TODO: Implement listMandateCallTest');
    });

    test('revoke-mandate-call-test', async ({ request, globalState }) => {
      // TODO: Implement revokeMandateCallTest helper
      // cy.revokeMandateCallTest(globalState);

      console.log('TODO: Implement revokeMandateCallTest');
    });

    test('revoke-revoked-mandate-call-test', async ({ request, globalState }) => {
      // TODO: Implement revokeMandateCallTest helper for already revoked mandate
      // cy.revokeMandateCallTest(globalState);

      console.log('TODO: Implement revokeMandateCallTest for already revoked mandate');
    });
  });

  test.describe.serial('Card - Zero auth CIT and MIT payment flow test', () => {
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

    test('list-mandate-call-test (after CIT)', async ({ request, globalState }) => {
      // TODO: Implement listMandateCallTest helper
      // cy.listMandateCallTest(globalState);

      console.log('TODO: Implement listMandateCallTest after CIT');
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

    test('list-mandate-call-test (after MIT)', async ({ request, globalState }) => {
      // TODO: Implement listMandateCallTest helper
      // cy.listMandateCallTest(globalState);

      console.log('TODO: Implement listMandateCallTest after MIT');
    });

    test('revoke-mandate-call-test', async ({ request, globalState }) => {
      // TODO: Implement revokeMandateCallTest helper
      // cy.revokeMandateCallTest(globalState);

      console.log('TODO: Implement revokeMandateCallTest');
    });
  });
});
