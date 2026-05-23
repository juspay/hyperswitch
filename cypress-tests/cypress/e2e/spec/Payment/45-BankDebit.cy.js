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
        cy.visit(nextActionUrl);
        cy.document().should("have.property", "readyState", "complete");

        // 1. Initial Page Handling — Click "CLOSE" button
        cy.wait(30000);
        cy.contains("button", "CLOSE", { timeout: 15000 })
          .should("be.visible")
          .click();

        // 1. Simulator Selection — Select "SIMULADOR", click "continue"
        cy.wait(2000);
        cy.get(".multiselect")
          .should("be.visible")
          .click();
        cy.contains(".multiselect__option", "SIMULADOR", { timeout: 10000 })
          .should("be.visible")
          .click();
        cy.contains("button", /continue/i, { timeout: 10000 })
          .should("be.visible")
          .click();

        // 2. Login Step — Enter user1 / 1234, click "access"
        cy.wait(2000);
        cy.get("input")
          .not('[type="password"]')
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

        // 3. Contract & Account Selection
        cy.wait(5000);

        // Open Contract selection dropdown
        cy.contains(/Contract selection/i, { timeout: 15000 })
          .should("be.visible")
          .click();

        // Select Contract: 1
        cy.wait(2000);
        cy.contains(/Contract:\s*1/i, { timeout: 15000 })
          .should("be.visible")
          .click();

        // Wait for Account dropdown to appear
        cy.wait(2000);

        // Open Account selection dropdown
        cy.contains(/Account selection/i, { timeout: 15000 })
          .should("be.visible")
          .click();

        // Select Account: ES**********679
        cy.wait(2000);
        cy.contains(/Account:\s*ES\*+679/i, { timeout: 15000 })
          .should("be.visible")
          .click();

        // Click confirm
        cy.wait(1000);
        cy.contains("button", /confirm/i, { timeout: 10000 })
          .should("be.visible")
          .and("not.be.disabled")
          .click();

        // 4. OTP Verification
        cy.wait(2000);

        cy.get('input[type="text"], input[type="tel"], input[inputmode="numeric"]', {
          timeout: 15000,
        })
          .should("be.visible")
          .first()
          .clear()
          .type("1111");

        cy.contains("button", /continue/i, { timeout: 10000 })
          .should("be.visible")
          .click();

        // 5. Final Validation — Wait for redirect to complete, validate success state
        cy.log("Waiting for redirect/payment flow to complete...");
        // Wait for the page to redirect back or show completion
        cy.wait(5000);
        // Validate final success/completion state on the page
        cy.contains(/success|completed|confirmado|realizado|succeeded|finalizado/i, {
          timeout: 30000,
        });
        cy.log("Inespay simulator flow completed successfully");
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
