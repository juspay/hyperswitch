import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import * as utils from "../../configs/Payout/Utils";

let globalState;

describe("[Payout] Payout Link", () => {
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

  context("Payout Link with Card - Auto Fulfill", () => {
    let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payout-link-with-card-auto-fulfill-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "payout_link_pm"
      ]["CardAutoFulfill"];

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

  context("Payout Link with Card - Manual Fulfill", () => {
    let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payout-link-with-card-manual-fulfill-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "payout_link_pm"
      ]["CardManualFulfill"];

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
        "payout_link_pm"
      ]["CardFulfill"];

      cy.fulfillPayoutCallTest({}, data, globalState);
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-payout-call-test", () => {
      cy.retrievePayoutCallTest(globalState);
    });
  });

  context("Payout Link with Bank Transfer - Auto Fulfill", () => {
    let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payout-link-with-bank-transfer-auto-fulfill-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "payout_link_pm"
      ]["BankTransferAutoFulfill"];

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

  context("Payout Link with Bank Transfer - Manual Fulfill", () => {
    let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payout-link-with-bank-transfer-manual-fulfill-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "payout_link_pm"
      ]["BankTransferManualFulfill"];

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
        "payout_link_pm"
      ]["BankTransferFulfill"];

      cy.fulfillPayoutCallTest({}, data, globalState);
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-payout-call-test", () => {
      cy.retrievePayoutCallTest(globalState);
    });
  });
});
