import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("UPI Payments - Hyperswitch", () => {
  let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

  context("[Payment] [UPI - UPI Collect] Create & Confirm + Refund", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    afterEach("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("Create payment intent", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "upi_pm"
      ]["PaymentIntent"];

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("List Merchant payment methods", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "upi_pm"
      ]["UpiCollect"];

      cy.confirmUpiCall(fixtures.confirmBody, data, true, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Handle UPI Redirection", () => {
      const expected_redirection = fixtures.confirmBody["return_url"];
      const payment_method_type = globalState.get("paymentMethodType");

      cy.handleUpiRedirection(
        globalState,
        payment_method_type,
        expected_redirection
      );
    });

    it("Retrieve payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "upi_pm"
      ]["UpiCollect"];

      cy.retrievePaymentCallTest(globalState, data);
    });

    it("Refund payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "upi_pm"
      ]["Refund"];

      cy.refundCallTest(fixtures.refundBody, data, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
  });

  // Skipping UPI Intent intentionally as connector is throwing 5xx during redirection
  context.skip("[Payment] [UPI - UPI Intent] Create & Confirm", () => {
    shouldContinue = true; // variable that will be used to skip tests if a previous test fails

    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("Create payment intent", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "upi_pm"
      ]["PaymentIntent"];

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("List Merchant payment methods", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "upi_pm"
      ]["UpiIntent"];

      cy.confirmUpiCall(fixtures.confirmBody, data, true, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Handle UPI Redirection", () => {
      const expected_redirection = fixtures.confirmBody["return_url"];
      const payment_method_type = globalState.get("paymentMethodType");

      cy.handleUpiRedirection(
        globalState,
        payment_method_type,
        expected_redirection
      );
    });

    it("Retrieve payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "upi_pm"
      ]["UpiIntent"];

      cy.retrievePaymentCallTest(globalState, data);
    });
  });
});

// TODO: This test is incomplete. Above has to be replicated here with changes to support SCL
describe.skip("UPI Payments -- Hyperswitch Stripe Compatibility Layer", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });
});
