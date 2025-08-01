import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Card - Moto payment flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Card-Moto payment flow test Create+Confirm", () => {
    let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create+confirm-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["MotoPaymentIntent"];

      cy.createConfirmPaymentTest(
        fixtures.createConfirmPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["MotoPaymentIntent"];

      cy.retrievePaymentCallTest(globalState, data);
    });
  });
});
