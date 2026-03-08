import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";
import reportErrors from "../../../utils/reportErrors";

let globalState;

describe("Card - ThreeDS payment flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Card-ThreeDS payment flow test Create and Confirm", () => {
    it("create payment intent -> payment methods call -> confirm payment intent -> handle redirection", () => {
      const errorStack = [];
      let shouldContinue = true;

      cy.stepTest("create payment intent", errorStack, () => {
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
      });

      cy.stepTest("payment methods call", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: payment methods call");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.stepTest("confirm payment intent", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: confirm payment intent");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["3DSAutoCapture"];

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

      cy.stepTest("handle redirection", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: handle redirection");
          return;
        }
        const expected_redirection = fixtures.confirmBody["return_url"];
        cy.handleRedirection(globalState, expected_redirection);
      });

      cy.then(() => {
        if (errorStack.length > 0) {
          reportErrors(errorStack);
        }
      });
    });
  });
});
