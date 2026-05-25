import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import * as utils from "../../configs/Payout/Utils";

let globalState;

describe("[Payout] Priority", () => {
  let shouldContinue = true;

  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
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

  context("Payout with priority=instant", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payout-with-priority-instant-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["PayoutPriority"];

      cy.createConfirmPayoutTest(
        fixtures.createPayoutBody,
        data,
        true,
        true,
        globalState
      );
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-payout-verify-priority-instant-test", () => {
      const data = {
        Response: {
          body: {
            priority: "instant",
          },
        },
      };
      cy.retrievePayoutCallTest(globalState, data);
    });
  });

  context("Payout without priority - required field error", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payout-without-priority-error-test", () => {
      delete fixtures.createPayoutBody.priority;
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["PayoutPriorityMissing"];

      cy.createConfirmPayoutTest(
        fixtures.createPayoutBody,
        data,
        true,
        true,
        globalState
      );
    });
  });

  context("Payout with priority=regular", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payout-with-priority-regular-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["PayoutPriorityRegular"];

      cy.createConfirmPayoutTest(
        fixtures.createPayoutBody,
        data,
        true,
        true,
        globalState
      );
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-payout-verify-priority-regular-test", () => {
      const data = {
        Response: {
          body: {
            priority: "regular",
          },
        },
      };
      cy.retrievePayoutCallTest(globalState, data);
    });
  });
});
