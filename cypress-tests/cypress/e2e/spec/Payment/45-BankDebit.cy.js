import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, {
  CONNECTOR_LISTS,
  shouldIncludeConnector,
} from "../../configs/Payment/Utils";
import * as utils from "../../configs/Payment/Utils";
import { isLocalhost } from "../../../utils/RequestBodyUtils";

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
    before(function () {
      const baseUrl = globalState.get("baseUrl");
      // Skip on localhost unless vault is available (CYPRESS_VAULT_AVAILABLE=true)
      // VAULT_AVAILABLE env var is checked via isVaultAvailable() in RequestBodyUtils
      if (isLocalhost(baseUrl) && !Cypress.env("VAULT_AVAILABLE")) {
        const vaultUrl = Cypress.env("VAULT_URL") || "http://localhost:3001";
        // Check if vault is actually responding via node task
        cy.task("checkVaultHealth", { vaultUrl }).then((result) => {
          if (!result.healthy) {
            cy.task(
              "cli_log",
              `Skipping ACH Bank Debit tests - vault not responding at ${vaultUrl}. Set CYPRESS_VAULT_AVAILABLE=true to skip this check.`
            );
            this.skip();
          } else {
            cy.task(
              "cli_log",
              `Vault is healthy at ${vaultUrl}, running ACH tests`
            );
          }
        });
      }
    });

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
          cy.task("cli_log", "Skipping step: Confirm ACH Bank Debit");
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
    it("Create and Confirm BECS Bank Debit -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create and Confirm BECS Bank Debit", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_debit_pm"
        ]["Becs"];
        if (!utils.should_continue_further(data)) {
          cy.task(
            "cli_log",
            "Skipping step: Create and Confirm BECS Bank Debit"
          );
          shouldContinue = false;
          return;
        }
        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_debit_pm"
        ]["Becs"];
        cy.retrievePaymentCallTest({ globalState, data: data });
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
          cy.task("cli_log", "Skipping step: Confirm BACS Bank Debit");
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

  context("SEPA Bank Debit Mandate flow test", () => {
    it("CIT mandate creation -> MIT mandate reuse for SEPA", () => {
      let shouldContinue = true;

      cy.step("CIT mandate creation for SEPA", () => {
        // citForMandatesCallTest params: requestBody, data, amount, confirm, capture_method, payment_type, globalState
        // data contains connector-specific Request (amount, currency, bank details) and Response (expected status)
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_debit_pm"
        ]["MandateSingleUseSepa"];
        if (!utils.should_continue_further(data)) {
          cy.task("cli_log", "Skipping step: CIT mandate creation for SEPA");
          shouldContinue = false;
          return;
        }
        cy.citForMandatesCallTest(
          fixtures.citConfirmBody,
          data,
          data.Request.amount,
          true,
          "automatic",
          "new_mandate",
          globalState
        );
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("MIT mandate reuse for SEPA", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: MIT mandate reuse for SEPA");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_debit_pm"
        ]["MITAutoCaptureSepa"];
        cy.mitForMandatesCallTest(
          fixtures.mitConfirmBody,
          data,
          data.Request.amount,
          true,
          "automatic",
          globalState
        );
      });
    });
  });

  context("BECS Bank Debit Mandate flow test", () => {
    it("CIT mandate creation -> MIT mandate reuse for BECS", () => {
      let shouldContinue = true;

      cy.step("CIT mandate creation for BECS", () => {
        // citForMandatesCallTest params: requestBody, data, amount, confirm, capture_method, payment_type, globalState
        // data contains connector-specific Request (amount, currency, bank details) and Response (expected status)
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_debit_pm"
        ]["MandateSingleUseBecs"];
        if (!utils.should_continue_further(data)) {
          cy.task("cli_log", "Skipping step: CIT mandate creation for BECS");
          shouldContinue = false;
          return;
        }
        cy.citForMandatesCallTest(
          fixtures.citConfirmBody,
          data,
          data.Request.amount,
          true,
          "automatic",
          "new_mandate",
          globalState
        );
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("MIT mandate reuse for BECS", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: MIT mandate reuse for BECS");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_debit_pm"
        ]["MITAutoCaptureBecs"];
        cy.mitForMandatesCallTest(
          fixtures.mitConfirmBody,
          data,
          data.Request.amount,
          true,
          "automatic",
          globalState
        );
      });
    });
  });

  context("ACH Bank Debit Mandate flow test", () => {
    before(function () {
      const baseUrl = globalState.get("baseUrl");
      // Skip on localhost unless vault is confirmed running via health check
      if (isLocalhost(baseUrl)) {
        const vaultUrl = Cypress.env("VAULT_URL") || "http://localhost:3001";
        // Check if vault is actually responding via node task
        cy.task("checkVaultHealth", { vaultUrl }).then((result) => {
          if (!result.healthy) {
            cy.task(
              "cli_log",
              `Skipping ACH Bank Debit Mandate tests - vault not responding at ${vaultUrl}`
            );
            this.skip();
          } else {
            cy.task(
              "cli_log",
              `Vault is healthy at ${vaultUrl}, running ACH mandate tests`
            );
          }
        });
      }
    });

    it("CIT mandate creation -> MIT mandate reuse for ACH", () => {
      let shouldContinue = true;

      cy.step("CIT mandate creation for ACH", () => {
        // citForMandatesCallTest params: requestBody, data, amount, confirm, capture_method, payment_type, globalState
        // data contains connector-specific Request (amount, currency, bank details) and Response (expected status)
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_debit_pm"
        ]["MandateSingleUseAch"];
        if (!utils.should_continue_further(data)) {
          cy.task("cli_log", "Skipping step: CIT mandate creation for ACH");
          shouldContinue = false;
          return;
        }
        cy.citForMandatesCallTest(
          fixtures.citConfirmBody,
          data,
          data.Request.amount,
          true,
          "automatic",
          "new_mandate",
          globalState
        );
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Verify ACH microdeposits for mandate", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Verify ACH microdeposits for mandate"
          );
          return;
        }
        cy.verifyAchMicrodepositCallTest(globalState);
      });

      cy.step("MIT mandate reuse for ACH", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: MIT mandate reuse for ACH");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_debit_pm"
        ]["MITAutoCaptureAch"];
        cy.mitForMandatesCallTest(
          fixtures.mitConfirmBody,
          data,
          data.Request.amount,
          true,
          "automatic",
          globalState
        );
      });
    });
  });

  context("BACS Bank Debit Mandate flow test", () => {
    it("CIT mandate creation -> MIT mandate reuse for BACS", () => {
      let shouldContinue = true;

      cy.step("CIT mandate creation for BACS", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_debit_pm"
        ]["MandateSingleUseBacs"];
        if (!utils.should_continue_further(data)) {
          cy.task("cli_log", "Skipping step: CIT mandate creation for BACS");
          shouldContinue = false;
          return;
        }
        cy.citForMandatesCallTest(
          fixtures.citConfirmBody,
          data,
          data.Request.amount,
          true,
          "automatic",
          "new_mandate",
          globalState
        );
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("MIT mandate reuse for BACS", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: MIT mandate reuse for BACS");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_debit_pm"
        ]["MITAutoCaptureBacs"];
        if (!utils.should_continue_further(data)) {
          cy.task("cli_log", "Skipping step: MIT mandate reuse for BACS");
          return;
        }
        cy.mitForMandatesCallTest(
          fixtures.mitConfirmBody,
          data,
          data.Request.amount,
          true,
          "automatic",
          globalState
        );
      });
    });
  });
});
