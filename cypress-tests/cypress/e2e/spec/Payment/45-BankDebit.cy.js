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

  context("Inespay SEPA Bank Debit Create, Confirm and Retrieve flow", () => {
    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm SEPA -> Simulate Redirect -> Retrieve Payment", () => {
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

        // Set up intercepts BEFORE the visit so we can wait for the XHRs
        cy.intercept("GET", "**/contracts-list/**").as("contractsList");
        cy.intercept("GET", "**/accounts-list**").as("accountsList");

        // Visit the Inespay simulator page
        cy.visit(nextActionUrl, { failOnStatusCode: false });

        // Wait for the simulator page to load (not a 4xx/5xx error page)
        cy.get("body", { timeout: 30000 }).should("be.visible");

        // Step 0: Wait for the simulator to load and dismiss any overlay.
        // The Inespay simulator first shows a "Transmitting payment information" loading page,
        // then navigates to the simulator UI. Wait for the navigation to complete.
        cy.url({ timeout: 60000 }).should("match", /\/(accounts|authorize)/);
        // Additional wait for the simulator UI to fully render
        cy.get(".multiselect, .modal, form", { timeout: 30000 }).should("exist");
        cy.wait(1000);

        // Dismiss any overlay/modal that may be covering the multiselect:
        // 1. An "Attention!" info dialog with a CLOSE button
        // 2. A Bulma .modal-background overlay
        cy.get("body").then(($body) => {
          if ($body.find(".modal").length > 0 || $body.find(".modal-background").length > 0) {
            // Try clicking a CLOSE/DISMISS button in the modal
            const allButtons = $body.find("button");
            let dismissed = false;
            allButtons.each((_, btn) => {
              if (!dismissed && /close/i.test(btn.textContent)) {
                cy.wrap(btn).click({ force: true });
                dismissed = true;
              }
            });
            if (!dismissed) {
              cy.get(".modal-background, .modal-close").first().click({ force: true });
            }
            cy.wait(800);
          }
        });

        // Steps 1 & 2 are conditional: the simulator may auto-select SIMULADOR and
        // skip the login step, landing directly on the /accounts/ contract selection page.
        cy.url().then((url) => {
          if (/\/accounts\//i.test(url)) {
            // Already on contract selection page — skip Steps 1 & 2
            cy.log("Already on accounts/contract page — skipping SIMULADOR selection and login");
          } else {
            // Step 1: Simulator Selection — open first multiselect, choose SIMULADOR, click continue
            cy.get(".multiselect", { timeout: 15000 })
              .first()
              .should("exist")
              .find(".multiselect__placeholder, .multiselect__single, .body-feature-input-placeholder")
              .first()
              .click({ force: true });
            cy.wait(500);
            cy.get(".multiselect__element", { timeout: 15000 })
              .contains(/simulador/i)
              .click({ force: true });
            cy.contains("button", /continue/i, { timeout: 10000 })
              .click({ force: true });

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
          }
        });

        // Step 3a: Contract & Account Selection
        // Wait for navigation to /accounts/ page and for contracts-list XHR to complete.
        cy.url({ timeout: 30000 }).should("match", /\/accounts\//i);
        cy.wait("@contractsList", { timeout: 15000 });
        cy.wait(1000); // allow Vue to re-render after XHR response

        // Open the Contract dropdown by clicking #contracts .multiselect
        cy.get("#contracts .multiselect", { timeout: 5000 }).should("exist");
        cy.get("#contracts .multiselect").click({ force: true });
        cy.wait(500);

        // Select "Contract 1" from the dropdown options
        cy.get(".multiselect__element", { timeout: 10000 })
          .contains(/contract\s*1/i)
          .click({ force: true });

        // Wait for accounts-list XHR to complete after contract selection
        cy.wait("@accountsList", { timeout: 15000 });
        cy.wait(500);

        // Open the Account dropdown (force since #account may have display:none briefly)
        cy.get("#account .multiselect").click({ force: true });
        cy.wait(500);

        // Select the first available account — click the option span inside the element
        cy.get(".multiselect__option", { timeout: 10000 })
          .first()
          .click({ force: true });
        cy.wait(500);

        // Step 3c: Click confirm button — wait until enabled and click
        cy.contains("button", /confirm/i, { timeout: 15000 })
          .and("not.be.disabled")
          .click({ force: true });

        // Step 4: OTP Verification — wait for the validation/OTP page and enter 1111
        cy.url({ timeout: 20000 }).should("match", /\/validation\//i);
        cy.wait(3000); // wait for Vue SPA to fully mount the OTP form after transfer/init XHR

        // The OTP page shows an SMS input field; the test code is always 1111
        // Try the broadest possible input selector to find the OTP field
        cy.get("input", { timeout: 20000 })
          .filter(":visible")
          .first()
          .type("1111", { force: true });

        cy.contains("button", /confirm/i, { timeout: 10000 }).click({ force: true });

        // Step 5: Wait for the simulator to redirect back to Hyperswitch (localhost)
        cy.log("Waiting for redirect back to Hyperswitch...");
        cy.url({ timeout: 60000 }).should((url) => {
          const isBack =
            /localhost/i.test(url) ||
            /status=(succeeded|success|completed)/i.test(url) ||
            /payment_status=(succeeded|success|completed)/i.test(url) ||
            /payment_id=/i.test(url);
          expect(isBack, `Expected redirect back to localhost, got: ${url}`).to
            .be.true;
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

    });
  });
});
