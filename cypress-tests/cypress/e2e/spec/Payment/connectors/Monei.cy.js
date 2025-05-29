import * as fixtures from "../../../../fixtures/imports";
import State from "../../../../utils/State";
import getConnectorDetails, * as utils from "../../../configs/Payment/Utils";

let globalState;

describe("Monei Connector Payment Tests", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      // Set connector to monei for these tests
      globalState.set("connectorId", "monei");
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Account and Connector Setup", () => {
    it("creates a business profile for testing", () => {
      cy.createBusinessProfileTest(
        fixtures.createBusinessProfileBody,
        globalState
      );
    });

    it("creates a merchant connector account for Monei", () => {
      cy.createConnectorCallTest(
        "payment",
        fixtures.createConnectorBody,
        ["card"],
        globalState
      );
    });
  });

  context("Card-NoThreeDS Auto Capture Payment Flow", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("creates a payment intent", () => {
      const data = getConnectorDetails("monei")["card_pm"]["PaymentIntent"];

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("lists available payment methods", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("confirms payment with no 3DS", () => {
      const data = getConnectorDetails("monei")["card_pm"]["No3DSAutoCapture"];

      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("verifies payment status", () => {
      const data = getConnectorDetails("monei")["card_pm"]["No3DSAutoCapture"];

      cy.retrievePaymentCallTest(globalState, data);
    });
  });

  context("Card-NoThreeDS Manual Capture Payment Flow", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("creates a payment intent for manual capture", () => {
      const data = getConnectorDetails("monei")["card_pm"]["PaymentIntent"];

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "manual",
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("confirms payment for manual capture", () => {
      const data = getConnectorDetails("monei")["card_pm"]["No3DSManualCapture"];

      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("captures the authorized payment", () => {
      const data = getConnectorDetails("monei")["card_pm"]["Capture"];

      cy.captureCallTest(fixtures.captureBody, data, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("verifies payment status after capture", () => {
      const data = getConnectorDetails("monei")["card_pm"]["Capture"];

      cy.retrievePaymentCallTest(globalState, data);
    });
  });

  context("Refund Flow", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("creates and confirms payment for refund", () => {
      const data = getConnectorDetails("monei")["card_pm"]["No3DSAutoCapture"];

      cy.createConfirmPaymentTest(
        fixtures.createConfirmPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("refunds the payment", () => {
      const data = getConnectorDetails("monei")["card_pm"]["Refund"];

      cy.refundCallTest(fixtures.refundBody, data, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("verifies refund status", () => {
      const data = getConnectorDetails("monei")["card_pm"]["SyncRefund"];

      cy.retrieveRefundCallTest(globalState, data);
    });
  });

  context("Partial Refund Flow", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("creates and confirms payment for partial refund", () => {
      const data = getConnectorDetails("monei")["card_pm"]["No3DSAutoCapture"];

      cy.createConfirmPaymentTest(
        fixtures.createConfirmPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("partially refunds the payment", () => {
      const data = getConnectorDetails("monei")["card_pm"]["PartialRefund"];
      const partialRefundBody = {
        ...fixtures.refundBody,
        amount: 2000,
      };

      cy.refundCallTest(partialRefundBody, data, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("verifies partial refund status", () => {
      const data = getConnectorDetails("monei")["card_pm"]["SyncRefund"];

      cy.retrieveRefundCallTest(globalState, data);
    });
  });

  context("Partial Capture Flow", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("creates a payment intent for partial capture", () => {
      const data = getConnectorDetails("monei")["card_pm"]["PaymentIntent"];

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "manual",
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("confirms payment for partial capture", () => {
      const data = getConnectorDetails("monei")["card_pm"]["No3DSManualCapture"];

      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("partially captures the authorized payment", () => {
      const data = getConnectorDetails("monei")["card_pm"]["PartialCapture"];
      const partialCaptureBody = {
        ...fixtures.captureBody,
        amount_to_capture: 2000,
      };

      cy.captureCallTest(partialCaptureBody, data, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("verifies payment status after partial capture", () => {
      const data = getConnectorDetails("monei")["card_pm"]["PartialCapture"];

      cy.retrievePaymentCallTest(globalState, data);
    });
  });

  context("Saved Card Flow", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("creates a customer for card saving", () => {
      cy.createCustomerTest(fixtures.createCustomerBody, globalState);
    });

    it("creates payment with saved card setup", () => {
      const data = getConnectorDetails("monei")["card_pm"]["SaveCardUseNo3DSAutoCapture"];

      cy.createPaymentIntentTest(
        {
          ...fixtures.createPaymentBody,
          customer_id: globalState.get("customerId"),
          setup_future_usage: "on_session",
        },
        data,
        "no_three_ds",
        "automatic",
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("confirms payment with saved card", () => {
      const data = getConnectorDetails("monei")["card_pm"]["SaveCardUseNo3DSAutoCapture"];

      cy.confirmCallTest(
        {
          ...fixtures.confirmBody,
          customer_id: globalState.get("customerId"),
          setup_future_usage: "on_session",
        },
        data,
        true,
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("verifies payment with saved card", () => {
      const data = getConnectorDetails("monei")["card_pm"]["SaveCardUseNo3DSAutoCapture"];

      cy.retrievePaymentCallTest(globalState, data);
    });
  });
});
