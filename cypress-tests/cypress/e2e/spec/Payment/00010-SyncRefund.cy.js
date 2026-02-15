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

  it("Card - Sync Refund flow test", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
      "PaymentIntent"
    ];

    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      data,
      "no_three_ds",
      "automatic",
      globalState
    );

    if (!utils.should_continue_further(data)) return;

    cy.paymentMethodsCallTest(globalState);

    const confirmData =
      getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "No3DSAutoCapture"
      ];

    cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);

    if (!utils.should_continue_further(confirmData)) return;

    cy.retrievePaymentCallTest({ globalState, data: confirmData });

    const refundData =
      getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Refund"];

    cy.refundCallTest(fixtures.refundBody, refundData, globalState);

    if (!utils.should_continue_further(refundData)) return;

    const syncRefundData =
      getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "SyncRefund"
      ];

    cy.syncRefundCallTest(syncRefundData, globalState);
  });
});