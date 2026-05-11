import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

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
      let shouldContinue = true;

      cy.step("create payment intent", () => {
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

      cy.step("payment methods call", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: payment methods call");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("confirm payment intent", () => {
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

      cy.step("handle redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: handle redirection");
          return;
        }
        const expected_redirection = fixtures.confirmBody["return_url"];
        cy.handleRedirection(globalState, expected_redirection);
      });
    });
  });
});
