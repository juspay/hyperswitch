import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Trustpay - Order Create Flow Tests", () => {
  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Trustpay Apple Pay Order Create flow test", () => {
    let shouldContinue = true;

    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("Create Payment Intent with Order Create for Apple Pay", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "order_create_pm"
      ]["ApplePayOrderCreate"];

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Payment Methods Call", () => {
      if (!shouldContinue) {
        cy.task("cli_log", "Skipping step: Payment Methods Call");
        return;
      }
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm Apple Pay Order Create Payment", () => {
      if (!shouldContinue) {
        cy.task("cli_log", "Skipping step: Confirm Apple Pay Order Create");
        return;
      }
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "order_create_pm"
      ]["ApplePayOrderCreate"];

      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Retrieve Payment with requires_customer_action status", () => {
      if (!shouldContinue) {
        cy.task("cli_log", "Skipping step: Retrieve Payment");
        return;
      }
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "order_create_pm"
      ]["ApplePayOrderCreate"];

      cy.retrievePaymentCallTest({
        globalState,
        data,
        expectedIntentStatus: "requires_customer_action",
      });
    });
  });

  context("Trustpay Google Pay Order Create flow test", () => {
    let shouldContinue = true;

    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("Create Payment Intent with Order Create for Google Pay", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "order_create_pm"
      ]["GooglePayOrderCreate"];

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Payment Methods Call", () => {
      if (!shouldContinue) {
        cy.task("cli_log", "Skipping step: Payment Methods Call");
        return;
      }
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm Google Pay Order Create Payment", () => {
      if (!shouldContinue) {
        cy.task("cli_log", "Skipping step: Confirm Google Pay Order Create");
        return;
      }
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "order_create_pm"
      ]["GooglePayOrderCreate"];

      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Retrieve Payment with requires_customer_action status", () => {
      if (!shouldContinue) {
        cy.task("cli_log", "Skipping step: Retrieve Payment");
        return;
      }
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "order_create_pm"
      ]["GooglePayOrderCreate"];

      cy.retrievePaymentCallTest({
        globalState,
        data,
        expectedIntentStatus: "requires_customer_action",
      });
    });
  });
});
