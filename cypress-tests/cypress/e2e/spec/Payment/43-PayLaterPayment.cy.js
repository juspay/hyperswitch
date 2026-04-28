import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Pay Later Payment tests", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Affirm Create, Confirm, Capture and Refund flow test", () => {
    it("Create Payment Intent -> List Merchant Payment Methods -> Confirm Payment -> Capture Payment -> Refund Payment", () => {
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
        ]["Affirm"];
        cy.confirmPayLaterCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle Redirect", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle Redirect");
          return;
        }
        cy.handlePayLaterRedirect(globalState);
      });

      cy.step("Capture Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Capture Payment");
          return;
        }
        const captureData = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["Capture"];
        cy.captureCallTest(fixtures.captureBody, captureData, globalState);
        if (!utils.should_continue_further(captureData)) {
          shouldContinue = false;
        }
      });

      cy.step("Refund Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Refund Payment");
          return;
        }
        const refundData = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["Refund"];
        cy.refundCallTest(fixtures.refundBody, refundData, globalState);
      });
    });
  });
});
