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

  // Entity Type test cases
  context("Create payout with entity_type=Company", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payout-with-entity-type-company", () => {
      cy.createConfirmRecurringPayout(
        globalState,
        "EntityTypeCompany",
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

  context("Create payout with entity_type=Individual", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payout-with-entity-type-individual", () => {
      cy.createConfirmRecurringPayout(
        globalState,
        "EntityTypeIndividual",
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

  context("Create payout with entity_type=NaturalPerson", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payout-with-entity-type-natural-person", () => {
      cy.createConfirmRecurringPayout(
        globalState,
        "EntityTypeNaturalPerson",
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

  context("Create payout with entity_type=NonProfit", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payout-with-entity-type-non-profit", () => {
      cy.createConfirmRecurringPayout(
        globalState,
        "EntityTypeNonProfit",
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

  context("Create payout with entity_type=Personal", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payout-with-entity-type-personal", () => {
      cy.createConfirmRecurringPayout(
        globalState,
        "EntityTypePersonal",
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

  context("Create payout with entity_type=PublicSector", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payout-with-entity-type-public-sector", () => {
      cy.createConfirmRecurringPayout(
        globalState,
        "EntityTypePublicSector",
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

  context(
    "Create payout without entity_type field (defaults to Individual)",
    () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("create-payout-with-entity-type-default", () => {
        cy.createConfirmRecurringPayout(
          globalState,
          "EntityTypeDefault",
          true,
          false
        ).then((shouldProceed) => {
          if (!shouldProceed) shouldContinue = false;
        });
      });

      it("retrieve-payout-call-test", () => {
        cy.retrievePayoutCallTest(globalState);
      });
    }
  );

  context("Validation: invalid entity_type should fail", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payout-with-invalid-entity-type-should-fail", () => {
      cy.createConfirmRecurringPayout(
        globalState,
        "EntityTypeInvalid",
        false,
        false
      ).then((shouldProceed) => {
        if (!shouldProceed) shouldContinue = false;
      });
    });
  });
});
