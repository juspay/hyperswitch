import * as fixtures from "../../fixtures/imports";
import State from "../../utils/State";
import * as utils from "../PayoutUtils/Utils";

let globalState;

describe("[Payout] Cards", () => {
  let should_continue = true; // variable that will be used to skip tests if a previous test fails

  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);

      // Check if the connector supports card payouts (based on the connector configuration in creds)
      if (!globalState.get("payoutsExecution")) {
        should_continue = false;
      }
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  beforeEach(function () {
    if (!should_continue) {
      this.skip();
    }
  });

  context("Payout Card with Auto Fulfill", () => {
    let should_continue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!should_continue) {
        this.skip();
      }
    });

    it("confirm-payout-call-with-auto-fulfill-test", () => {
      let data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["Fulfill"];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.createConfirmPayoutTest(
        fixtures.createPayoutBody,
        req_data,
        res_data,
        true,
        true,
        globalState
      );

      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("retrieve-payout-call-test", () => {
      cy.retrievePayoutCallTest(globalState);
    });
  });

  context("Payout Card with Manual Fulfill - Create Confirm", () => {
    let should_continue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!should_continue) {
        this.skip();
      }
    });

    it("confirm-payout-call-with-manual-fulfill-test", () => {
      let data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["Confirm"];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.createConfirmPayoutTest(
        fixtures.createPayoutBody,
        req_data,
        res_data,
        true,
        false,
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("fulfill-payout-call-test", () => {
      let data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["Fulfill"];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.fulfillPayoutCallTest({}, req_data, res_data, globalState);
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("retrieve-payout-call-test", () => {
      cy.retrievePayoutCallTest(globalState);
    });
  });
});
