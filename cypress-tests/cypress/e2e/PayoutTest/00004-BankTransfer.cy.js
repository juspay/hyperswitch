import createPayoutBody from "../../fixtures/create-payout-confirm-body.json";
import State from "../../utils/State";
import * as utils from "../PayoutUtils/utils";

let globalState;

// TODO: Add test for Bank Transfer - ACH
describe.skip("[Payout] [Bank Transfer - ACH]", () => {
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
});

// TODO: Add test for Bank Transfer - BACS
describe.skip("[Payout] [Bank Transfer - BACS]", () => {
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
});

describe("[Payout] [Bank Transfer - SEPA]", () => {
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

  context("[Payout] [Bank transfer - SEPA] Auto Fulfill", () => {
    let should_continue = true; // variable that will be used to skip tests if a previous test fails

    it("confirm-payout-call-with-auto-fulfill-test", () => {
      let data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["sepa"]["Fulfill"];

      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.createConfirmPayoutTest(
        createPayoutBody,
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

  context("[Payout] [Bank transfer - SEPA] Manual Fulfill", () => {
    let should_continue = true; // variable that will be used to skip tests if a previous test fails

    it("confirm-payout-call-with-manual-fulfill-test", () => {
      let data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["sepa"]["Confirm"];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.createConfirmPayoutTest(
        createPayoutBody,
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
        "bank_transfer_pm"
      ]["sepa"]["Fulfill"];
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

  context("[Payout] [Bank transfer - SEPA] Manual Confirm", () => {
    let should_continue = true; // variable that will be used to skip tests if a previous test fails

    it("confirm-payout-call-with-manual-confirm-test", () => {
      let data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["sepa"]["Create"];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.createConfirmPayoutTest(
        createPayoutBody,
        req_data,
        res_data,
        false,
        true,
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("confirm-payout-call", () => {
      let data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["sepa"]["Confirm"];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.updatePayoutCallTest({}, req_data, res_data, false, globalState);
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("fulfill-payout-call-test", () => {
      let data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["sepa"]["Fulfill"];
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

  context("[Payout] [Bank transfer - SEPA] Manual", () => {
    let should_continue = true; // variable that will be used to skip tests if a previous test fails

    it("create-payout-call", () => {
      let data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["sepa"]["Create"];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.createConfirmPayoutTest(
        createPayoutBody,
        req_data,
        res_data,
        false,
        false,
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("confirm-payout-call", () => {
      let data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["sepa"]["Confirm"];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.updatePayoutCallTest({}, req_data, res_data, false, globalState);
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("fulfill-payout-call-test", () => {
      let data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["sepa"]["Fulfill"];
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
