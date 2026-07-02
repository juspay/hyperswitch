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
      cy.createConfirmRecurringPayout(
        globalState,
        "RecurringTrue",
        true,
        false
      ).then((shouldProceed) => {
        if (!shouldProceed) shouldContinue = false;
      });
    });

    it("fulfill-recurring-payout-test", () => {
      cy.getPayoutRecurringData(globalState, "RecurringTrueFulfill").then(
        ({ data, shouldContinue: shouldProceed }) => {
          if (!shouldProceed) {
            shouldContinue = false;
            return;
          }
          cy.fulfillPayoutCallTest({}, data, globalState);
          if (shouldContinue)
            shouldContinue = utils.should_continue_further(data);
        }
      );
    });

    it("create-recurring-payout-using-saved-method", () => {
      cy.createConfirmRecurringPayout(
        globalState,
        "RecurringUseMethod",
        true,
        false
      ).then((shouldProceed) => {
        if (!shouldProceed) shouldContinue = false;
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

    it("create-payout-with-recurring-false", () => {
      cy.createConfirmRecurringPayout(
        globalState,
        "RecurringFalse",
        true,
        false
      ).then((shouldProceed) => {
        if (!shouldProceed) shouldContinue = false;
      });
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
      cy.createConfirmRecurringPayout(
        globalState,
        "RecurringDefault",
        true,
        false
      ).then((shouldProceed) => {
        if (!shouldProceed) shouldContinue = false;
      });
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
      cy.createConfirmRecurringPayout(
        globalState,
        "RecurringInvalidConfirm",
        false,
        false
      ).then((shouldProceed) => {
        if (!shouldProceed) shouldContinue = false;
      });
    });
  });
});
