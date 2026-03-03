import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("UPI Payments - Hyperswitch", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("should complete UPI Collect payment and refund", () => {
    it("Create Payment Intent -> Fetch Payment Methods -> Confirm UPI Collect Payment -> Handle UPI Redirection -> Retrieve Payment -> Refund Payment", () => {
      const createPaymentData = getConnectorDetails(
        globalState.get("connectorId")
      )["upi_pm"]["PaymentIntent"];
      cy.step("Create Payment Intent", () =>
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          createPaymentData,
          "three_ds",
          "automatic",
          globalState
        )
      );

      cy.step("Fetch Payment Methods", () =>
        cy.paymentMethodsCallTest(globalState)
      );

      const confirmData = getConnectorDetails(globalState.get("connectorId"))[
        "upi_pm"
      ]["UpiCollect"];
      cy.step("Confirm UPI Collect Payment", () =>
        cy.confirmUpiCall(fixtures.confirmBody, confirmData, true, globalState)
      );

      if (!utils.should_continue_further(confirmData)) return;

      const expected_redirection = fixtures.confirmBody["return_url"];
      const payment_method_type = globalState.get("paymentMethodType");
      cy.step("Handle UPI Redirection", () =>
        cy.handleUpiRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        )
      );

      cy.step("Retrieve Payment", () =>
        cy.retrievePaymentCallTest({ globalState, data: confirmData })
      );

      const refundData = getConnectorDetails(globalState.get("connectorId"))[
        "upi_pm"
      ]["Refund"];
      cy.step("Refund Payment", () =>
        cy.refundCallTest(fixtures.refundBody, refundData, globalState)
      );
    });
  });

  // Skipping UPI Intent intentionally as connector is throwing 5xx during redirection
  context("should complete UPI Intent payment", () => {
    it.skip("Create Payment Intent -> Fetch Payment Methods -> Confirm UPI Intent Payment -> Handle UPI Redirection -> Retrieve Payment", () => {
      const createPaymentData = getConnectorDetails(
        globalState.get("connectorId")
      )["upi_pm"]["PaymentIntent"];
      cy.step("Create Payment Intent", () =>
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          createPaymentData,
          "three_ds",
          "automatic",
          globalState
        )
      );

      cy.step("Fetch Payment Methods", () =>
        cy.paymentMethodsCallTest(globalState)
      );

      const confirmData = getConnectorDetails(globalState.get("connectorId"))[
        "upi_pm"
      ]["UpiIntent"];
      cy.step("Confirm UPI Intent Payment", () =>
        cy.confirmUpiCall(fixtures.confirmBody, confirmData, true, globalState)
      );

      if (!utils.should_continue_further(confirmData)) return;

      const expected_redirection = fixtures.confirmBody["return_url"];
      const payment_method_type = globalState.get("paymentMethodType");
      cy.step("Handle UPI Redirection", () =>
        cy.handleUpiRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        )
      );

      cy.step("Retrieve Payment", () =>
        cy.retrievePaymentCallTest({ globalState, data: confirmData })
      );
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
