import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, {
  shouldIncludeConnector,
  CONNECTOR_LISTS,
  should_continue_further,
} from "../../configs/Payment/Utils";

let globalState;

describe("Inespay - Refund flow", () => {
  const connector = Cypress.env("CONNECTOR") || "inespay";

  before("seed global state", function () {
    // Inclusion gate: Only run for Inespay connector
    if (
      shouldIncludeConnector(
        connector,
        CONNECTOR_LISTS.INCLUDE.INESPAY_REFUND || []
      )
    ) {
      this.skip();
      return;
    }
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Inespay - Full Refund flow for auto-captured SEPA payment", () => {
    it("should create payment intent, confirm payment, and process full refund with sync", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_transfer_pm"
        ]["PaymentIntent"];
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );
        if (!should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Confirm Payment Intent", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment Intent");
          return;
        }
        const confirmData = getConnectorDetails(
          globalState.get("connectorId")
        )["bank_transfer_pm"]["No3DSAutoCapture"];
        cy.confirmCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment after Confirmation", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const confirmData = getConnectorDetails(
          globalState.get("connectorId")
        )["bank_transfer_pm"]["No3DSAutoCapture"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
        if (!should_continue_further(confirmData)) {
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
        cy.refundCallTest(fixtures.refundBody, refundData, globalState);
        if (!should_continue_further(refundData)) {
          shouldContinue = false;
        }
      });

      cy.step("Sync Refund Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Sync Refund");
          return;
        }
        const syncRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["bank_transfer_pm"]["SyncRefund"];
        cy.syncRefundCallTest(syncRefundData, globalState);
      });
    });
  });

  context(
    "Inespay - Partial Refund flow for auto-captured SEPA payment",
    () => {
      it("should create payment intent, confirm payment, and process partial refund with sync", () => {
        let shouldContinue = true;

        cy.step("Create Payment Intent", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "bank_transfer_pm"
          ]["PaymentIntent"];
          cy.createPaymentIntentTest(
            fixtures.createPaymentBody,
            data,
            "no_three_ds",
            "automatic",
            globalState
          );
          if (!should_continue_further(data)) {
            shouldContinue = false;
          }
        });

      cy.step("Confirm Payment Intent", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment Intent");
          return;
        }
        const confirmData = getConnectorDetails(
          globalState.get("connectorId")
        )["bank_transfer_pm"]["No3DSAutoCapture"];
        cy.confirmCallTest(
            fixtures.confirmBody,
            confirmData,
            true,
            globalState
          );
          if (!should_continue_further(confirmData)) {
            shouldContinue = false;
          }
        });

        cy.step("Retrieve Payment after Confirmation", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Retrieve Payment");
            return;
          }
          const confirmData = getConnectorDetails(
            globalState.get("connectorId")
          )["bank_transfer_pm"]["No3DSAutoCapture"];
          cy.retrievePaymentCallTest({ globalState, data: confirmData });
          if (!should_continue_further(confirmData)) {
            shouldContinue = false;
          }
        });

        cy.step("Partial Refund Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Partial Refund");
            return;
          }
          const refundData = getConnectorDetails(
            globalState.get("connectorId")
          )["bank_transfer_pm"]["PartialRefund"];
          cy.refundCallTest(fixtures.refundBody, refundData, globalState);
          if (!should_continue_further(refundData)) {
            shouldContinue = false;
          }
        });

        cy.step("Sync Refund Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Sync Refund");
            return;
          }
          const syncRefundData = getConnectorDetails(
            globalState.get("connectorId")
          )["bank_transfer_pm"]["SyncRefund"];
          cy.syncRefundCallTest(syncRefundData, globalState);
        });
      });
    }
  );

  context("Inespay - Sync Refund status retrieval", () => {
    it("should retrieve refund status via GET /refunds/{id} endpoint", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_transfer_pm"
        ]["PaymentIntent"];
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );
        if (!should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Confirm Payment Intent", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment Intent");
          return;
        }
        const confirmData = getConnectorDetails(
          globalState.get("connectorId")
        )["bank_transfer_pm"]["No3DSAutoCapture"];
        cy.confirmCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!should_continue_further(confirmData)) {
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
        cy.refundCallTest(fixtures.refundBody, refundData, globalState);
        if (!should_continue_further(refundData)) {
          shouldContinue = false;
        }
      });

      cy.step("Sync Refund Status", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Sync Refund Status");
          return;
        }
        const syncRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["bank_transfer_pm"]["SyncRefund"];
        cy.syncRefundCallTest(syncRefundData, globalState);
      });
    });
  });
});
