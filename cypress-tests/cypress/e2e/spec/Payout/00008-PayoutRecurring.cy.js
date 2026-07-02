import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import * as utils from "../../configs/Payout/Utils";

let globalState;

describe("[Payout] Recurring", () => {
  let shouldContinue = true;

  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);

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

    it("create-payout-with-recurring-true", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["sepa_bank_transfer"]["RecurringTrue"];
      if (!utils.should_continue_further(data)) {
        shouldContinue = false;
        return;
      }
      cy.createConfirmPayoutTest(
        fixtures.createPayoutBody,
        data,
        true,
        false,
        globalState
      ).then((response) => {
        if (response.body.payout_method_id) {
          globalState.set("payoutMethodId", response.body.payout_method_id);
        }
      });
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("fulfill-recurring-payout-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["sepa_bank_transfer"]["RecurringTrueFulfill"];
      cy.fulfillPayoutCallTest({}, data, globalState);
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("create-recurring-payout-using-saved-method", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["sepa_bank_transfer"]["RecurringUseMethod"];
      if (!utils.should_continue_further(data)) {
        shouldContinue = false;
        return;
      }
      data.Request.payout_method_id = globalState.get("payoutMethodId");
      cy.createConfirmPayoutTest(
        fixtures.createPayoutBody,
        data,
        true,
        false,
        globalState
      ).then((response) => {
        if (response.body.payout_method_id) {
          globalState.set("payoutMethodId", response.body.payout_method_id);
        }
      });
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
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

    it("create-payout-with-recurring-false", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["sepa_bank_transfer"]["RecurringFalse"];
      if (!utils.should_continue_further(data)) {
        shouldContinue = false;
        return;
      }
      cy.createConfirmPayoutTest(
        fixtures.createPayoutBody,
        data,
        true,
        false,
        globalState
      ).then((response) => {
        if (response.body.payout_method_id) {
          globalState.set("payoutMethodId", response.body.payout_method_id);
        }
      });
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

    it("create-payout-with-recurring-omitted", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["sepa_bank_transfer"]["RecurringDefault"];
      if (!utils.should_continue_further(data)) {
        shouldContinue = false;
        return;
      }
      cy.createConfirmPayoutTest(
        fixtures.createPayoutBody,
        data,
        true,
        false,
        globalState
      ).then((response) => {
        if (response.body.payout_method_id) {
          globalState.set("payoutMethodId", response.body.payout_method_id);
        }
      });
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

    it("attempt-payout-without-confirm-should-fail", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["sepa_bank_transfer"]["RecurringInvalidConfirm"];
      if (!utils.should_continue_further(data)) {
        shouldContinue = false;
        return;
      }
      data.Request.payout_method_id = globalState.get("payoutMethodId");
      cy.createConfirmPayoutTest(
        fixtures.createPayoutBody,
        data,
        false,
        false,
        globalState
      ).then((response) => {
        if (response.body.payout_method_id) {
          globalState.set("payoutMethodId", response.body.payout_method_id);
        }
      });
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
  });
});
