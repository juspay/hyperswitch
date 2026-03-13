import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";
import reportErrors from "../../../utils/reportErrors";

let globalState;

describe("Bank Redirect tests", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Blik Create and Confirm flow test", () => {
    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm Payment", () => {
      const errorStack = [];
      let shouldContinue = true;

      cy.step("Create Payment Intent", errorStack, () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["PaymentIntent"]("Blik");
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

      cy.step("List Merchant Payment Methods", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Merchant Payment Methods");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["Blik"];
        cy.confirmBankRedirectCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
      });

      cy.then(() => {
        if (errorStack.length > 0) {
          reportErrors(errorStack);
        }
      });
    });
  });

  context("EPS Create and Confirm flow test", () => {
    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm Payment -> Handle Bank Redirect Redirection", () => {
      const errorStack = [];
      let shouldContinue = true;

      cy.step("Create Payment Intent", errorStack, () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["PaymentIntent"]("Eps");
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

      cy.step("List Merchant Payment Methods", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Merchant Payment Methods");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["Eps"];
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

      cy.step("Handle Bank Redirect Redirection", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle Bank Redirect Redirection");
          return;
        }
        const expected_redirection = fixtures.confirmBody["return_url"];
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handleBankRedirectRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      });

      cy.then(() => {
        if (errorStack.length > 0) {
          reportErrors(errorStack);
        }
      });
    });
  });

  context("iDEAL Create and Confirm flow test", () => {
    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm Payment -> Handle Bank Redirect Redirection", () => {
      const errorStack = [];
      let shouldContinue = true;

      cy.step("Create Payment Intent", errorStack, () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["PaymentIntent"]("Ideal");
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

      cy.step("List Merchant Payment Methods", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Merchant Payment Methods");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["Ideal"];
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

      cy.step("Handle Bank Redirect Redirection", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle Bank Redirect Redirection");
          return;
        }
        const expected_redirection = fixtures.confirmBody["return_url"];
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handleBankRedirectRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      });

      cy.then(() => {
        if (errorStack.length > 0) {
          reportErrors(errorStack);
        }
      });
    });
  });

  context("Sofort Create and Confirm flow test", () => {
    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm Payment -> Handle Bank Redirect Redirection", () => {
      const errorStack = [];
      let shouldContinue = true;

      cy.step("Create Payment Intent", errorStack, () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["PaymentIntent"]("Sofort");
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

      cy.step("List Merchant Payment Methods", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Merchant Payment Methods");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["Sofort"];
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

      cy.step("Handle Bank Redirect Redirection", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle Bank Redirect Redirection");
          return;
        }
        const expected_redirection = fixtures.confirmBody["return_url"];
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handleBankRedirectRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      });

      cy.then(() => {
        if (errorStack.length > 0) {
          reportErrors(errorStack);
        }
      });
    });
  });

  context("Przelewy24 Create and Confirm flow test", () => {
    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm Payment -> Handle Bank Redirect Redirection", () => {
      const errorStack = [];
      let shouldContinue = true;

      cy.step("Create Payment Intent", errorStack, () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["PaymentIntent"]("Przelewy24");
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

      cy.step("List Merchant Payment Methods", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Merchant Payment Methods");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["Przelewy24"];
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

      cy.step("Handle Bank Redirect Redirection", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle Bank Redirect Redirection");
          return;
        }
        const expected_redirection = fixtures.confirmBody["return_url"];
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handleBankRedirectRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      });

      cy.then(() => {
        if (errorStack.length > 0) {
          reportErrors(errorStack);
        }
      });
    });
  });

  context("OpenBankingUk Create and Confirm flow test", () => {
    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm Payment -> Handle Bank Redirect Redirection -> Retrieve Payment", () => {
      const errorStack = [];
      let shouldContinue = true;

      cy.step("Create Payment Intent", errorStack, () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["PaymentIntent"]("OpenBankingUk");
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

      cy.step("List Merchant Payment Methods", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Merchant Payment Methods");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["OpenBankingUk"];
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

      cy.step("Handle Bank Redirect Redirection", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle Bank Redirect Redirection");
          return;
        }
        const expected_redirection = fixtures.confirmBody["return_url"];
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handleBankRedirectRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      });

      cy.step("Retrieve Payment", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["OpenBankingUk"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
      });

      cy.then(() => {
        if (errorStack.length > 0) {
          reportErrors(errorStack);
        }
      });
    });
  });

  context("OnlineBankingFpx Create and Confirm flow test", () => {
    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm Payment -> Handle Bank Redirect Redirection -> Retrieve Payment", () => {
      const errorStack = [];
      let shouldContinue = true;

      cy.step("Create Payment Intent", errorStack, () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["PaymentIntent"]("OnlineBankingFpx");
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

      cy.step("List Merchant Payment Methods", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Merchant Payment Methods");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["OnlineBankingFpx"];
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

      cy.step("Handle Bank Redirect Redirection", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle Bank Redirect Redirection");
          return;
        }
        const expected_redirection = fixtures.confirmBody["return_url"];
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handleBankRedirectRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      });

      cy.step("Retrieve Payment", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["OnlineBankingFpx"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
      });

      cy.then(() => {
        if (errorStack.length > 0) {
          reportErrors(errorStack);
        }
      });
    });
  });

  context("Interac Create and Confirm flow test", () => {
    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm Payment -> Handle Bank Redirect Redirection -> Retrieve Payment", () => {
      const errorStack = [];
      let shouldContinue = true;

      cy.step("Create Payment Intent", errorStack, () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["PaymentIntent"]("Interac");
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

      cy.step("List Merchant Payment Methods", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Merchant Payment Methods");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["Interac"];
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

      cy.step("Handle Bank Redirect Redirection", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle Bank Redirect Redirection");
          return;
        }
        const expected_redirection = fixtures.confirmBody["return_url"];
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handleBankRedirectRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      });

      cy.step("Retrieve Payment", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["Interac"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
      });

      cy.then(() => {
        if (errorStack.length > 0) {
          reportErrors(errorStack);
        }
      });
    });
  });
});
