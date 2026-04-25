import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Card - Surcharge payment flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Card payment with surcharge", () => {
    it("Create Payment Intent with surcharge -> Payment Methods Call -> Confirm Payment with surcharge -> Retrieve Payment with surcharge", () => {
      let shouldContinue = true;

      cy.step("Check if connector supports surcharge", () => {
        const connectorId = globalState.get("connectorId");
        if (
          !utils.CONNECTOR_LISTS.INCLUDE.SURCHARGE.includes(connectorId)
        ) {
          cy.task(
            "cli_log",
            `Skipping surcharge flow: connector ${connectorId} not in SURCHARGE list`
          );
          shouldContinue = false;
        }
      });

      cy.step("Create Payment Intent with surcharge", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Create Payment Intent with surcharge"
          );
          return;
        }

        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentWithSurcharge"];

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

      cy.step("Confirm Payment with surcharge", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Confirm Payment with surcharge"
          );
          return;
        }

        const confirmData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["ConfirmPaymentWithSurcharge"];

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

      cy.step("Retrieve Payment with surcharge", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Retrieve Payment with surcharge"
          );
          return;
        }

        const confirmData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["ConfirmPaymentWithSurcharge"];

        cy.retrievePaymentCallTest({ globalState, data: confirmData });
      });
    });
  });
});
