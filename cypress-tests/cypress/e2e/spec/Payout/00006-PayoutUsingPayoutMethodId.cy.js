import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import * as utils from "../../configs/Payout/Utils";

let globalState;
let payoutBody;

describe("[Payout] Saved Bank transfer", () => {
  let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);

      // Check if the connector supports card payouts (based on the connector configuration in creds)
      if (!globalState.get("payoutsExecution")) {
        shouldContinue = false;
      }
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  beforeEach(function () {
    if (!shouldContinue) {
      this.skip();
    }
  });

  context(
    "[Payout] [Bank Transfer] Onboard Customer Prior to Transaction",
    () => {
      let shouldContinue = true; // variable that will be used to skip tests if a previous test fails
      beforeEach("reset payoutBody", () => {
        payoutBody = Cypress._.cloneDeep(fixtures.createPayoutBody);
      });

      it("create customer", () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      it("create payment method", () => {
        const data = utils.getConnectorDetails(globalState.get("connectorId"))[
          "bank_transfer_pm"
        ]["sepa_bank_transfer"]["SavePayoutMethod"];

        cy.createPaymentMethodTest(globalState, data);
      });

      it("list customer payment methods", () => {
        cy.listCustomerPMCallTest(globalState);
      });

      it("[Payout] [Bank transfer] [SEPA] Fulfill using Token", () => {
        const data = utils.getConnectorDetails(globalState.get("connectorId"))[
          "bank_transfer_pm"
        ]["sepa_bank_transfer"]["Token"];

        cy.createConfirmWithTokenPayoutTest(
          payoutBody,
          data,
          true,
          true,
          globalState
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("retrieve-payout-call-test", () => {
        cy.retrievePayoutCallTest(globalState);
      });
    }
  );

  context(
    "[Payout] [Bank Transfer] Save payment method after successful transaction",
    () => {
      let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("create customer", () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      it("confirm-payout-call-with-auto-fulfill-test", () => {
        const data = utils.getConnectorDetails(globalState.get("connectorId"))[
          "bank_transfer_pm"
        ]["sepa_bank_transfer"]["Fulfill"];

        cy.createConfirmPayoutTest(payoutBody, data, true, true, globalState);

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("list customer payment methods", () => {
        cy.listCustomerPMCallTest(globalState);
      });

      it("[Payout] [Bank transfer] [SEPA] Fulfill using payout_method_id", () => {
        const data = utils.getConnectorDetails(globalState.get("connectorId"))[
          "bank_transfer_pm"
        ]["sepa_bank_transfer"]["Token"];

        cy.createConfirmWithPayoutMethodIdTest(
          payoutBody,
          data,
          true,
          true,
          globalState
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("retrieve-payout-call-test", () => {
        cy.retrievePayoutCallTest(globalState);
      });
    }
  );
});
