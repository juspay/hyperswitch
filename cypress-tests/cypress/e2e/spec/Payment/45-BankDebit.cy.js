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
        cy.contains("button", "CLOSE", { timeout: 15000 })
          .should("be.visible")
          .click();

        // 2. Bank Selection — Click visible Vue multiselect wrapper, select "SIMULADOR", click "CONTINUE"
        cy.get(".multiselect", { timeout: 10000 })
          .should("be.visible")
          .click();
        cy.contains(".multiselect__option", "SIMULADOR", { timeout: 10000 })
          .should("be.visible")
          .click();
        cy.contains("button", "Continue", { timeout: 10000 })
          .should("be.visible")
          .click();

        // 3. Login Step — Enter user1 / 1234, click "ACCESS"
        // Wait for the login form to render (Vue SPA transition)
        cy.get('input[type="text"]:visible', { timeout: 15000 })
          .should("be.visible")
          .clear()
          .type("user1");
        cy.get('input[type="password"]:visible', { timeout: 10000 })
          .should("be.visible")
          .clear()
          .type("1234");
        cy.contains("button", "ACCESS", { timeout: 10000 })
          .should("be.visible")
          .click();

        // 4. Contract & Account Selection — Select "Contract: 1" and "Account: ES*********679", click "CONFIRM"
        // The simulator renders these as Vue multiselects or native selects;
        // try the multiselect pattern first (same component family as step 2).
        cy.get("body", { timeout: 15000 }).then(($body) => {
          const multiCount = $body.find(".multiselect:visible").length;
          if (multiCount >= 2) {
            // Vue multiselect path
            cy.get(".multiselect:visible").eq(0).click();
            cy.contains(".multiselect__option", "Contract: 1", {
              timeout: 10000,
            })
              .should("be.visible")
              .click();

            cy.get(".multiselect:visible").eq(1).click();
            cy.contains(".multiselect__option", "679", { timeout: 10000 })
              .should("be.visible")
              .click();
          } else {
            // Native <select> fallback
            cy.get("select:visible", { timeout: 10000 })
              .eq(0)
              .select("Contract: 1");
            cy.get("select:visible", { timeout: 10000 })
              .eq(1)
              .select("Account: ES*********679");
          }
        });

        cy.contains("button", "CONFIRM", { timeout: 10000 })
          .should("be.visible")
          .and("not.be.disabled")
          .click();

        // 5. OTP Verification — Enter 1111, click "continue"
        cy.get('input[type="text"]:visible, input[type="tel"]:visible, input[inputmode="numeric"]:visible', {
          timeout: 15000,
        })
          .should("be.visible")
          .first()
          .clear()
          .type("1111");
        cy.contains("button", /continue/i, { timeout: 10000 })
          .should("be.visible")
          .click();

        // 6. Final Payment Completion — Wait up to 30 seconds for success
        cy.contains(/success|completed|confirmado|realizado|succeeded/i, {
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
