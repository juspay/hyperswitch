import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, {
  CONNECTOR_LISTS,
  shouldIncludeConnector,
} from "../../configs/Payment/Utils";
import * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Bank Debit tests", () => {
  before("seed global state", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);

        if (
          shouldIncludeConnector(
            globalState.get("connectorId"),
            CONNECTOR_LISTS.INCLUDE.BANK_DEBIT
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

  context("SEPA Bank Debit Create and Confirm flow test", () => {
    let shouldContinue = true;

    before("seed global state", function () {
      let skip = false;

      cy.task("getGlobalState")
        .then((state) => {
          globalState = new State(state);
          const connector = globalState.get("connectorId");

          if (
            shouldIncludeConnector(
              connector,
              CONNECTOR_LISTS.INCLUDE.INESPAY_BANK_SIMULATION
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

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });
    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm SEPA Bank Debit -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent for SEPA", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_debit_pm"
        ]["PaymentIntent"]("Sepa");
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

      cy.step("List Merchant Payment Methods", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Merchant Payment Methods");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm SEPA Bank Debit", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Merchant Payment Methods");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "bank_debit_pm"
        ]["Sepa"];
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
          "bank_debit_pm"
        ]["Sepa"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });
    });
  });

  context("ACH Bank Debit Create and Confirm flow test", () => {
    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm ACH Bank Debit -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent for ACH", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_debit_pm"
        ]["PaymentIntent"]("Ach");
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

      cy.step("List Merchant Payment Methods", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Merchant Payment Methods");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm ACH Bank Debit", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Merchant Payment Methods");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "bank_debit_pm"
        ]["Ach"];
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
          "bank_debit_pm"
        ]["Ach"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });
    });
  });

  context("BECS Bank Debit Create and Confirm flow test", () => {
    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm BECS Bank Debit -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent for BECS", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_debit_pm"
        ]["PaymentIntent"]("Becs");
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

      cy.step("List Merchant Payment Methods", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Merchant Payment Methods");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm BECS Bank Debit", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Merchant Payment Methods");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "bank_debit_pm"
        ]["Becs"];
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
          "bank_debit_pm"
        ]["Becs"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });
    });
  });

  context("BACS Bank Debit Create and Confirm flow test", () => {
    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm BACS Bank Debit -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent for BACS", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_debit_pm"
        ]["PaymentIntent"]("Bacs");
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

      cy.step("List Merchant Payment Methods", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Merchant Payment Methods");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm BACS Bank Debit", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Merchant Payment Methods");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "bank_debit_pm"
        ]["Bacs"];
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
          "bank_debit_pm"
        ]["Bacs"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });
    });
  });
});

// Inespay SEPA Bank Debit — isolated top-level describe, runs only for Inespay
describe("Inespay SEPA Bank Debit tests", () => {
  let globalState;

  before("seed global state", function () {
    let skip = false;
    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        const connector = globalState.get("connectorId");
        if (connector !== "inespay") {
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

  context("Inespay SEPA Bank Debit Create, Confirm, Refund and Sync flow", () => {
    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm SEPA -> Simulate Redirect -> Retrieve Payment -> Refund -> Sync Refund", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent for SEPA", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_debit_pm"
        ]["PaymentIntent"]("Sepa");
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

      cy.step("List Merchant Payment Methods", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Merchant Payment Methods");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm SEPA Bank Debit", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm SEPA Bank Debit");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "bank_debit_pm"
        ]["Sepa"];
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

      cy.step("Simulate Inespay Redirect Flow", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Simulate Inespay Redirect Flow");
          return;
        }
        const nextActionUrl = globalState.get("nextActionUrl");
        expect(nextActionUrl, "nextActionUrl should be present").to.be.a(
          "string"
        );

        // Suppress uncaught exceptions from the simulator page
        cy.on("uncaught:exception", () => false);

        // Visit the Inespay simulator page
        cy.visit(nextActionUrl, { failOnStatusCode: false });

        // Wait for the simulator page to load (not a 4xx/5xx error page)
        cy.get("body", { timeout: 30000 }).should("be.visible");

        // Step 0: Handle any modal overlay — click CLOSE button if present (non-blocking)
        cy.get("body").then(($body) => {
          // Look for any close/dismiss button in a modal overlay
          const possibleCloseSelectors = [
            'button[class*="close"]',
            'button[aria-label*="close" i]',
            'button[aria-label*="dismiss" i]',
            ".modal button",
            ".overlay button",
          ];
          for (const sel of possibleCloseSelectors) {
            const btn = $body.find(sel);
            if (btn.length > 0) {
              cy.wrap(btn.first()).click({ force: true });
              break;
            }
          }
        });

        // Step 1: Simulator Selection — open first multiselect, choose SIMULADOR, click continue
        cy.get(".multiselect", { timeout: 15000 })
          .first()
          .should("be.visible")
          .click();
        cy.get(".multiselect__element", { timeout: 10000 })
          .contains(/simulador/i)
          .click();
        cy.contains("button", /continue/i, { timeout: 10000 })
          .should("be.visible")
          .click();

        // Step 2: Login Step — enter credentials and submit
        cy.get('input[type="text"], input:not([type="password"])', {
          timeout: 15000,
        })
          .first()
          .should("be.visible")
          .clear()
          .type("user1");
        cy.get('input[type="password"]', { timeout: 10000 })
          .should("be.visible")
          .first()
          .clear()
          .type("1234");
        cy.contains("button", /access/i, { timeout: 10000 })
          .should("be.visible")
          .click();

        // Step 3a: Contract & Account Selection — click the contract dropdown
        // Wait for the page/section to load after login
        cy.get(".multiselect", { timeout: 15000 }).should(
          "have.length.at.least",
          1
        );

        // Open the first multiselect (Contract dropdown)
        cy.get(".multiselect").first().click();

        // Select "Contract 1" from the dropdown options
        cy.get(".multiselect__element", { timeout: 10000 })
          .contains(/contract\s*1/i)
          .click();

        // Step 3b: Open the second multiselect (Account dropdown)
        cy.get(".multiselect", { timeout: 10000 }).eq(1).click();

        // Select the account ending in 679
        cy.get(".multiselect__element", { timeout: 10000 })
          .contains(/ES[\*\s]*679/i)
          .click();

        // Step 3c: Click confirm button (wait until it is enabled)
        cy.contains("button", /confirm/i, { timeout: 15000 })
          .should("be.visible")
          .and("not.be.disabled")
          .click();

        // Step 4: OTP Verification — enter 1111 and submit
        cy.get(
          'input[inputmode="numeric"], input[type="number"], input[type="tel"], input[maxlength="4"]',
          { timeout: 15000 }
        )
          .should("be.visible")
          .first()
          .clear()
          .type("1111");
        cy.contains("button", /continue/i, { timeout: 10000 })
          .should("be.visible")
          .click();

        // Step 5: Final Validation — wait for payment to complete and validate success state
        cy.log("Waiting for redirect / payment flow to complete...");

        // Poll for URL change indicating successful redirect back to merchant
        cy.url({ timeout: 45000 }).should((url) => {
          const isSuccess =
            /status=(succeeded|success|completed)/i.test(url) ||
            /payment_status=(succeeded|success|completed)/i.test(url) ||
            /return_url/i.test(url) ||
            /localhost/i.test(url);
          expect(isSuccess, `Expected success redirect, got: ${url}`).to.be
            .true;
        });

        cy.log("Inespay simulator redirect flow completed successfully");
      });

      cy.step("Retrieve Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "bank_debit_pm"
        ]["Sepa"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Full Refund", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Full Refund");
          return;
        }
        const refundData = getConnectorDetails(globalState.get("connectorId"))[
          "bank_debit_pm"
        ]["Refund"];
        cy.refundCallTest(refundData, globalState);
        if (!utils.should_continue_further(refundData)) {
          shouldContinue = false;
        }
      });

      cy.step("Sync Refund", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Sync Refund");
          return;
        }
        const syncRefundData = getConnectorDetails(globalState.get("connectorId"))[
          "bank_debit_pm"
        ]["SyncRefund"];
        cy.syncRefundCallTest(syncRefundData, globalState);
        if (!utils.should_continue_further(syncRefundData)) {
          shouldContinue = false;
        }
      });
    });
  });
});
