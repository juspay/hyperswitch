import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Session Call flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Session token retrieval flow", () => {
    it("session-call-flow", () => {
      let shouldContinue = true;

      cy.step("Step 1: Create payment intent", () => {
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

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      cy.step("Step 2: Get session token", () => {
        if (!shouldContinue) {
          cy.log(
            "⏭️ Skipping step due to previous failure - should_continue_further returned false"
          );
          return;
        }

        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SessionToken"];

        cy.sessionTokenCall(fixtures.sessionTokenBody, data, globalState);
      });
    });
  });
});
