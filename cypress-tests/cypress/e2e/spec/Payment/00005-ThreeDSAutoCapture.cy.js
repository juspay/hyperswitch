import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";
import step from "../../../utils/customStep";

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

<<<<<<< Updated upstream
      cy.step("create payment intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];
=======
      step("create payment intent", shouldContinue, () => {
        const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
          "PaymentIntent"
        ];
>>>>>>> Stashed changes

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

      step("payment methods call", shouldContinue, () => {
        cy.paymentMethodsCallTest(globalState);
      });

      step("confirm payment intent", shouldContinue, () => {
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

      step("handle redirection", shouldContinue, () => {
        const expected_redirection = fixtures.confirmBody["return_url"];
        cy.handleRedirection(globalState, expected_redirection);
      });
    });
  });
});
