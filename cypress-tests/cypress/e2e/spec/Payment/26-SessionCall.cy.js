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

  context(
    "Create Payment Intent and Session Token flow test",
    () => {
      it("create-payment-call-test -> session-call-test", () => {
        let shouldContinue = true;

        cy.step("create-payment-call-test", () => {
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

        cy.step("session-call-test", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: session-call-test");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["SessionToken"];

          cy.sessionTokenCall(fixtures.sessionTokenBody, data, globalState);
        });
      });
    }
  );
});
