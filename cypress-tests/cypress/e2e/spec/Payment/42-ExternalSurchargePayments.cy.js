import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import {
  captureFullAmount,
  cardData,
  runSurchargePayment,
  shouldSkipExternalSurcharge,
} from "../../../utils/externalSurcharge";

let globalState;

// Needs the surcharge connector that 03-ConnectorCreate creates from the
// "interpayments" entry in creds.json; the spec skips itself without it.
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
});
