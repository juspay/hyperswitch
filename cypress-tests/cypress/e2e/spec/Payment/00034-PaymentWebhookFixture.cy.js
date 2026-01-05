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
    const data =
      utils.getConnectorDetails(connectorID)["card_pm"]["No3DSAutoCapture"];

    return cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
  });

  it("Update-payment_status", () => {
    const merchantId = globalState.get("merchantId");
    const paymentId = globalState.get("paymentID");

    return cy
      .request({
        method: "PUT",
        url: `${globalState.get("baseUrl")}/payments/${paymentId}/manual-update`,
        headers: {
          "Content-Type": "application/json",
          "api-key": globalState.get("adminApiKey"),
          "X-Merchant-Id": merchantId,
        },
        body: {
          attempt_status: "pending",
          attempt_id: `${paymentId}_1`,
          merchant_id: merchantId,
          payment_id: paymentId,
        },
      })
      .then((resp) => {
        expect(resp.status).to.eq(200);
      });
  });

  it("send-webhook", () => {
    const connectorID = globalState.get("connectorId");
    const connectorName = globalState.get("connectorName");
    const connectorTransactionId = globalState.get("connectorTransactionID");

    return cy
      .fixture(`webhooks/${connectorName}_payment_success.json`)
      .then((payload) => {
        const webhookPayload = { ...payload };

        // Replace nested field: data.object.id
        webhookPayload.data.object.id = connectorTransactionId;

        return cy.request({
          method: "POST",
          url: `${globalState.get("baseUrl")}/webhooks/${globalState.get("merchantId")}/${connectorID}`,
          body: webhookPayload,
          headers: { "Content-Type": "application/json" },
        });
      })
      .then((response) => {
        expect(response.status).to.equal(200);
      });
  });
});
