import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Real Time Payment", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("QRIS automatic capture flow", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("Create Payment Intent", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "real_time_payment_pm"
      ]["PaymentIntent"];

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Payment Methods Call Test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm QRIS", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "real_time_payment_pm"
      ]["Qris"];

      cy.confirmRealTimePaymentCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Retrieve Payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "real_time_payment_pm"
      ]["QrisRetrieve"];

      cy.retrievePaymentCallTest({
        globalState,
        data,
        expectedIntentStatus: "requires_customer_action",
      });
    });

    it("Sync Payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "real_time_payment_pm"
      ]["QrisRetrieve"];

      cy.retrievePaymentCallTest({
        globalState,
        data,
        expectedIntentStatus: "requires_customer_action",
      });
    });
  });

  context("QRIS mandate setup flow", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("Create Payment Intent with Mandate Setup", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "real_time_payment_pm"
      ]["PaymentIntent"];

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Payment Methods Call Test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm QRIS with Mandate", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "real_time_payment_pm"
      ]["QrisMandate"];

      cy.confirmRealTimePaymentCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Retrieve Payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "real_time_payment_pm"
      ]["QrisRetrieve"];

      cy.retrievePaymentCallTest({
        globalState,
        data,
        expectedIntentStatus: "requires_customer_action",
      });
    });
  });
});
