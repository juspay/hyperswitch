import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Bank Transfers", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Bank transfer - Pix forward flow", () => {
    it("Create Payment Intent for Pix -> List Merchant Payment Methods -> Confirm Bank Transfer for Pix -> Handle Bank Transfer Redirection for Pix", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent for Pix", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_transfer_pm"
        ]["PaymentIntent"]("Pix");
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

      cy.step("Confirm Bank Transfer for Pix", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Bank Transfer for Pix");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "bank_transfer_pm"
        ]["Pix"];
        cy.confirmBankTransferCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle Bank Transfer Redirection for Pix", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Handle Bank Transfer Redirection for Pix"
          );
          return;
        }
        const expected_redirection = fixtures.confirmBody["return_url"];
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handleBankTransferRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      });
    });
  });

  context("Bank transfer - Instant Bank Transfer Finland forward flow", () => {
    it("Create Payment Intent  -> List Merchant Payment Methods -> Confirm Bank Transfer  -> Handle Bank Transfer Redirection ", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent ", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_transfer_pm"
        ]["PaymentIntent"]("InstantBankTransferFinland");
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

      cy.step("Confirm Bank Transfer ", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Bank Transfer ");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "bank_transfer_pm"
        ]["InstantBankTransferFinland"];
        cy.confirmBankTransferCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle Bank Transfer Redirection ", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Handle Bank Transfer Redirection "
          );
          return;
        }
        const expected_redirection = fixtures.confirmBody["return_url"];
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handleBankTransferRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      });
    });
  });

  context("Bank transfer - Instant Bank Transfer Poland forward flow", () => {
    it("Create Payment Intent  -> List Merchant Payment Methods -> Confirm Bank Transfer  -> Handle Bank Transfer Redirection ", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent ", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_transfer_pm"
        ]["PaymentIntent"]("InstantBankTransferPoland");
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

      cy.step("Confirm Bank Transfer ", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Bank Transfer ");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "bank_transfer_pm"
        ]["InstantBankTransferPoland"];
        cy.confirmBankTransferCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle Bank Transfer Redirection ", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Handle Bank Transfer Redirection "
          );
          return;
        }
        const expected_redirection = fixtures.confirmBody["return_url"];
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handleBankTransferRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      });
    });
  });

  context("Bank transfer - Ach flow", () => {
    it("Create Payment Intent  -> List Merchant Payment Methods -> Confirm Bank Transfer  -> Handle Bank Transfer Redirection ", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent ", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_transfer_pm"
        ]["PaymentIntent"]("Ach");
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

      cy.step("Confirm Bank Transfer ", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Bank Transfer ");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "bank_transfer_pm"
        ]["Ach"];
        cy.confirmBankTransferCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      if (globalState.get("connectorId") != "checkbook") {
        cy.step("Handle Bank Transfer Redirection ", () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Handle Bank Transfer Redirection "
            );
            return;
          }
          const expected_redirection = fixtures.confirmBody["return_url"];
          const payment_method_type = globalState.get("paymentMethodType");
          cy.handleBankTransferRedirection(
            globalState,
            payment_method_type,
            expected_redirection
          );
        });
      }
    });
  });
});
