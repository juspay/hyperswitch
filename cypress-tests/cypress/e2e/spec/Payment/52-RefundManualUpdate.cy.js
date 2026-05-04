import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Refund Manual Update Tests", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
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
          cy.task("cli_log", "Skipping step: Manual Update Refund Status to Failed");
          return;
        }
        const merchantId = globalState.get("merchantId");
        const refundManualUpdateRequestBody = {
          merchant_id: merchantId,
          status: "failed",
        };
        const manualUpdateData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["ManualRefundUpdate"];
        cy.manualRefundStatusUpdateTest(
          globalState,
          refundManualUpdateRequestBody
        );
        if (!utils.should_continue_further(manualUpdateData)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Refund to Verify Status Update", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Refund to Verify Status Update");
          return;
        }
        const syncRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["SyncRefund"];
        cy.syncRefundCallTest(syncRefundData, globalState);
      });
    });
  });

  context("Manual Update - Error Code and Error Message with SetOrUnset", () => {
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

      cy.step("Manual Update Error Code and Error Message", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Manual Update Error Code and Error Message");
          return;
        }
        const merchantId = globalState.get("merchantId");
        const refundManualUpdateRequestBody = {
          merchant_id: merchantId,
          status: "failed",
          error_code: {
            set: "TEST_ERROR_CODE",
          },
          error_message: {
            set: "Test error message for manual update",
          },
        };
        const manualUpdateData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["ManualRefundUpdate"];
        cy.manualRefundStatusUpdateTest(
          globalState,
          refundManualUpdateRequestBody
        );
        if (!utils.should_continue_further(manualUpdateData)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Refund to Verify Error Code and Message", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Refund to Verify Error Code and Message");
          return;
        }
        const syncRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["SyncRefund"];
        cy.syncRefundCallTest(syncRefundData, globalState);
      });
    });
  });

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
          cy.task("cli_log", "Skipping step: Manual Update Partial Refund Status to Failed");
          return;
        }
        const merchantId = globalState.get("merchantId");
        const refundManualUpdateRequestBody = {
          merchant_id: merchantId,
          status: "failed",
          error_code: {
            set: "PARTIAL_REFUND_FAILED",
          },
          error_message: {
            set: "Partial refund failed via manual update",
          },
        };
        const manualUpdateData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["ManualRefundUpdate"];
        cy.manualRefundStatusUpdateTest(
          globalState,
          refundManualUpdateRequestBody
        );
        if (!utils.should_continue_further(manualUpdateData)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Partial Refund to Verify Status Update", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Partial Refund to Verify Status Update");
          return;
        }
        const syncRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["SyncRefund"];
        cy.syncRefundCallTest(syncRefundData, globalState);
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
          cy.task("cli_log", "Skipping step: Manual Update 1 - Set Error Code and Message");
          return;
        }
        const merchantId = globalState.get("merchantId");
        const refundManualUpdateRequestBody = {
          merchant_id: merchantId,
          status: "failed",
          error_code: {
            set: "IDEMPOTENCY_TEST",
          },
          error_message: {
            set: "First manual update for idempotency test",
          },
        };
        const manualUpdateData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["ManualRefundUpdate"];
        cy.manualRefundStatusUpdateTest(
          globalState,
          refundManualUpdateRequestBody
        );
        if (!utils.should_continue_further(manualUpdateData)) {
          shouldContinue = false;
        }
      });

      cy.step("Manual Update 2 - Same Data (Idempotency Check)", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Manual Update 2 - Same Data (Idempotency Check)");
          return;
        }
        const merchantId = globalState.get("merchantId");
        const refundManualUpdateRequestBody = {
          merchant_id: merchantId,
          status: "failed",
          error_code: {
            set: "IDEMPOTENCY_TEST",
          },
          error_message: {
            set: "First manual update for idempotency test",
          },
        };
        const manualUpdateData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["ManualRefundUpdate"];
        cy.manualRefundStatusUpdateTest(
          globalState,
          refundManualUpdateRequestBody
        );
        if (!utils.should_continue_further(manualUpdateData)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Refund to Verify Idempotency", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Refund to Verify Idempotency");
          return;
        }
        const syncRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["SyncRefund"];
        cy.syncRefundCallTest(syncRefundData, globalState);
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
          cy.task("cli_log", "Skipping step: Manual Update with Error Code and Message");
          return;
        }
        const merchantId = globalState.get("merchantId");
        const refundManualUpdateRequestBody = {
          merchant_id: merchantId,
          status: "failed",
          error_code: {
            set: "UNSET_TEST",
          },
          error_message: {
            set: "This error will be unset",
          },
        };
        const manualUpdateData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["ManualRefundUpdate"];
        cy.manualRefundStatusUpdateTest(
          globalState,
          refundManualUpdateRequestBody
        );
        if (!utils.should_continue_further(manualUpdateData)) {
          shouldContinue = false;
        }
      });

      cy.step("Unset Error Code and Error Message", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Unset Error Code and Error Message");
          return;
        }
        const merchantId = globalState.get("merchantId");
        const refundManualUpdateRequestBody = {
          merchant_id: merchantId,
          error_code: {
            unset: null,
          },
          error_message: {
            unset: null,
          },
        };
        const manualUpdateData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["ManualRefundUpdate"];
        cy.manualRefundStatusUpdateTest(
          globalState,
          refundManualUpdateRequestBody
        );
        if (!utils.should_continue_further(manualUpdateData)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Refund to Verify Unset Fields", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Refund to Verify Unset Fields");
          return;
        }
        const syncRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["SyncRefund"];
        cy.syncRefundCallTest(syncRefundData, globalState);
      });
    });
  });
});
