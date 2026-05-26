import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, {
  CONNECTOR_LISTS,
  shouldIncludeConnector,
} from "../../configs/Payment/Utils";
import * as utils from "../../configs/Payment/Utils";

let globalState;

describe("PayLater tests", () => {
  before("seed global state", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);

        if (
          shouldIncludeConnector(
            globalState.get("connectorId"),
            CONNECTOR_LISTS.INCLUDE.PAY_LATER
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

  context("Klarna PayLater - Auto Capture flow test", () => {
    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm Payment -> Handle PayLater Redirection -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["AutoCapture"];
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "three_ds",
          "automatic",
          globalState
        );
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("List Merchant Payment Methods", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Merchant Payment Methods");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["Klarna"];
        cy.confirmBankRedirectCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle PayLater Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle PayLater Redirection");
          return;
        }
        const expected_redirection =
          globalState.get("baseUrl") + "/payments/completion";
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handlePayLaterRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      });

      cy.step("Retrieve Payment after Redirection", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Retrieve Payment after Redirection"
          );
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["Klarna"];
        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });

  context("Klarna PayLater - Manual Capture flow test", () => {
    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm Payment -> Handle PayLater Redirection -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["ManualCapture"];
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "three_ds",
          "manual",
          globalState
        );
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("List Merchant Payment Methods", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Merchant Payment Methods");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["Klarna"];
        cy.confirmBankRedirectCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle PayLater Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle PayLater Redirection");
          return;
        }
        const expected_redirection =
          globalState.get("baseUrl") + "/payments/completion";
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handlePayLaterRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      });

      cy.step("Retrieve Payment after Redirection", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Retrieve Payment after Redirection"
          );
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["Klarna"];
        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });

  context("Affirm PayLater - Auto Capture flow test", () => {
    before("skip if connector does not support Affirm", function () {
      if (
        shouldIncludeConnector(
          globalState.get("connectorId"),
          CONNECTOR_LISTS.INCLUDE.AFFIRM
        )
      ) {
        this.skip();
      }
    });
    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm Payment -> Handle PayLater Redirection -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["AutoCapture"];
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "three_ds",
          "automatic",
          globalState
        );
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("List Merchant Payment Methods", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Merchant Payment Methods");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["Affirm"];
        cy.confirmBankRedirectCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle PayLater Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle PayLater Redirection");
          return;
        }
        const expected_redirection =
          globalState.get("baseUrl") + "/payments/completion";
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handlePayLaterRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      });

      cy.step("Retrieve Payment after Redirection", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Retrieve Payment after Redirection"
          );
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["Affirm"];
        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });

  context("Affirm PayLater - Manual Capture flow test", () => {
    before("skip if connector does not support Affirm", function () {
      if (
        shouldIncludeConnector(
          globalState.get("connectorId"),
          CONNECTOR_LISTS.INCLUDE.AFFIRM
        )
      ) {
        this.skip();
      }
    });
    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm Payment -> Handle PayLater Redirection -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["ManualCapture"];
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "three_ds",
          "manual",
          globalState
        );
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("List Merchant Payment Methods", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Merchant Payment Methods");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["Affirm"];
        cy.confirmBankRedirectCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle PayLater Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle PayLater Redirection");
          return;
        }
        const expected_redirection =
          globalState.get("baseUrl") + "/payments/completion";
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handlePayLaterRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      });

      cy.step("Retrieve Payment after Redirection", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Retrieve Payment after Redirection"
          );
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["Affirm"];
        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });

  context("Affirm - Full Payment and Refund Flow", () => {
    beforeEach(function () {
      if (
        shouldIncludeConnector(
          globalState.get("connectorId"),
          CONNECTOR_LISTS.INCLUDE.AFFIRM_PAY_LATER
        )
      ) {
        this.skip();
      }
    });

    it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> Handle Redirection -> Retrieve Payment -> Refund Payment -> Sync Refund", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["PaymentIntent"];
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "manual",
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
          "pay_later_pm"
        ]["AffirmManualCapture"];
        cy.confirmBankRedirectCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle Redirection");
          return;
        }
        const expected_redirection =
          globalState.get("baseUrl") + "/payments/completion";
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handleAffirmRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
        cy.task("cli_log", "Affirm redirection handler completed");
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
          "pay_later_pm"
        ]["AffirmManualCapture"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Capture Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Capture Payment");
          return;
        }
        const captureData = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["Capture"];
        cy.captureCallTest(fixtures.captureBody, captureData, globalState);
        if (!utils.should_continue_further(captureData)) {
          shouldContinue = false;
        }
      });

      cy.step("Refund Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Refund Payment");
          return;
        }
        const refundData = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["Refund"];
        cy.refundCallTest(fixtures.refundBody, refundData, globalState);
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
        )["pay_later_pm"]["SyncRefund"];
        cy.syncRefundCallTest(syncRefundData, globalState);
      });
    });
  });

  context("Affirm - Partial Refund Flow", () => {
    beforeEach(function () {
      if (
        shouldIncludeConnector(
          globalState.get("connectorId"),
          CONNECTOR_LISTS.INCLUDE.AFFIRM_PAY_LATER
        )
      ) {
        this.skip();
      }
    });

    it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> Handle Redirection -> Retrieve Payment -> Partial Refund -> Sync Refund", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["PaymentIntent"];
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "manual",
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
          "pay_later_pm"
        ]["AffirmManualCapture"];
        cy.confirmBankRedirectCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle Redirection");
          return;
        }
        const expected_redirection =
          globalState.get("baseUrl") + "/payments/completion";
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handleAffirmRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
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
          "pay_later_pm"
        ]["AffirmManualCapture"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Capture Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Capture Payment");
          return;
        }
        const captureData = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["Capture"];
        cy.captureCallTest(fixtures.captureBody, captureData, globalState);
        if (!utils.should_continue_further(captureData)) {
          shouldContinue = false;
        }
      });

      cy.step("Partial Refund Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Partial Refund Payment");
          return;
        }
        const partialRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["pay_later_pm"]["PartialRefund"];
        cy.refundCallTest(fixtures.refundBody, partialRefundData, globalState);
        if (!utils.should_continue_further(partialRefundData)) {
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
        )["pay_later_pm"]["SyncRefund"];
        cy.syncRefundCallTest(syncRefundData, globalState);
      });
    });
  });

  context(
    "Klarna PayLater - Manual Capture with Capture and Retrieve flow test",
    () => {
      it("Create Payment Intent -> List Merchant Payment Methods -> Confirm Payment -> Handle PayLater Redirection -> Capture Payment -> Retrieve Payment after Capture", () => {
        let shouldContinue = true;

        cy.step("Create Payment Intent", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "pay_later_pm"
          ]["ManualCapture"];
          cy.createPaymentIntentTest(
            fixtures.createPaymentBody,
            data,
            "three_ds",
            "manual",
            globalState
          );
          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("List Merchant Payment Methods", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: List Merchant Payment Methods");
            return;
          }
          cy.paymentMethodsCallTest(globalState);
        });

        cy.step("Confirm Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm Payment");
            return;
          }
          const confirmData = getConnectorDetails(
            globalState.get("connectorId")
          )["pay_later_pm"]["Klarna"];
          cy.confirmBankRedirectCallTest(
            fixtures.confirmBody,
            confirmData,
            true,
            globalState
          );
          if (!utils.should_continue_further(confirmData)) {
            shouldContinue = false;
          }
        });

        cy.step("Handle PayLater Redirection", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Handle PayLater Redirection");
            return;
          }
          const expected_redirection =
            globalState.get("baseUrl") + "/payments/completion";
          const payment_method_type = globalState.get("paymentMethodType");
          cy.handlePayLaterRedirection(
            globalState,
            payment_method_type,
            expected_redirection
          );
        });

        cy.step("Capture Payment on wrong status", () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Capture Payment on wrong status"
            );
            return;
          }
          const captureData = getConnectorDetails(
            globalState.get("connectorId")
          )["pay_later_pm"]["CaptureOnWrongStatus"];
          cy.captureCallTest(fixtures.captureBody, captureData, globalState);
        });

        cy.step("Retrieve Payment after failed Capture", () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Retrieve Payment after failed Capture"
            );
            return;
          }
          const klarnaData = getConnectorDetails(
            globalState.get("connectorId")
          )["pay_later_pm"]["Klarna"];
          cy.retrievePaymentCallTest({ globalState, data: klarnaData });
        });
      });
    }
  );

  context("Klarna PayLater - Separate Create and Confirm flow test", () => {
    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm Payment -> Handle PayLater Redirection", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["AutoCapture"];
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "three_ds",
          "automatic",
          globalState
        );
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("List Merchant Payment Methods", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Merchant Payment Methods");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["Klarna"];
        cy.confirmBankRedirectCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle PayLater Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle PayLater Redirection");
          return;
        }
        const expected_redirection =
          globalState.get("baseUrl") + "/payments/completion";
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handlePayLaterRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      });

      cy.step("Retrieve Payment after Redirection", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Retrieve Payment after Redirection"
          );
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["Klarna"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
      });
    });
  });

  context("Capture on wrong status - Error test", () => {
    it("Create Payment Intent -> Confirm Payment -> Attempt Capture on requires_customer_action status", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["ManualCapture"];
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "three_ds",
          "manual",
          globalState
        );
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("List Merchant Payment Methods", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Merchant Payment Methods");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["Klarna"];
        cy.confirmBankRedirectCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
      });

      cy.step("Attempt Capture on wrong status", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Attempt Capture on wrong status");
          return;
        }
        const captureData = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["CaptureOnWrongStatus"];
        cy.captureCallTest(fixtures.captureBody, captureData, globalState);
      });
    });
  });

  context("Confirm without payment_method_data - Error test", () => {
    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm Payment without payment_method_data", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["AutoCapture"];
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "three_ds",
          "automatic",
          globalState
        );
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("List Merchant Payment Methods", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Merchant Payment Methods");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment without payment_method_data", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Confirm Payment without payment_method_data"
          );
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["ConfirmWithoutPmData"];
        cy.confirmBankRedirectCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
      });
    });
  });

  context("Atome Manual Capture - Not Supported", () => {
    it("Create and Confirm Payment with manual capture (expect error)", () => {
      cy.step("Create and Confirm Payment", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["AtomeManualCaptureUnsupported"];
        const apiKey = globalState.get("apiKey");
        const baseUrl = globalState.get("baseUrl");
        const profileId = globalState.get("profileId");
        const customerId = globalState.get("customerId");

        const requestBody = { ...fixtures.createPaymentBody };
        for (const key in data.Request) {
          requestBody[key] = data.Request[key];
        }
        requestBody.authentication_type = "three_ds";
        requestBody.capture_method = "manual";
        requestBody.profile_id = profileId;
        requestBody.customer_id = customerId;

        cy.request({
          method: "POST",
          url: `${baseUrl}/payments`,
          headers: {
            "Content-Type": "application/json",
            "api-key": apiKey,
          },
          body: requestBody,
          failOnStatusCode: false,
        }).then((response) => {
          expect(response.status).to.equal(200);
          expect(response.body.status).to.equal("requires_confirmation");
          expect(response.body.connector).to.equal(null);
        });
      });
    });
  });
});
