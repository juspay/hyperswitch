import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import * as utils from "../../configs/Payout/Utils";

let globalState;

describe("[Payout] Cards", () => {
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

  context("Payout Card with Auto Fulfill", () => {
    let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("confirm-payout-call-with-auto-fulfill-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["Fulfill"];

      cy.createConfirmPayoutTest(
        fixtures.createPayoutBody,
        data,
        true,
        true,
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-payout-call-test", () => {
      cy.retrievePayoutCallTest(globalState);
    });
  });

  context("Payout Card with Manual Fulfill - Create Confirm", () => {
    let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("confirm-payout-call-with-manual-fulfill-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["Confirm"];

      cy.createConfirmPayoutTest(
        fixtures.createPayoutBody,
        data,
        true,
        false,
        globalState
      );
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("fulfill-payout-call-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["Fulfill"];

      cy.fulfillPayoutCallTest({}, data, globalState);
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-payout-call-test", () => {
      cy.retrievePayoutCallTest(globalState);
    });
  });
});
