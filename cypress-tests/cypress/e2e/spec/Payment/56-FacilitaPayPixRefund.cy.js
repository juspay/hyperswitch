import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import * as utils from "../../configs/Payment/Utils";
import getConnectorDetails, {
  CONNECTOR_LISTS,
  shouldIncludeConnector,
} from "../../configs/Payment/Utils";

let globalState;

describe("FacilitaPay Pix Refund flow tests", () => {
  before("seed global state", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);

        if (
          shouldIncludeConnector(
            globalState.get("connectorId"),
            CONNECTOR_LISTS.INCLUDE.FACILITAPAY_PIX_REFUND
          )
        ) {
          skip = true;
          return;
        }
      })
      .then(() => {
        if (skip) {
          this.skip();
        }
      });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("FacilitaPay Pix Full Refund flow test", () => {
    it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> Retrieve Payment after Confirmation -> Refund Payment -> Sync Refund Payment -> List Refunds -> Retrieve Payment after Refund", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_transfer_pm"
        ]["PaymentIntent"]("Pix");
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

      cy.step("Confirm Payment Intent", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment Intent");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "bank_transfer_pm"
        ]["PixForRefund"];
        if (!confirmData) {
          shouldContinue = false;
          return;
        }
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

      cy.step("Retrieve Payment after Confirmation", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Retrieve Payment after Confirmation"
          );
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "bank_transfer_pm"
        ]["PixForRefund"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Refund Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Refund Payment");
          return;
        }
        const refundData = getConnectorDetails(globalState.get("connectorId"))[
          "bank_transfer_pm"
        ]["Refund"];
        if (!refundData) {
          shouldContinue = false;
          return;
        }
        const newRefundData = {
          ...refundData,
          Response: refundData.ResponseCustom || refundData.Response,
        };
        cy.refundCallTest(fixtures.refundBody, newRefundData, globalState);
        if (!utils.should_continue_further(refundData)) {
          shouldContinue = false;
        }
      });

      cy.step("Sync Refund Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Sync Refund Payment");
          return;
        }
        const syncRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["bank_transfer_pm"]["SyncRefund"];
        if (!syncRefundData) {
          shouldContinue = false;
          return;
        }
        cy.syncRefundCallTest(syncRefundData, globalState);
        if (!utils.should_continue_further(syncRefundData)) {
          shouldContinue = false;
        }
      });

      cy.step("List Refunds", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Refunds");
          return;
        }
        cy.listRefundCallTest(fixtures.listRefundCall, globalState);
      });

      cy.step("Retrieve Payment after Refund", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment after Refund");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "bank_transfer_pm"
        ]["PixForRefund"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
      });
    });
  });
});
