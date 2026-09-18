import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;
let connector;

describe("Voucher Payment tests", () => {
  before("seed global state", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        connector = globalState.get("connectorId");

        if (
          utils.shouldIncludeConnector(
            connector,
            utils.CONNECTOR_LISTS.INCLUDE.VOUCHER
          )
        ) {
          skip = true;
          return;
        }
      })
      .then(() => {
        if (skip) {
          cy.log(
            `Skipping voucher payment tests for connector: ${connector} -- not in VOUCHER inclusion list`
          );
          this.skip();
        }
      });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Boleto Voucher Payment", () => {
    it("Create Payment Intent -> List Payment Methods -> Confirm Voucher Payment -> Handle Voucher Redirection -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "voucher_pm"
        ]["PaymentIntent"]("Boleto");
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

      cy.step("Confirm Boleto Voucher Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Boleto Voucher Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "voucher_pm"
        ]["Boleto"];
        cy.confirmVoucherCallTest(
          fixtures.confirmBody,
          data,
          true,
          globalState
        );
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle Voucher Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle Voucher Redirection");
          return;
        }
        const expected_redirection = fixtures.confirmBody["return_url"];
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handleVoucherRedirection(
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
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "voucher_pm"
        ]["Boleto"];
        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });

  context("OXXO Voucher Payment", () => {
    it("Create Payment Intent -> List Payment Methods -> Confirm Voucher Payment -> Handle Voucher Redirection -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "voucher_pm"
        ]["PaymentIntent"]("Oxxo");
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

      cy.step("Confirm OXXO Voucher Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm OXXO Voucher Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "voucher_pm"
        ]["Oxxo"];
        cy.confirmVoucherCallTest(
          fixtures.confirmBody,
          data,
          true,
          globalState
        );
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle Voucher Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle Voucher Redirection");
          return;
        }
        const expected_redirection = fixtures.confirmBody["return_url"];
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handleVoucherRedirection(
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
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "voucher_pm"
        ]["Oxxo"];
        cy.retrievePaymentCallTest({ globalState, data });
      });
    });

    it("OXXO invalid voucher value should return deserialization error", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "voucher_pm"
      ]["OxxoInvalidFormat"];

      cy.createConfirmPaymentTest(
        fixtures.createConfirmPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
    });
  });

  context("Alfamart Voucher Payment", () => {
    it("Create Payment Intent -> List Payment Methods -> Confirm Voucher Payment -> Handle Voucher Redirection -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "voucher_pm"
        ]["PaymentIntent"]("Alfamart");
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

      cy.step("Confirm Alfamart Voucher Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Alfamart Voucher Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "voucher_pm"
        ]["Alfamart"];
        cy.confirmVoucherCallTest(
          fixtures.confirmBody,
          data,
          true,
          globalState
        );
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle Voucher Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle Voucher Redirection");
          return;
        }
        const expected_redirection = fixtures.confirmBody["return_url"];
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handleVoucherRedirection(
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
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "voucher_pm"
        ]["Alfamart"];
        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });

  context("Indomaret Voucher Payment", () => {
    it("Create Payment Intent -> List Payment Methods -> Confirm Voucher Payment -> Handle Voucher Redirection -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "voucher_pm"
        ]["PaymentIntent"]("Indomaret");
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

      cy.step("Confirm Indomaret Voucher Payment", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Confirm Indomaret Voucher Payment"
          );
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "voucher_pm"
        ]["Indomaret"];
        cy.confirmVoucherCallTest(
          fixtures.confirmBody,
          data,
          true,
          globalState
        );
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle Voucher Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle Voucher Redirection");
          return;
        }
        const expected_redirection = fixtures.confirmBody["return_url"];
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handleVoucherRedirection(
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
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "voucher_pm"
        ]["Indomaret"];
        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });

  context("Seven-Eleven Voucher Payment", () => {
    it("Create Payment Intent -> List Payment Methods -> Confirm Voucher Payment -> Handle Voucher Redirection -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "voucher_pm"
        ]["PaymentIntent"]("SevenEleven");
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

      cy.step("Confirm Seven-Eleven Voucher Payment", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Confirm Seven-Eleven Voucher Payment"
          );
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "voucher_pm"
        ]["SevenEleven"];
        cy.confirmVoucherCallTest(
          fixtures.confirmBody,
          data,
          true,
          globalState
        );
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle Voucher Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle Voucher Redirection");
          return;
        }
        const expected_redirection = fixtures.confirmBody["return_url"];
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handleVoucherRedirection(
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
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "voucher_pm"
        ]["SevenEleven"];
        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });

  context("Lawson Voucher Payment", () => {
    it("Create Payment Intent -> List Payment Methods -> Confirm Voucher Payment -> Handle Voucher Redirection -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "voucher_pm"
        ]["PaymentIntent"]("Lawson");
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

      cy.step("Confirm Lawson Voucher Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Lawson Voucher Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "voucher_pm"
        ]["Lawson"];
        cy.confirmVoucherCallTest(
          fixtures.confirmBody,
          data,
          true,
          globalState
        );
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle Voucher Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle Voucher Redirection");
          return;
        }
        const expected_redirection = fixtures.confirmBody["return_url"];
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handleVoucherRedirection(
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
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "voucher_pm"
        ]["Lawson"];
        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });

  context("MiniStop Voucher Payment", () => {
    it("Create Payment Intent -> List Payment Methods -> Confirm Voucher Payment -> Handle Voucher Redirection -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "voucher_pm"
        ]["PaymentIntent"]("MiniStop");
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

      cy.step("Confirm MiniStop Voucher Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm MiniStop Voucher Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "voucher_pm"
        ]["MiniStop"];
        cy.confirmVoucherCallTest(
          fixtures.confirmBody,
          data,
          true,
          globalState
        );
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle Voucher Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle Voucher Redirection");
          return;
        }
        const expected_redirection = fixtures.confirmBody["return_url"];
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handleVoucherRedirection(
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
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "voucher_pm"
        ]["MiniStop"];
        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });

  context("FamilyMart Voucher Payment", () => {
    it("Create Payment Intent -> List Payment Methods -> Confirm Voucher Payment -> Handle Voucher Redirection -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "voucher_pm"
        ]["PaymentIntent"]("FamilyMart");
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

      cy.step("Confirm FamilyMart Voucher Payment", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Confirm FamilyMart Voucher Payment"
          );
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "voucher_pm"
        ]["FamilyMart"];
        cy.confirmVoucherCallTest(
          fixtures.confirmBody,
          data,
          true,
          globalState
        );
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle Voucher Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle Voucher Redirection");
          return;
        }
        const expected_redirection = fixtures.confirmBody["return_url"];
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handleVoucherRedirection(
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
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "voucher_pm"
        ]["FamilyMart"];
        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });

  context("Seicomart Voucher Payment", () => {
    it("Create Payment Intent -> List Payment Methods -> Confirm Voucher Payment -> Handle Voucher Redirection -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "voucher_pm"
        ]["PaymentIntent"]("Seicomart");
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

      cy.step("Confirm Seicomart Voucher Payment", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Confirm Seicomart Voucher Payment"
          );
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "voucher_pm"
        ]["Seicomart"];
        cy.confirmVoucherCallTest(
          fixtures.confirmBody,
          data,
          true,
          globalState
        );
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle Voucher Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle Voucher Redirection");
          return;
        }
        const expected_redirection = fixtures.confirmBody["return_url"];
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handleVoucherRedirection(
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
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "voucher_pm"
        ]["Seicomart"];
        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });

  context("PayEasy Voucher Payment", () => {
    it("Create Payment Intent -> List Payment Methods -> Confirm Voucher Payment -> Handle Voucher Redirection -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "voucher_pm"
        ]["PaymentIntent"]("PayEasy");
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

      cy.step("Confirm PayEasy Voucher Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm PayEasy Voucher Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "voucher_pm"
        ]["PayEasy"];
        cy.confirmVoucherCallTest(
          fixtures.confirmBody,
          data,
          true,
          globalState
        );
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle Voucher Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle Voucher Redirection");
          return;
        }
        const expected_redirection = fixtures.confirmBody["return_url"];
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handleVoucherRedirection(
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
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "voucher_pm"
        ]["PayEasy"];
        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });
});
