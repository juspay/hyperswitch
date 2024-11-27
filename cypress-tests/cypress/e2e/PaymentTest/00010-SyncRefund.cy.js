import * as fixtures from "../../fixtures/imports";
import State from "../../utils/State";
import getConnectorDetails, * as utils from "../PaymentUtils/Utils";

let globalState;

describe("Card - Sync Refund flow test", () => {
  let should_continue = true; // variable that will be used to skip tests if a previous test fails

  beforeEach(function () {
    if (!should_continue) {
      this.skip();
    }
  });

  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  it("create-payment-call-test", () => {
    let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
      "PaymentIntent"
    ];

    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      data,
      "no_three_ds",
      "automatic",
      globalState
    );

    if (should_continue) should_continue = utils.should_continue_further(data);
  });

  it("payment_methods-call-test", () => {
    cy.paymentMethodsCallTest(globalState);
  });

  it("confirm-call-test", () => {
    let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
      "No3DSAutoCapture"
    ];

    cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

    if (should_continue) should_continue = utils.should_continue_further(data);
  });

  it("retrieve-payment-call-test", () => {
    let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
      "No3DSAutoCapture"
    ];

    cy.retrievePaymentCallTest(globalState, data);
  });

  it("refund-call-test", () => {
    let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
      "Refund"
    ];

    cy.refundCallTest(fixtures.refundBody, data, 6500, globalState);

    if (should_continue) should_continue = utils.should_continue_further(data);
  });

  it("sync-refund-call-test", () => {
    let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
      "SyncRefund"
    ];

    cy.syncRefundCallTest(data, globalState);

    if (should_continue) should_continue = utils.should_continue_further(data);
  });
});
