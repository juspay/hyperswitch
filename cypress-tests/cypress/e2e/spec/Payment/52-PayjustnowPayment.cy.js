import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

function retrievePaymentForPayLater(globalState, data) {
  const { Configs: configs = {} } = data || {};
  const configInfo = (() => {
    const cfg = configs;
    const dualMerchant = cfg.dualMerchant;
    const merchantPrefix = dualMerchant ? "dualMerchant" : "merchant";
    const profilePrefix = dualMerchant ? "dualProfile" : "profile";
    const merchantConnectorPrefix = dualMerchant
      ? "dualMerchantConnector"
      : "merchantConnector";
    return { merchantPrefix, profilePrefix, merchantConnectorPrefix };
  })();
  const payment_id = globalState.get("paymentID");

  cy.request({
    method: "GET",
    url: `${globalState.get("baseUrl")}/payments/${payment_id}?force_sync=true`,
    headers: {
      "Content-Type": "application/json",
      "api-key": globalState.get("apiKey"),
    },
    failOnStatusCode: false,
  }).then((response) => {
    expect(response.status).to.equal(200);
    expect(response.body.payment_id).to.equal(payment_id);
    expect(response.body.amount).to.equal(globalState.get("paymentAmount"));
    expect(
      ["succeeded", "requires_customer_action"],
      "payment status"
    ).to.include(response.body.status);

    if (response.body.payment_method_id) {
      globalState.set("paymentMethodId", response.body.payment_method_id);
    }
  });
}

describe("Pay Later tests", () => {
  let shouldContinue = true;

  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      if (
        !utils.CONNECTOR_LISTS.INCLUDE.PAY_LATER.includes(
          globalState.get("connectorId")
        )
      ) {
        shouldContinue = false;
      }
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  beforeEach(function () {
    if (!shouldContinue) {
      this.skip();
    }
  });

  context("PayLater Create and Confirm flow test", () => {
    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm PayLater Payment -> Handle Bank Redirect Redirection -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["PaymentIntent"]("Payjustnow");
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

      cy.step("Confirm PayLater Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm PayLater Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["Payjustnow"];
        
        // Update return_url to use ngrok URL for external accessibility
        const confirmBodyWithReturnUrl = {
          ...fixtures.confirmBody,
          return_url: Cypress.env("NGROK_URL") || "https://example.com/return"
        };
        
        cy.confirmBankRedirectCallTest(
          confirmBodyWithReturnUrl,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle Bank Redirect Redirection", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Handle Bank Redirect Redirection"
          );
          return;
        }
        const expected_redirection = Cypress.env("NGROK_URL") || "https://example.com/return";
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handleBankRedirectRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      });

      cy.step("Retrieve Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["Payjustnow"];
        retrievePaymentForPayLater(globalState, confirmData);
      });
    });
  });

  context("PayLater Full Refund flow test", () => {
    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm PayLater Payment -> Handle Bank Redirect Redirection -> Retrieve Payment -> Refund Payment -> Sync Refund", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["PaymentIntent"]("Payjustnow");
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

      cy.step("Confirm PayLater Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm PayLater Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["Payjustnow"];
        
        // Update return_url to use ngrok URL for external accessibility
        const confirmBodyWithReturnUrl = {
          ...fixtures.confirmBody,
          return_url: Cypress.env("NGROK_URL") || "https://example.com/return"
        };
        
        cy.confirmBankRedirectCallTest(
          confirmBodyWithReturnUrl,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle Bank Redirect Redirection", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Handle Bank Redirect Redirection"
          );
          return;
        }
        const expected_redirection = Cypress.env("NGROK_URL") || "https://example.com/return";
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handleBankRedirectRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      });

      cy.step("Retrieve Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["Payjustnow"];
        retrievePaymentForPayLater(globalState, confirmData);
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
          "pay_later_pm"
        ]["Refund"];
        cy.refundCallTest(fixtures.refundBody, refundData, globalState);
        if (!utils.should_continue_further(refundData)) {
          shouldContinue = false;
        }
      });

      cy.step("Sync Refund", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Sync Refund");
          return;
        }
        const syncRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["pay_later_pm"]["SyncRefund"];
        cy.syncRefundCallTest(syncRefundData, globalState);
      });
    });
  });

  context("PayLater Partial Refund flow test", () => {
    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm PayLater Payment -> Handle Bank Redirect Redirection -> Retrieve Payment -> Partial Refund Payment -> Sync Refund", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["PaymentIntent"]("Payjustnow");
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

      cy.step("Confirm PayLater Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm PayLater Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["Payjustnow"];
        
        // Update return_url to use ngrok URL for external accessibility
        const confirmBodyWithReturnUrl = {
          ...fixtures.confirmBody,
          return_url: Cypress.env("NGROK_URL") || "https://example.com/return"
        };
        
        cy.confirmBankRedirectCallTest(
          confirmBodyWithReturnUrl,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle Bank Redirect Redirection", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Handle Bank Redirect Redirection"
          );
          return;
        }
        const expected_redirection = Cypress.env("NGROK_URL") || "https://example.com/return";
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handleBankRedirectRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      });

      cy.step("Retrieve Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["Payjustnow"];
        retrievePaymentForPayLater(globalState, confirmData);
        if (!utils.should_continue_further(confirmData)) {
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
        cy.refundCallTest(
          fixtures.refundBody,
          partialRefundData,
          globalState
        );
        if (!utils.should_continue_further(partialRefundData)) {
          shouldContinue = false;
        }
      });

      cy.step("Sync Refund", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Sync Refund");
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
