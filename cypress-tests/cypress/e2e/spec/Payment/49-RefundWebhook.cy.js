import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";
import { payment_methods_enabled } from "../../configs/Payment/Commons";

let globalState;
let connector;

describe("Refund Webhook Tests", () => {
  before("seed global state", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        connector = globalState.get("connectorId");

        if (
          utils.shouldIncludeConnector(
            connector,
            utils.CONNECTOR_LISTS.INCLUDE.REFUNDS_WEBHOOK
          )
        ) {
          skip = true;
        }
      })
      .then(() => {
        if (skip) {
          this.skip();
        }
      });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  it("merchant-create-call-test", () => {
    cy.merchantCreateCallTest(fixtures.merchantCreateBody, globalState);
  });

  it("api-key-create-call-test", () => {
    cy.apiKeyCreateTest(fixtures.apiKeyCreateBody, globalState);
  });

  it("Create merchant connector account", () => {
    const connectorBody = structuredClone(fixtures.createConnectorBody);

    const webhookConfig = getConnectorDetails(globalState.get("connectorId"))[
      "webhook"
    ];
    if (webhookConfig?.webhookSecret) {
      connectorBody.connector_webhook_details = {
        merchant_secret: webhookConfig.webhookSecret,
      };
    }

    cy.createConnectorCallTest(
      "payment_processor",
      connectorBody,
      payment_methods_enabled,
      globalState
    );
  });

  context("NoThreeDS Auto Capture + Refund flow", () => {
    it("create-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PaymentIntent"];

      cy.createPaymentIntentTest(
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

    it("Confirm No 3DS Auto Capture", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSAutoCapture"];

      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
    });

    it("refund-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["Refund"];

      cy.refundCallTest(fixtures.refundBody, data, globalState);
    });

    it("sync-refund-call-test", () => {
      // Sync to ensure connectorRefundId is populated in globalState
      // before sending the refund webhook
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["SyncRefund"];

      cy.syncRefundCallTest(data, globalState);
    });
  });

  context("Refund Webhook Processing - Status Update & Retrieval", function () {
    let merchantId;

    before(function () {
      connector = globalState.get("connectorId");
      merchantId = globalState.get("merchantId");

      // Skip this context if connectorRefundId is not available
      // (sandbox connectors may not return it)
      if (!globalState.get("connectorRefundId")) {
        this.skip();
      }
    });

    it("Update-refund_status", () => {
      const refundManualUpdateRequestBody = {
        merchant_id: merchantId,
        status: "pending",
      };

      cy.manualRefundStatusUpdateTest(
        globalState,
        refundManualUpdateRequestBody
      );
    });

    it("send-refund-webhook", () => {
      const webhookBody = structuredClone(
        fixtures.IncomingWebhookBody.webhookBodies[connector]["refund"]
      );

      const webhookConfig = getConnectorDetails(connector)["webhook"];

      cy.IncomingWebhookTest(globalState, webhookBody, webhookConfig, "refund");
    });

    it("Sync Refund Call Test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["SyncRefund"];

      cy.syncRefundCallTest(data, globalState);
    });
  });
});
