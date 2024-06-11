import confirmBody from "../../fixtures/confirm-body.json";
import createPaymentBody from "../../fixtures/create-payment-body.json";
import refundBody from "../../fixtures/refund-flow-body.json";
import State from "../../utils/State";
import getConnectorDetails from "../PaymentUtils/utils";
import * as utils from "../PaymentUtils/utils";

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
    let req_data = data["Request"];
    let res_data = data["Response"];
    cy.createPaymentIntentTest(
      createPaymentBody,
      req_data,
      res_data,
      "no_three_ds",
      "automatic",
      globalState,
    );
    if (should_continue)
      should_continue = utils.should_continue_further(res_data);
  });

  it("payment_methods-call-test", () => {
    cy.paymentMethodsCallTest(globalState);
  });

  it("confirm-call-test", () => {
    console.log("confirm -> " + globalState.get("connectorId"));
    let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
      "No3DSAutoCapture"
    ];
    let req_data = data["Request"];
    let res_data = data["Response"];
    console.log("det -> " + data.card);
    cy.confirmCallTest(confirmBody, req_data, res_data, true, globalState);
    if (should_continue)
      should_continue = utils.should_continue_further(res_data);
  });

  it("retrieve-payment-call-test", () => {
    cy.retrievePaymentCallTest(globalState);
  });

  it("refund-call-test", () => {
    let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
      "Refund"
    ];
    let req_data = data["Request"];
    let res_data = data["Response"];
    cy.refundCallTest(refundBody, req_data, res_data, 6500, globalState);
    if (should_continue)
      should_continue = utils.should_continue_further(res_data);
  });

  it("sync-refund-call-test", () => {
    let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
      "SyncRefund"
    ];
    let req_data = data["Request"];
    let res_data = data["Response"];
    cy.syncRefundCallTest(req_data, res_data, globalState);
    if (should_continue)
      should_continue = utils.should_continue_further(res_data);
  });
});
