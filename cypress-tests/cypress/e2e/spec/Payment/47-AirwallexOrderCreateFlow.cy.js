import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Airwallex - Order Create Flow", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Order Create with Valid Order Details", () => {
    it("should create payment intent with valid order details", function () {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PaymentIntentWithOrderDetails"];

      if (data?.Configs?.TRIGGER_SKIP) {
        this.skip();
      }

      let shouldContinue = true;

      cy.step("Create Payment Intent with Order Details", () => {
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

      cy.step("Confirm Payment Intent", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment Intent");
          return;
        }
        const confirmData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["PaymentConfirmWithOrderDetails"];

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
    });
  });

  context("Order Create with Invalid Order Details", () => {
    it("should handle invalid order details gracefully", function () {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PaymentIntentWithInvalidOrderDetails"];

      if (data?.Configs?.TRIGGER_SKIP) {
        this.skip();
      }

      let shouldContinue = true;

      cy.step("Create Payment Intent with Invalid Order Details", () => {
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
    });
  });

  context("Order Create with Large Amount Order Details", () => {
    it("should create payment intent with large amount in order details", function () {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PaymentIntentWithLargeAmountOrderDetails"];

      if (data?.Configs?.TRIGGER_SKIP) {
        this.skip();
      }

      let shouldContinue = true;

      cy.step("Create Payment Intent with Large Amount Order Details", () => {
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

      cy.step("Confirm Payment Intent", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment Intent");
          return;
        }
        const confirmData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["PaymentConfirmWithLargeAmountOrderDetails"];

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
    });
  });

  context("Order Create with Special Characters in Order Details", () => {
    it("should create payment intent with special characters in order details", function () {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PaymentIntentWithSpecialCharsOrderDetails"];

      if (data?.Configs?.TRIGGER_SKIP) {
        this.skip();
      }

      let shouldContinue = true;

      cy.step("Create Payment Intent with Special Characters in Order Details", () => {
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

      cy.step("Confirm Payment Intent", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment Intent");
          return;
        }
        const confirmData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["PaymentConfirmWithSpecialCharsOrderDetails"];

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
    });
  });

  context("Order Create with Order Details and Shipping", () => {
    it("should create payment intent with order details and shipping cost", function () {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PaymentIntentWithOrderDetailsAndShipping"];

      if (data?.Configs?.TRIGGER_SKIP) {
        this.skip();
      }

      let shouldContinue = true;

      cy.step("Create Payment Intent with Order Details and Shipping", () => {
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

      cy.step("Confirm Payment Intent", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment Intent");
          return;
        }
        const confirmData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["PaymentConfirmWithOrderDetailsAndShipping"];

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
    });
  });

  context("Order Create with Minimal Order Details", () => {
    it("should create payment intent with minimal order details", function () {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PaymentIntentWithMinimalOrderDetails"];

      if (data?.Configs?.TRIGGER_SKIP) {
        this.skip();
      }

      let shouldContinue = true;

      cy.step("Create Payment Intent with Minimal Order Details", () => {
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

      cy.step("Confirm Payment Intent", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment Intent");
          return;
        }
        const confirmData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["PaymentConfirmWithMinimalOrderDetails"];

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
    });
  });

  context("Order Create with Order Details - Manual Capture", () => {
    it("should create payment intent with order details and manual capture", function () {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PaymentIntentWithOrderDetailsManual"];

      if (data?.Configs?.TRIGGER_SKIP) {
        this.skip();
      }

      let shouldContinue = true;

      cy.step("Create Payment Intent with Order Details - Manual", () => {
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "manual",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Confirm Payment Intent", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment Intent");
          return;
        }
        const confirmData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["PaymentConfirmWithOrderDetailsManual"];

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

      cy.step("Capture Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Capture Payment");
          return;
        }
        const captureData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["CaptureWithOrderDetails"];

        cy.captureCallTest(fixtures.captureBody, captureData, globalState);
      });
    });
  });
});
