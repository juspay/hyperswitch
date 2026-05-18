import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Xendit - Split Payments and Refunds", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      // Skip if connector is not in SPLIT_PAYMENTS inclusion list
      if (
        !utils.CONNECTOR_LISTS.INCLUDE.SPLIT_PAYMENTS.includes(
          globalState.get("connectorId")
        )
      ) {
        this.skip();
      }
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Split Payment and Refund Flow", () => {
    it("Create Split Payment -> Create Split Refund -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Split Payment", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["XenditSplitPayment"];

        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Create Split Refund", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Create Split Refund");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["XenditSplitRefund"];

        cy.refundCallTest(fixtures.refundBody, data, globalState);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["XenditSplitPayment"];

        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });
});
