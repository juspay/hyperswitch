import State from "../../../utils/State";
import * as utils from "../../configs/Payout/Utils";

let globalState;
let payoutBody;

describe("[Payout] Recurring", () => {
  let shouldContinue = true;

  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);

      // Synchronously set shouldContinue after globalState is available
      // This ensures the flag is set before any beforeEach checks run
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
      cy.getPayoutRecurringData(globalState, "RecurringTrue").then(
        ({ payoutBody: pb, data, shouldContinue: shouldProceed }) => {
          payoutBody = pb;
          if (!shouldProceed) {
            shouldContinue = false;
            return;
          }

          cy.createConfirmPayoutTest(
            payoutBody,
            data,
            true, // confirm=true — immediately confirm the payout
            false, // auto_fulfill=false — do not auto-fulfill; payout stays in requires_fulfillment state
            globalState
          ).then((response) => {
            // recurring=true because we explicitly set recurring:true in the request to mark this as a recurring payout.
            // The API saves the payout method after success and echoes recurring:true.
            // (payout_method_id not asserted here because Wise does not return it in the create response — see RecurringUseMethod TRIGGER_SKIP.)
            cy.verifyRecurringPayoutResponse(
              response,
              true,
              undefined,
              "requires_fulfillment"
            );
          });

          if (shouldContinue)
            shouldContinue = utils.should_continue_further(data);
        }
      );
    });

    it("fulfill-recurring-payout-test", () => {
      cy.getPayoutRecurringData(globalState, "RecurringTrueFulfill").then(
        ({ payoutBody: pb, data, shouldContinue: shouldProceed }) => {
          payoutBody = pb;
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
      cy.getPayoutRecurringData(globalState, "RecurringUseMethod").then(
        ({ payoutBody: pb, data, shouldContinue: shouldProceed }) => {
          payoutBody = pb;
          if (!shouldProceed) {
            shouldContinue = false;
            return;
          }

          // Use the payout_method_id saved from the create-payout-with-recurring-true test
          cy.injectPayoutMethodId(data, globalState);

          cy.createConfirmPayoutTest(
            payoutBody,
            data,
            true, // confirm=true — immediately confirm the payout
            false, // auto_fulfill=false — do not auto-fulfill; payout stays in requires_fulfillment state
            globalState
          ).then((response) => {
            // recurring=true and payout_method_id matches the saved method from the RecurringTrue test
            // — this verifies the saved payout method can be reused for subsequent recurring payouts.
            cy.verifyRecurringPayoutResponse(
              response,
              true,
              globalState.get("payoutMethodId"),
              "requires_fulfillment"
            );
          });

          if (shouldContinue)
            shouldContinue = utils.should_continue_further(data);
        }
      );
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
      cy.getPayoutRecurringData(globalState, "RecurringFalse").then(
        ({ payoutBody: pb, data, shouldContinue: shouldProceed }) => {
          payoutBody = pb;
          if (!shouldProceed) {
            shouldContinue = false;
            return;
          }

          cy.createConfirmPayoutTest(
            payoutBody,
            data,
            true, // confirm=true — immediately confirm the payout
            false, // auto_fulfill=false — do not auto-fulfill; payout stays in requires_fulfillment state
            globalState
          ).then((response) => {
            // recurring=false because we explicitly set recurring:false in the request — this is a one-time payout.
            // No payout method is saved for future reuse. The API echoes recurring:false.
            cy.verifyRecurringPayoutResponse(
              response,
              false,
              undefined,
              "requires_fulfillment"
            );
          });

          if (shouldContinue)
            shouldContinue = utils.should_continue_further(data);
        }
      );
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
      cy.getPayoutRecurringData(globalState, "RecurringDefault").then(
        ({ payoutBody: pb, data, shouldContinue: shouldProceed }) => {
          payoutBody = pb;
          if (!shouldProceed) {
            shouldContinue = false;
            return;
          }

          cy.createConfirmPayoutTest(
            payoutBody,
            data,
            true, // confirm=true — immediately confirm the payout (POST /payouts/create with confirm:true in body)
            false, // auto_fulfill=false — do not auto-fulfill; payout stays in requires_fulfillment state
            globalState
          ).then((response) => {
            // recurring defaults to false when the field is omitted (see crates/router/src/core/payouts.rs:3142: recurring: req.recurring.unwrap_or(false)).
            // The API echoes recurring:false, confirming the default.
            cy.verifyRecurringPayoutResponse(
              response,
              false,
              undefined,
              "requires_fulfillment"
            );
          });

          if (shouldContinue)
            shouldContinue = utils.should_continue_further(data);
        }
      );
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
      cy.getPayoutRecurringData(globalState, "RecurringInvalidConfirm").then(
        ({ payoutBody: pb, data, shouldContinue: shouldProceed }) => {
          payoutBody = pb;
          if (!shouldProceed) {
            shouldContinue = false;
            return;
          }

          // Inject real payout_method_id from RecurringTrue to pass deserialization
          // so the confirm=false validation runs and returns the expected error
          cy.injectPayoutMethodId(data, globalState);

          // This test validates that using payout_method_id with confirm=false returns error
          // Error assertion (IR_06: Confirm must be true for recurring payouts) is handled by
          // createConfirmPayoutTest -> defaultErrorHandler using RecurringInvalidConfirm.Response.body.error config.
          cy.createConfirmPayoutTest(
            payoutBody,
            data,
            false,
            false,
            globalState
          );

          // For error responses, we expect should_continue_further to return false
          // but the test itself should pass (asserting the error)
          if (shouldContinue)
            shouldContinue = utils.should_continue_further(data);
        }
      );
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
      cy.getPayoutRecurringData(globalState, "EntityTypeCompany").then(
        ({ payoutBody: pb, data, shouldContinue: shouldProceed }) => {
          payoutBody = pb;
          if (!shouldProceed) {
            shouldContinue = false;
            return;
          }

          cy.createConfirmPayoutTest(
            payoutBody,
            data,
            true,
            false,
            globalState
          );

          if (shouldContinue)
            shouldContinue = utils.should_continue_further(data);
        }
      );
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
      cy.getPayoutRecurringData(globalState, "EntityTypeIndividual").then(
        ({ payoutBody: pb, data, shouldContinue: shouldProceed }) => {
          payoutBody = pb;
          if (!shouldProceed) {
            shouldContinue = false;
            return;
          }

          cy.createConfirmPayoutTest(
            payoutBody,
            data,
            true,
            false,
            globalState
          );

          if (shouldContinue)
            shouldContinue = utils.should_continue_further(data);
        }
      );
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
      cy.getPayoutRecurringData(globalState, "EntityTypeNaturalPerson").then(
        ({ payoutBody: pb, data, shouldContinue: shouldProceed }) => {
          payoutBody = pb;
          if (!shouldProceed) {
            shouldContinue = false;
            return;
          }

          cy.createConfirmPayoutTest(
            payoutBody,
            data,
            true,
            false,
            globalState
          );

          if (shouldContinue)
            shouldContinue = utils.should_continue_further(data);
        }
      );
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
      cy.getPayoutRecurringData(globalState, "EntityTypeNonProfit").then(
        ({ payoutBody: pb, data, shouldContinue: shouldProceed }) => {
          payoutBody = pb;
          if (!shouldProceed) {
            shouldContinue = false;
            return;
          }

          cy.createConfirmPayoutTest(
            payoutBody,
            data,
            true,
            false,
            globalState
          );

          if (shouldContinue)
            shouldContinue = utils.should_continue_further(data);
        }
      );
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
      cy.getPayoutRecurringData(globalState, "EntityTypePersonal").then(
        ({ payoutBody: pb, data, shouldContinue: shouldProceed }) => {
          payoutBody = pb;
          if (!shouldProceed) {
            shouldContinue = false;
            return;
          }

          cy.createConfirmPayoutTest(
            payoutBody,
            data,
            true,
            false,
            globalState
          );

          if (shouldContinue)
            shouldContinue = utils.should_continue_further(data);
        }
      );
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
      cy.getPayoutRecurringData(globalState, "EntityTypePublicSector").then(
        ({ payoutBody: pb, data, shouldContinue: shouldProceed }) => {
          payoutBody = pb;
          if (!shouldProceed) {
            shouldContinue = false;
            return;
          }

          cy.createConfirmPayoutTest(
            payoutBody,
            data,
            true,
            false,
            globalState
          );

          if (shouldContinue)
            shouldContinue = utils.should_continue_further(data);
        }
      );
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
        cy.getPayoutRecurringData(globalState, "EntityTypeDefault").then(
          ({ payoutBody: pb, data, shouldContinue: shouldProceed }) => {
            payoutBody = pb;
            if (!shouldProceed) {
              shouldContinue = false;
              return;
            }

            cy.createConfirmPayoutTest(
              payoutBody,
              data,
              true,
              false,
              globalState
            );

            if (shouldContinue)
              shouldContinue = utils.should_continue_further(data);
          }
        );
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
      cy.getPayoutRecurringData(globalState, "EntityTypeInvalid").then(
        ({ payoutBody: pb, data, shouldContinue: shouldProceed }) => {
          payoutBody = pb;
          if (!shouldProceed) {
            shouldContinue = false;
            return;
          }

          // This test validates that invalid entity_type returns 400 error
          // confirm=false — intentionally do NOT confirm; the invalid entity_type triggers a 400 before confirmation
          cy.createConfirmPayoutTest(
            payoutBody,
            data,
            false,
            false,
            globalState
          );

          // For error responses, we expect should_continue_further to return false
          if (shouldContinue)
            shouldContinue = utils.should_continue_further(data);
        }
      );
    });
  });
});
