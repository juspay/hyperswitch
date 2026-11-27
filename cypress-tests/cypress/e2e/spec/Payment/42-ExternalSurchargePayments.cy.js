import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import {
  captureFullAmount,
  capturePartialAmount,
  cardData,
  isSurchargeServiceAvailable,
  retrieveAfterCapture,
  runSurchargePayment,
  shouldSkipExternalSurcharge,
} from "../../../utils/externalSurcharge";

let globalState;

// Needs the surcharge connector that 03-ConnectorCreate creates from the
// "interpayments" entry in creds.json, and a router that can reach the
// surcharge service; the spec skips itself without either.
describe("External surcharge - payments", () => {
  before("seed global state", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        return shouldSkipExternalSurcharge(globalState);
      })
      .then((shouldSkip) => {
        skip = shouldSkip;
        if (!skip) {
          return isSurchargeServiceAvailable(globalState);
        }
      })
      .then((available) => {
        if (!skip && !available) {
          cy.task(
            "cli_log",
            "The surcharge service is not reachable in this environment, skipping the external surcharge spec"
          );
          skip = true;
        }
      })
      .then(() => {
        if (skip) {
          this.skip();
        }
      });
  });

  after("unlink the surcharge connector and flush global state", () => {
    cy.unlinkSurchargeConnectorFromProfile(globalState);
    cy.task("setGlobalState", globalState.data);
  });

  it("Point the profile at the surcharge connector, create a customer", () => {
    cy.linkSurchargeConnectorToProfile(globalState);
    cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
  });

  context("Automatic capture", () => {
    it("Create Payment Intent -> Eligibility -> Confirm -> Retrieve Payment", () => {
      const state = runSurchargePayment(globalState);

      cy.step("Retrieve Payment (the surcharge is persisted)", () => {
        if (!state.shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        cy.retrievePaymentCallTest({
          globalState,
          data: cardData(globalState, "ExternalSurchargeRetrieve"),
        });
      });
    });
  });

  context("Manual capture", () => {
    it("Create Payment Intent -> Eligibility -> Confirm -> Capture -> Retrieve Payment", () => {
      const state = runSurchargePayment(globalState, {
        captureMethod: "manual",
      });

      cy.step("Capture the full charged amount", () => {
        if (!state.shouldContinue) {
          cy.task("cli_log", "Skipping step: Capture");
          return;
        }
        captureFullAmount(globalState);
      });

      cy.step(
        "Retrieve Payment (the surcharge is persisted and all of it was received)",
        () => {
          if (!state.shouldContinue) {
            cy.task("cli_log", "Skipping step: Retrieve Payment");
            return;
          }
          retrieveAfterCapture(globalState, {
            captureFlow: "Capture",
            expectedReceived: "net_amount",
          });
        }
      );
    });

    it("Create Payment Intent -> Eligibility -> Confirm -> Partial Capture -> Retrieve Payment", () => {
      const state = runSurchargePayment(globalState, {
        captureMethod: "manual",
      });

      cy.step("Capture only part of the order amount", () => {
        if (!state.shouldContinue) {
          cy.task("cli_log", "Skipping step: Partial Capture");
          return;
        }
        capturePartialAmount(globalState);
      });

      cy.step(
        "Retrieve Payment (only the captured amount was received)",
        () => {
          if (!state.shouldContinue) {
            cy.task("cli_log", "Skipping step: Retrieve Payment");
            return;
          }
          retrieveAfterCapture(globalState, {
            captureFlow: "PartialCapture",
            expectedReceived: cardData(globalState, "PartialCapture").Request
              .amount_to_capture,
          });
        }
      );
    });
  });
});
