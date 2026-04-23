import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Card - Connector Testing Data flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Card-NoThreeDS payment with connector testing data", () => {
    it("Create Payment Intent with connector testing data -> Payment Methods Call -> Confirm Payment with connector testing data -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent with connector testing data", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ConnectorTestingData"];

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

      cy.step("Payment Methods Call", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Payment Methods Call");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment with connector testing data", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Confirm Payment with connector testing data"
          );
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ConnectorTestingDataConfirm"];

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
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ConnectorTestingDataConfirm"];

        cy.retrievePaymentCallTest({ globalState, data: confirmData });
      });
    });
  });
});
