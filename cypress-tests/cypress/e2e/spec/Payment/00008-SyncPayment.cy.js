import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";
import step from "../../../utils/customStep";

let globalState;

describe("Card - Sync payment flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Card - Sync payment flow test", () => {
    it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> Retrieve Payment after Confirmation", () => {
      let shouldContinue = true;

      step("Create Payment Intent", shouldContinue, () => {
        const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentIntent"];

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

      step("Payment Methods Call", shouldContinue, () => {
        cy.paymentMethodsCallTest(globalState);
      });

      step("Confirm Payment Intent", shouldContinue, () => {
        const confirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["No3DSAutoCapture"];

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

      step("Retrieve Payment after Confirmation", shouldContinue, () => {
        const confirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["No3DSAutoCapture"];

        cy.retrievePaymentCallTest({ globalState, data: confirmData });
      });
    });
  });
});