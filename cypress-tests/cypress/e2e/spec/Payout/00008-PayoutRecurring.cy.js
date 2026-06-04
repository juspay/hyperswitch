import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import * as utils from "../../configs/Payout/Utils";

let globalState;
let payoutBody;

describe("[Payout] Recurring", () => {
  let shouldContinue = true;

  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);

      // Check if the connector supports payouts and is in the inclusion list for recurring tests
      if (
        !globalState.get("payoutsExecution") ||
        !utils.CONNECTOR_LISTS?.INCLUDE?.PAYOUT_RECURRING?.includes(
          globalState.get("connectorId")
        )
      ) {
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

  context("Create payout with recurring=true", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    beforeEach("reset payoutBody", () => {
      payoutBody = Cypress._.cloneDeep(fixtures.createPayoutBody);
    });

    it("create-payout-with-recurring-true", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["sepa_bank_transfer"]["RecurringTrue"];

      cy.createConfirmPayoutTest(payoutBody, data, true, true, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);

      // Capture payout_method_id from response if present
      cy.task("getGlobalState").then((state) => {
        const payoutData = state.data;
        if (payoutData && payoutData.payoutId) {
          cy.task("cli_log", `Payout created with ID: ${payoutData.payoutId}`);
        }
      });
    });

    it("retrieve-payout-call-test", () => {
      cy.retrievePayoutCallTest(globalState);
    });
  });

  context("Create payout with recurring=false", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    beforeEach("reset payoutBody", () => {
      payoutBody = Cypress._.cloneDeep(fixtures.createPayoutBody);
    });

    it("create-payout-with-recurring-false", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["sepa_bank_transfer"]["RecurringFalse"];

      cy.createConfirmPayoutTest(payoutBody, data, true, true, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-payout-call-test", () => {
      cy.retrievePayoutCallTest(globalState);
    });
  });

  context("Create payout without recurring field (defaults to false)", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    beforeEach("reset payoutBody", () => {
      payoutBody = Cypress._.cloneDeep(fixtures.createPayoutBody);
    });

    it("create-payout-with-recurring-omitted", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["sepa_bank_transfer"]["RecurringDefault"];

      cy.createConfirmPayoutTest(payoutBody, data, true, true, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-payout-call-test", () => {
      cy.retrievePayoutCallTest(globalState);
    });
  });

  context("Validation: payout_method_id requires confirm=true", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    beforeEach("reset payoutBody", () => {
      payoutBody = Cypress._.cloneDeep(fixtures.createPayoutBody);
    });

    it("attempt-payout-without-confirm-should-fail", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["sepa_bank_transfer"]["RecurringInvalidConfirm"];

      // This test validates that using payout_method_id with confirm=false returns error
      cy.createConfirmPayoutTest(payoutBody, data, false, false, globalState);

      // For error responses, we expect should_continue_further to return false
      // but the test itself should pass (asserting the error)
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
  });
});
