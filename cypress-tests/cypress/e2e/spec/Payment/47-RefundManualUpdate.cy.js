import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Refund Manual Update Tests", () => {
  before(function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        const connector = globalState.get("connectorId");

        if (
          utils.shouldIncludeConnector(
            connector,
            utils.CONNECTOR_LISTS.INCLUDE.REFUND_MANUAL_UPDATE
          )
        ) {
          skip = true;
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

  context("Manual Update - Status Change (pending -> failed)", () => {
    it("Create Payment Intent -> Confirm Payment -> Create Refund -> Manual Update Status -> Retrieve Refund to Verify", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
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
          "card_pm"
        ]["No3DSAutoCapture"];
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

      cy.step("Create Refund", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Create Refund");
          return;
        }
        const refundData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Refund"];
        cy.refundCallTest(fixtures.refundBody, refundData, globalState);
        if (!utils.should_continue_further(refundData)) {
          shouldContinue = false;
        }
      });

      cy.step("Manual Update Refund Status to Failed", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Manual Update Refund Status to Failed"
          );
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ManualRefundUpdate"];
        cy.manualRefundStatusUpdateTest(data, globalState);
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Refund to Verify Status Update", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Retrieve Refund to Verify Status Update"
          );
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SyncRefundManualUpdateFailed"];
        cy.syncRefundCallTest(data, globalState);
      });
    });
  });

  context(
    "Manual Update - Error Code and Error Message with SetOrUnset",
    () => {
      it("Create Payment Intent -> Confirm Payment -> Create Refund -> Manual Update Error Code/Message -> Retrieve Refund to Verify", () => {
        let shouldContinue = true;

        cy.step("Create Payment Intent", () => {
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
          const confirmData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["No3DSAutoCapture"];
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

        cy.step("Create Refund", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Create Refund");
            return;
          }
          const refundData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["Refund"];
          cy.refundCallTest(fixtures.refundBody, refundData, globalState);
          if (!utils.should_continue_further(refundData)) {
            shouldContinue = false;
          }
        });

        cy.step("Manual Update Error Code and Error Message", () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Manual Update Error Code and Error Message"
            );
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["ManualRefundUpdateErrorCode"];
          cy.manualRefundStatusUpdateTest(data, globalState);
          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Retrieve Refund to Verify Error Code and Message", () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Retrieve Refund to Verify Error Code and Message"
            );
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["SyncRefundManualUpdateErrorCode"];
          cy.syncRefundCallTest(data, globalState);
        });
      });
    }
  );

  context("Manual Update - Partial Refund Status Update", () => {
    it("Create Payment Intent -> Confirm Payment -> Create Partial Refund -> Manual Update Status -> Retrieve Refund to Verify", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
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
          "card_pm"
        ]["No3DSAutoCapture"];
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

      cy.step("Create Partial Refund", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Create Partial Refund");
          return;
        }
        const partialRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["PartialRefund"];
        cy.refundCallTest(fixtures.refundBody, partialRefundData, globalState);
        if (!utils.should_continue_further(partialRefundData)) {
          shouldContinue = false;
        }
      });

      cy.step("Manual Update Partial Refund Status to Failed", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Manual Update Partial Refund Status to Failed"
          );
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ManualRefundUpdatePartialRefund"];
        cy.manualRefundStatusUpdateTest(data, globalState);
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Partial Refund to Verify Status Update", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Retrieve Partial Refund to Verify Status Update"
          );
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SyncRefundManualUpdatePartialRefund"];
        cy.syncRefundCallTest(data, globalState);
      });
    });
  });

  context("Manual Update - Idempotency Test (Multiple Updates)", () => {
    it("Create Payment Intent -> Confirm Payment -> Create Refund -> Manual Update 1 -> Manual Update 2 (Same Data) -> Retrieve to Verify Idempotency", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
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
          "card_pm"
        ]["No3DSAutoCapture"];
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

      cy.step("Create Refund", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Create Refund");
          return;
        }
        const refundData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Refund"];
        cy.refundCallTest(fixtures.refundBody, refundData, globalState);
        if (!utils.should_continue_further(refundData)) {
          shouldContinue = false;
        }
      });

      cy.step("Manual Update 1 - Set Error Code and Message", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Manual Update 1 - Set Error Code and Message"
          );
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ManualRefundUpdateIdempotency"];
        cy.manualRefundStatusUpdateTest(data, globalState);
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Manual Update 2 - Same Data (Idempotency Check)", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Manual Update 2 - Same Data (Idempotency Check)"
          );
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ManualRefundUpdateIdempotency"];
        cy.manualRefundStatusUpdateTest(data, globalState);
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Refund to Verify Idempotency", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Retrieve Refund to Verify Idempotency"
          );
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SyncRefundManualUpdateIdempotency"];
        cy.syncRefundCallTest(data, globalState);
      });
    });
  });

  context("Manual Update - Unset Error Fields", () => {
    it("Create Payment Intent -> Confirm Payment -> Create Refund -> Manual Update with Error -> Unset Error Fields -> Retrieve to Verify", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
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
          "card_pm"
        ]["No3DSAutoCapture"];
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

      cy.step("Create Refund", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Create Refund");
          return;
        }
        const refundData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Refund"];
        cy.refundCallTest(fixtures.refundBody, refundData, globalState);
        if (!utils.should_continue_further(refundData)) {
          shouldContinue = false;
        }
      });

      cy.step("Manual Update with Error Code and Message", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Manual Update with Error Code and Message"
          );
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ManualRefundUpdateErrorCode"];
        cy.manualRefundStatusUpdateTest(data, globalState);
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Unset Error Code and Error Message", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Unset Error Code and Error Message"
          );
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ManualRefundUpdateUnset"];
        cy.manualRefundStatusUpdateTest(data, globalState);
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Refund to Verify Unset Fields", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Retrieve Refund to Verify Unset Fields"
          );
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SyncRefundManualUpdateUnset"];
        cy.syncRefundCallTest(data, globalState);
      });
    });
  });
});
