import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Card - Sync Refund flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Card - Sync Refund flow test", () => {
    it("Create Payment Intent + Payment Methods Call + Confirm Payment Intent + Retrieve Payment after Confirmation + Refund Payment + Sync Refund", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PaymentIntent"];

      cy.step("Create Payment Intent", () =>
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        )
      );

      if (!utils.should_continue_further(data)) return;

      cy.step("Payment Methods Call", () =>
        cy.paymentMethodsCallTest(globalState)
      );

      const confirmData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSAutoCapture"];

      cy.step("Confirm Payment Intent", () =>
        cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState)
      );

      if (!utils.should_continue_further(confirmData)) return;

      cy.step("Retrieve Payment after Confirmation", () =>
        cy.retrievePaymentCallTest({ globalState, data: confirmData })
      );

      const refundData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["Refund"];

      cy.step("Refund Payment", () =>
        cy.refundCallTest(fixtures.refundBody, refundData, globalState)
      );

      if (!utils.should_continue_further(refundData)) return;

      const syncRefundData = getConnectorDetails(
        globalState.get("connectorId")
      )["card_pm"]["SyncRefund"];

      cy.step("Sync Refund", () =>
        cy.syncRefundCallTest(syncRefundData, globalState)
      );
    });
  });
});
