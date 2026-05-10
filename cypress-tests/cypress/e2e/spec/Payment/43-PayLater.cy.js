import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("PayLater tests", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Klarna PayLater - Auto Capture flow test", () => {
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
    });
  });

  context("Klarna PayLater - Manual Capture flow test", () => {
    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm Payment -> Handle PayLater Redirection", () => {
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
    });
  });

  context("Affirm PayLater - Full Payment and Refund Flow", () => {
    before(function () {
      if (
        !utils.shouldIncludeConnector(
          globalState.get("connectorId"),
          utils.CONNECTOR_LISTS.INCLUDE.AFFIRM_REFUND
        )
      ) {
        cy.task(
          "cli_log",
          "Skipping Affirm tests: connector not in AFFIRM_REFUND list"
        );
        this.skip();
      }
    });

    it("Create Payment Intent -> Confirm with Affirm -> Verify Status -> Full Refund -> Sync Refund", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
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

      cy.step("Confirm Payment with Affirm", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment with Affirm");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["No3DSAutoCapture"];
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

      cy.step("Retrieve Payment Intent Status", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment Intent Status");
          return;
        }
        const retrieveData = getConnectorDetails(
          globalState.get("connectorId")
        )["common"]["RetrieveAfterRedirection"];
        cy.retrievePaymentIntentTest(globalState);
        if (!utils.should_continue_further(retrieveData)) {
          shouldContinue = false;
        }
      });

      cy.step("Full Refund", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Full Refund");
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

      cy.step("Sync Refund Status Check", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Sync Refund Status Check");
          return;
        }
        const syncRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["pay_later_pm"]["SyncRefund"];
        cy.syncRefundCallTest(syncRefundData, globalState);
      });
    });
  });

  context("Affirm PayLater - Partial Refund Flow", () => {
    before(function () {
      if (
        !utils.shouldIncludeConnector(
          globalState.get("connectorId"),
          utils.CONNECTOR_LISTS.INCLUDE.AFFIRM_REFUND
        )
      ) {
        cy.task(
          "cli_log",
          "Skipping Affirm tests: connector not in AFFIRM_REFUND list"
        );
        this.skip();
      }
    });

    it("Create Payment Intent -> Confirm with Affirm -> Verify Status -> Partial Refund -> Sync Refund", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
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

      cy.step("Confirm Payment with Affirm", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment with Affirm");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["No3DSAutoCapture"];
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

      cy.step("Retrieve Payment Intent Status", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment Intent Status");
          return;
        }
        const retrieveData = getConnectorDetails(
          globalState.get("connectorId")
        )["common"]["RetrieveAfterRedirection"];
        cy.retrievePaymentIntentTest(globalState);
        if (!utils.should_continue_further(retrieveData)) {
          shouldContinue = false;
        }
      });

      cy.step("Partial Refund", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Partial Refund");
          return;
        }
        const refundData = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["PartialRefund"];
        cy.refundCallTest(fixtures.refundBody, refundData, globalState);
        if (!utils.should_continue_further(refundData)) {
          shouldContinue = false;
        }
      });

      cy.step("Sync Refund Status Check", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Sync Refund Status Check");
          return;
        }
        const syncRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["pay_later_pm"]["SyncRefund"];
        cy.syncRefundCallTest(syncRefundData, globalState);
      });
    });
  });
});
