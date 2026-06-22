import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, {
  CONNECTOR_LISTS,
  shouldIncludeConnector,
} from "../../configs/Payment/Utils";
import * as utils from "../../configs/Payment/Utils";

let globalState;

describe("PayLater tests", () => {
  before("seed global state", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);

        if (
          shouldIncludeConnector(
            globalState.get("connectorId"),
            CONNECTOR_LISTS.INCLUDE.PAY_LATER
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

  context("Klarna PayLater - Auto Capture flow test", () => {
    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm Payment -> Handle PayLater Redirection -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["AutoCapture"];
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

      cy.step("Confirm Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["Klarna"];
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

      cy.step("Handle PayLater Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle PayLater Redirection");
          return;
        }
        const expected_redirection =
          globalState.get("baseUrl") + "/payments/completion";
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handlePayLaterRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      });

      cy.step("Retrieve Payment after Redirection", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Retrieve Payment after Redirection"
          );
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["Klarna"];
        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });

  context("Klarna PayLater - Manual Capture flow test", () => {
    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm Payment -> Handle PayLater Redirection -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["PaymentIntent"];
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "three_ds",
          "manual",
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

      cy.step("Confirm Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["Klarna"];
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

      cy.step("Handle PayLater Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle PayLater Redirection");
          return;
        }
        const expected_redirection =
          globalState.get("baseUrl") + "/payments/completion";
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handlePayLaterRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      });

      cy.step("Retrieve Payment after Redirection", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Retrieve Payment after Redirection"
          );
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["Klarna"];
        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });

  context("Confirm without payment_method_data - Error test", () => {
    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm Payment without payment_method_data", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["AutoCapture"];
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

      cy.step("Confirm Payment without payment_method_data", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Confirm Payment without payment_method_data"
          );
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["ConfirmWithoutPmData"];
        cy.confirmBankRedirectCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
      });
    });
  });

  context("Atome PayLater - Auto Capture flow test", () => {
    before("skip if connector does not support Atome", function () {
      if (
        shouldIncludeConnector(
          globalState.get("connectorId"),
          CONNECTOR_LISTS.INCLUDE.ATOME
        )
      ) {
        this.skip();
      }
    });
    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm Payment -> Handle PayLater Redirection -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["AtomeAutoCapture"];
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

      cy.step("Confirm Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["Atome"];
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

      cy.step("Handle PayLater Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle PayLater Redirection");
          return;
        }
        const expected_redirection =
          globalState.get("baseUrl") + "/payments/completion";
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handlePayLaterRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      });

      cy.step("Retrieve Payment after Redirection", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Retrieve Payment after Redirection"
          );
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["Atome"];
        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });

  context("AfterpayClearpay - Auto Capture flow test", () => {
    before("skip if connector does not support AfterpayClearpay", function () {
      if (
        shouldIncludeConnector(
          globalState.get("connectorId"),
          CONNECTOR_LISTS.INCLUDE.AFTERPAY_CLEARPAY
        )
      ) {
        this.skip();
      }
    });
    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm Payment -> Handle PayLater Redirection -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["AfterpayClearpayAutoCapture"];
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

      cy.step("Confirm Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["AfterpayClearpay"];
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

      cy.step("Handle PayLater Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle PayLater Redirection");
          return;
        }
        const expected_redirection =
          globalState.get("baseUrl") + "/payments/completion";
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handlePayLaterRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      });

      cy.step("Retrieve Payment after Redirection", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Retrieve Payment after Redirection"
          );
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["AfterpayClearpay"];
        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });

  context("Alma PayLater - Auto Capture flow test", () => {
    before("skip if connector does not support Alma", function () {
      if (
        shouldIncludeConnector(
          globalState.get("connectorId"),
          CONNECTOR_LISTS.INCLUDE.ALMA
        )
      ) {
        this.skip();
      }
    });
    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm Payment -> Handle PayLater Redirection -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["AlmaAutoCapture"];
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

      cy.step("Confirm Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["Alma"];
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

      cy.step("Handle PayLater Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle PayLater Redirection");
          return;
        }
        const expected_redirection =
          globalState.get("baseUrl") + "/payments/completion";
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handlePayLaterRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      });

      cy.step("Retrieve Payment after Redirection", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Retrieve Payment after Redirection"
          );
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["Alma"];
        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });

  context("Walley - Auto Capture flow test", () => {
    before("skip if connector does not support Walley", function () {
      if (
        shouldIncludeConnector(
          globalState.get("connectorId"),
          CONNECTOR_LISTS.INCLUDE.WALLEY
        )
      ) {
        this.skip();
      }
    });
    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm Payment -> Handle PayLater Redirection -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["WalleyAutoCapture"];
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

      cy.step("Confirm Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["Walley"];
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

      cy.step("Handle PayLater Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle PayLater Redirection");
          return;
        }
        const expected_redirection =
          globalState.get("baseUrl") + "/payments/completion";
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handlePayLaterRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      });

      cy.step("Retrieve Payment after Redirection", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Retrieve Payment after Redirection"
          );
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["Walley"];
        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });
});
