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

        // Step 0: Dismiss any overlay on the Inespay simulator page before interacting.
        // The simulator shows a modal-background overlay (and sometimes a warning-text overlay)
        // that must be gone before multiselect dropdowns become clickable.
        // Strategy: try multiple dismissal approaches, then wait for the overlay to be absent.
        cy.get("body", { timeout: 10000 }).then(($body) => {
          // 1. Dismiss .modal-background by clicking the modal-close button inside it, or the
          //    modal-background itself (clicking outside the modal box dismisses it in Bulma).
          if ($body.find(".modal-background").length > 0) {
            cy.get(".modal-background").click({ force: true });
            return;
          }
          // 2. Try Bulma .modal-close button (the × in the top-right of the modal)
          if ($body.find(".modal-close").length > 0) {
            cy.get(".modal-close").first().click({ force: true });
            return;
          }
          // 3. Try a button whose visible text is "close" (case-insensitive)
          const closeByText = $body
            .find("button")
            .filter((_, el) => /^\s*close\s*$/i.test(el.textContent));
          if (closeByText.length > 0) {
            cy.wrap(closeByText.first()).click({ force: true });
            return;
          }
          // 4. Try common attribute-based selectors
          const attrSelectors = [
            'button[class*="close"]',
            'button[aria-label="close"]',
            'button[aria-label="Close"]',
            'button[data-dismiss="modal"]',
            ".modal button",
            ".overlay button",
            ".warning button",
            ".warning-text ~ button",
          ];
          for (const sel of attrSelectors) {
            const btn = $body.find(sel);
            if (btn.length > 0) {
              cy.wrap(btn.first()).click({ force: true });
              return;
            }
          }
          // 5. If a warning-text paragraph is visible, click whatever button is near it
          const warningText = $body.find(".warning-text");
          if (warningText.length > 0) {
            const nearbyBtn = warningText
              .closest("div, section, aside")
              .find("button");
            if (nearbyBtn.length > 0) {
              cy.wrap(nearbyBtn.first()).click({ force: true });
            }
          }
        });

        // Wait for all known overlay selectors to be absent before proceeding.
        cy.get("body", { timeout: 15000 }).should(($body) => {
          expect($body.find(".modal-background").length, "modal-background absent").to.eq(0);
        });

        // Brief wait to allow Vue reactivity to finish removing the overlay from the DOM
        cy.wait(500);

        // Step 1: Simulator Selection — open first multiselect, choose SIMULADOR, click continue
        // Use { force: true } to guard against any residual overlay z-index issues.
        cy.get(".multiselect", { timeout: 15000 })
          .first()
          .should("be.visible")
          .click({ force: true });
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

        // Step 3a: Contract & Account Selection — wait for page to load after login
        // then open the Contract dropdown and select "Contract 1"
        cy.get(".multiselect", { timeout: 20000 }).should(
          "have.length.at.least",
          1
        );

        // Open the first multiselect (Contract dropdown)
        cy.get(".multiselect", { timeout: 15000 })
          .first()
          .click({ force: true });

        // Select "Contract 1" from the dropdown options
        cy.get(".multiselect__element", { timeout: 10000 })
          .contains(/contract\s*1/i)
          .should("be.visible")
          .click();

        // Step 3b: Open the second multiselect (Account dropdown) and select ES***679
        cy.get(".multiselect", { timeout: 15000 })
          .eq(1)
          .should("be.visible")
          .click({ force: true });

        // Select the account ending in 679
        cy.get(".multiselect__element", { timeout: 10000 })
          .contains(/ES[\*\s]*679/i)
          .should("be.visible")
          .click();

        // Step 3c: Click confirm button — wait until enabled
        cy.contains("button", /confirm/i, { timeout: 15000 })
          .should("be.visible")
          .and("not.be.disabled")
          .click();

        // Step 4: OTP Verification — enter 1111 and click continue
        cy.get(
          'input[inputmode="numeric"], input[type="number"], input[type="tel"], input[maxlength="4"]',
          { timeout: 20000 }
        )
          .should("be.visible")
          .first()
          .clear()
          .type("1111");
        cy.contains("button", /continue/i, { timeout: 10000 })
          .should("be.visible")
          .click();

        // Step 5: Final Validation — wait for the redirect / payment flow to complete
        // and validate the final success/completion state.
        cy.log("Waiting for redirect / payment flow to complete...");

        // The Inespay simulator redirects back to Hyperswitch after OTP confirmation.
        // Wait for the URL to indicate a successful outcome before asserting payment status.
        cy.url({ timeout: 60000 }).should((url) => {
          const isSuccess =
            /status=(succeeded|success|completed)/i.test(url) ||
            /payment_status=(succeeded|success|completed)/i.test(url) ||
            /payment_id=/i.test(url) ||
            /return_url/i.test(url) ||
            /localhost/i.test(url);
          expect(isSuccess, `Expected success redirect URL, got: ${url}`).to.be
            .true;
        });

        cy.log("Inespay simulator redirect flow completed successfully — final payment status will be validated by Retrieve Payment step");
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
