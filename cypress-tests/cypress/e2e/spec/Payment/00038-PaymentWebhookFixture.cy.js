import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import * as utils from "../../configs/Routing/Utils";
import { payment_methods_enabled } from "../../configs/Payment/Commons";

describe("Payment Webhook Tests — Split Steps", () => {

  let globalState;

  before(() => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  it("merchant-create-call-test", () => {  
    cy.merchantCreateCallTest(fixtures.merchantCreateBody, globalState);  
  });  
  
  it("api-key-create-call-test", () => {  
    cy.apiKeyCreateTest(fixtures.apiKeyCreateBody, globalState);  
  });

  it("connector-create-call-test", () => {
    cy.createConnectorCallTest(
      "payment_processor",
      fixtures.createConnectorBody,
      payment_methods_enabled,
      globalState
    );
  });

  it("customer-create-call-test", () => {
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
  });

  it("create-payment-intent", () => {
    const connectorID = globalState.get("connectorId");
    const data =
      utils.getConnectorDetails(connectorID)["card_pm"]["PaymentIntent"];

    return cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      data,
      "no_three_ds",
      "automatic",
      globalState
    );
  });

  it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
  });

  it("confirm-payment", () => {
    const connectorID = globalState.get("connectorId");
    const data = utils.getConnectorDetails(connectorID)["card_pm"]["No3DSAutoCapture"];

    return cy.confirmCallTest(
      fixtures.confirmBody,
      data,
      true,
      globalState
    );
  });


  it("Update-payment_status", () => {
  cy.updatePaymentStatusTest(globalState, {
    Request: {
      attempt_status: "pending",
    },
    Response: {
      status: 200,
    },
  });
});



it("send-webhook", () => {
  cy.sendWebhookTest(globalState, {
    Response: { status: 200 }
  });
});

});
