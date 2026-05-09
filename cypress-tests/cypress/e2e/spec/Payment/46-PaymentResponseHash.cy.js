import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Payment Response Hash - Business Profile Configuration", () => {
  let shouldContinue = true;

  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      const connectorId = globalState.get("connectorId");
      if (
        utils.CONNECTOR_LISTS.EXCLUDE.PAYMENT_RESPONSE_HASH.includes(
          connectorId
        )
      ) {
        shouldContinue = false;
      }
    });
  });

  beforeEach(function () {
    if (!shouldContinue) {
      this.skip();
    }
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Verify Payment Response Hash is Enabled by Default", () => {
    it("Verify merchant account has payment response hash enabled", () => {
      cy.verifyPaymentResponseHash(globalState);
    });
  });

  context("Card Payment with Response Hash", () => {
    it("Create Payment Intent -> Confirm Payment -> Verify Hash Config", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Confirm Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentResponseHash"];

        cy.confirmCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );

        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        cy.retrievePaymentCallTest({ globalState });
      });

      cy.step("Verify Payment Response Hash Configuration", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Verify Payment Response Hash");
          return;
        }
        cy.verifyPaymentResponseHash(globalState);
      });
    });
  });

  context("Card Create+Confirm with Response Hash", () => {
    it("Create and Confirm Payment -> Verify Hash Config", () => {
      let shouldContinue = true;

      cy.step("Create and Confirm Payment", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentResponseHash"];

        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        cy.retrievePaymentCallTest({ globalState });
      });

      cy.step("Verify Payment Response Hash Configuration", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Verify Payment Response Hash");
          return;
        }
        cy.verifyPaymentResponseHash(globalState);
      });
    });
  });
});
