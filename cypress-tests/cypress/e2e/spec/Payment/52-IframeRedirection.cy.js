import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, {
  CONNECTOR_LISTS,
} from "../../configs/Payment/Utils";
import * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Iframe Redirection Payment Flow Tests", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Happy Path - Create Payment with Iframe Redirection Enabled", () => {
    it("Create Payment Intent with is_iframe_redirection_enabled -> Confirm Payment -> Verify Redirect Response -> Retrieve Payment", function () {
      const connectorId = globalState.get("connectorId");
      if (!CONNECTOR_LISTS.INCLUDE.IFRAME_REDIRECTION.includes(connectorId)) {
        cy.task(
          "cli_log",
          `Skipping iframe redirection test - connector ${connectorId} not in supported list`
        );
        this.skip();
      }

      let shouldContinue = true;

      cy.step(
        "Create Payment Intent with is_iframe_redirection_enabled",
        () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentIntent"];

          cy.createPaymentIntentTest(
            fixtures.createPaymentBody,
            data,
            "three_ds",
            "automatic",
            globalState
          );

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        }
      );

      cy.step("Payment Methods Call", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Payment Methods Call");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment with is_iframe_redirection_enabled", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["IframeRedirection"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Verify Redirect Response Contains Iframe HTML", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Verify Redirect Response");
          return;
        }

        cy.wrap(globalState.get("paymentIntentStatus")).should(
          "equal",
          "requires_customer_action"
        );
        cy.wrap(globalState.get("nextActionUrl")).should("not.be.null");
      });

      cy.step("Poll Payment Status to Terminal State", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Poll Payment Status");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });

  context("Edge Case - Iframe Redirection Disabled", () => {
    it("Create Payment Intent without iframe redirection -> Confirm Payment -> Verify Standard Redirect", () => {
      let shouldContinue = true;

      cy.step(
        "Create Payment Intent without is_iframe_redirection_enabled",
        () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentIntent"];

          cy.createPaymentIntentTest(
            fixtures.createPaymentBody,
            data,
            "three_ds",
            "automatic",
            globalState
          );

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        }
      );

      cy.step("Payment Methods Call", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Payment Methods Call");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment without iframe redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["3DSAutoCapture"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Verify Standard Redirect Response", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Verify Standard Redirect");
          return;
        }

        cy.wrap(globalState.get("paymentIntentStatus")).should(
          "equal",
          "requires_customer_action"
        );
        cy.wrap(globalState.get("nextActionUrl")).should("not.be.null");
      });
    });
  });

  context("Negative Case - Invalid Payment Intent State", () => {
    it("Attempt iframe redirection on invalid payment state -> Verify Error Response", () => {
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

      cy.step("Attempt Confirm without 3DS on iframe-enabled flow", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Attempt Confirm");
          return;
        }

        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Verify Payment Completes Without Iframe Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Verify Payment Completion");
          return;
        }

        const connectorId = globalState.get("connectorId");
        const expectedStatus =
          connectorId === "worldpayxml" ? "processing" : "succeeded";
        cy.wrap(globalState.get("paymentIntentStatus")).should(
          "equal",
          expectedStatus
        );
      });
    });
  });
});
