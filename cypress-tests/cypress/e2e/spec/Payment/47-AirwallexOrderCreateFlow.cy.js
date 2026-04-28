import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Airwallex Order Create Flow", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Order Create with No-3DS Auto Capture - Happy Path", () => {
    it("Create Payment Intent -> Confirm Payment (triggers OrderCreate) -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "wallet_pm"
        ]["PaymentIntent"];

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

      cy.step("Confirm Payment with Order Create", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment with Order Create");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "wallet_pm"
        ]["OrderCreate"];

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
          "wallet_pm"
        ]["OrderCreate"];

        cy.retrievePaymentCallTest({ globalState, data: confirmData });
      });
    });
  });

  context("Order Create with Create+Confirm - Happy Path", () => {
    it("Create and Confirm Payment (triggers OrderCreate) -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create and Confirm Payment with Order Create", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "wallet_pm"
        ]["OrderCreate"];

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
          "wallet_pm"
        ]["OrderCreate"];

        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });

  context("Order Create with Order Details - Happy Path", () => {
    it("Create Payment with Order Details -> Confirm -> Retrieve", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent with Order Details", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "wallet_pm"
        ]["PaymentIntentWithOrderDetails"];

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

      cy.step("Confirm Payment with Order Details", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment with Order Details");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "wallet_pm"
        ]["PaymentConfirmWithOrderDetails"];

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
        cy.retrievePaymentCallTest({ globalState });
      });
    });
  });

  context("Order Create with Order Details and Shipping - Happy Path", () => {
    it("Create Payment with Order Details and Shipping -> Confirm -> Retrieve", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent with Order Details and Shipping", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "wallet_pm"
        ]["PaymentIntentWithOrderDetailsAndShipping"];

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

      cy.step("Confirm Payment with Order Details", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment with Order Details");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "wallet_pm"
        ]["PaymentConfirmWithOrderDetails"];

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
        cy.retrievePaymentCallTest({ globalState });
      });
    });
  });

  context("Order Create with Invalid Order Details - Negative Case", () => {
    it("Create Payment with Invalid Order Details -> Expect Error", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent with Invalid Order Details", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "wallet_pm"
        ]["PaymentIntentWithInvalidOrderDetails"];

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

      cy.step("Attempt Confirm Payment (should fail with error)", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Attempt Confirm Payment (expected to fail)");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "wallet_pm"
        ]["PaymentConfirmWithOrderDetails"];

        cy.confirmCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
      });
    });
  });

  context("Order Create with Large Amount - Edge Case", () => {
    it("Create Payment with Large Order Amount -> Confirm -> Retrieve", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent with Large Order Amount", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "wallet_pm"
        ]["PaymentIntentWithLargeAmountOrderDetails"];

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

      cy.step("Confirm Payment with Order Details", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment with Order Details");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "wallet_pm"
        ]["PaymentConfirmWithOrderDetails"];

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
        cy.retrievePaymentCallTest({ globalState });
      });
    });
  });

  context("Order Create with Special Characters in Order Details - Edge Case", () => {
    it("Create Payment with Special Characters in Order -> Confirm -> Retrieve", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent with Special Characters in Order Details", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "wallet_pm"
        ]["PaymentIntentWithSpecialCharsOrderDetails"];

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

      cy.step("Confirm Payment with Order Details", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment with Order Details");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "wallet_pm"
        ]["PaymentConfirmWithOrderDetails"];

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
        cy.retrievePaymentCallTest({ globalState });
      });
    });
  });
});
