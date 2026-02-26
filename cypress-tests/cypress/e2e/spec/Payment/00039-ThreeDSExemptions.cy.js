import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

// Helper to get PaymentIntent config with currency from exemption test
function getPaymentIntentWithCurrency(connectorId, exemptionTestKey) {
  const cardPm = getConnectorDetails(connectorId)["card_pm"];
  const paymentIntent = { ...cardPm["PaymentIntent"] };
  const exemptionTest = cardPm[exemptionTestKey];

  // Override currency from exemption test config if specified
  if (exemptionTest?.Request?.currency) {
    paymentIntent.Request = {
      ...paymentIntent.Request,
      currency: exemptionTest.Request.currency,
    };
  }

  return paymentIntent;
}

describe("Card - ThreeDS payment with exemption indicators", () => {
  let shouldContinue = true;

  beforeEach(function () {
    if (!shouldContinue) {
      this.skip();
    }
  });

  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("3DS with Low Value Exemption", () => {
    it("create-payment-call-test", () => {
      const data = getPaymentIntentWithCurrency(
        globalState.get("connectorId"),
        "3DSAutoCaptureWithLowValueExemption"
      );

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm 3DS with Low Value Exemption", function () {
      if (
        !utils.shouldIncludeConnector(
          globalState.get("connectorId"),
          utils.CONNECTOR_LISTS.INCLUDE.THREEDS_EXEMPTIONS
        )
      ) {
        this.skip();
      }

      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["3DSAutoCaptureWithLowValueExemption"];

      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
  });

  context("3DS with Transaction Risk Assessment Exemption", () => {
    it("create-payment-call-test", () => {
      const data = getPaymentIntentWithCurrency(
        globalState.get("connectorId"),
        "3DSAutoCaptureWithTRAExemption"
      );

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm 3DS with TRA Exemption", function () {
      if (
        !utils.shouldIncludeConnector(
          globalState.get("connectorId"),
          utils.CONNECTOR_LISTS.INCLUDE.THREEDS_EXEMPTIONS
        )
      ) {
        this.skip();
      }

      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["3DSAutoCaptureWithTRAExemption"];

      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
  });

  context("3DS with Trusted Listing Exemption", () => {
    it("create-payment-call-test", () => {
      const data = getPaymentIntentWithCurrency(
        globalState.get("connectorId"),
        "3DSAutoCaptureWithTrustedListingExemption"
      );

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm 3DS with Trusted Listing Exemption", function () {
      if (
        !utils.shouldIncludeConnector(
          globalState.get("connectorId"),
          utils.CONNECTOR_LISTS.INCLUDE.THREEDS_EXEMPTIONS
        )
      ) {
        this.skip();
      }

      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["3DSAutoCaptureWithTrustedListingExemption"];

      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
  });

  context("3DS with SCA Delegation Exemption", () => {
    it("create-payment-call-test", () => {
      const data = getPaymentIntentWithCurrency(
        globalState.get("connectorId"),
        "3DSAutoCaptureWithScaDelegationExemption"
      );

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm 3DS with SCA Delegation Exemption", function () {
      if (
        !utils.shouldIncludeConnector(
          globalState.get("connectorId"),
          utils.CONNECTOR_LISTS.INCLUDE.THREEDS_EXEMPTIONS
        )
      ) {
        this.skip();
      }

      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["3DSAutoCaptureWithScaDelegationExemption"];

      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
  });

  context("3DS with Secure Corporate Payment Exemption", () => {
    it("create-payment-call-test", () => {
      const data = getPaymentIntentWithCurrency(
        globalState.get("connectorId"),
        "3DSAutoCaptureWithSecureCorporateExemption"
      );

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm 3DS with Secure Corporate Exemption", function () {
      if (
        !utils.shouldIncludeConnector(
          globalState.get("connectorId"),
          utils.CONNECTOR_LISTS.INCLUDE.THREEDS_EXEMPTIONS
        )
      ) {
        this.skip();
      }

      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["3DSAutoCaptureWithSecureCorporateExemption"];

      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
  });
});
