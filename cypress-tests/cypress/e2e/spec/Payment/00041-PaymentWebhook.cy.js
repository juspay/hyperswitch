import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";
import { payment_methods_enabled } from "../../configs/Payment/Commons";

let globalState;
let connector;
let expectedIntentStatus;

describe("Payment Webhook Tests", () => {
  before("seed global state", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        connector = globalState.get("connectorId");

        if (
          utils.shouldIncludeConnector(
            connector,
            utils.CONNECTOR_LISTS.INCLUDE.PAYMENTS_WEBHOOK
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

    // If connector requires webhook signature verification (e.g. WorldPay),
    // set connector_webhook_details with the test secret during connector creation
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

  context("NoThreeDS Manual payment flow test", () => {
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

    it("Confirm No 3DS", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSAutoCapture"];

      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState).then(
        () => {
          expectedIntentStatus = globalState.get("paymentIntentStatus");
        }
      );
    });
  });

  context("Webhook Processing - Status Update & Retrieval", () => {
    let paymentId;
    let merchantId;

    before(() => {
      connector = globalState.get("connectorId");
      merchantId = globalState.get("merchantId");
      paymentId = globalState.get("paymentID");
    });

    it("Update-payment_status", () => {
      const PaymentsManualUpdateRequestBody = {
        attempt_status: "pending",
        attempt_id: `${paymentId}_1`,
        merchant_id: merchantId,
        payment_id: paymentId,
      };

      cy.manualPaymentStatusUpdateTest(
        globalState,
        PaymentsManualUpdateRequestBody
      );
    });

    it("send-webhook", () => {
      // Clone webhook fixture
      const webhookBody = structuredClone(
        fixtures.IncomingWebhookBody.webhookBodies[connector]["payment"]
      );

      // Extract webhook configuration for the specified connector
      const webhookConfig = getConnectorDetails(connector)["webhook"];
      const data = webhookConfig["TransactionIdConfig"];

      // Normalize transaction ID
      // Some connectors (e.g. NMI, WorldPay) use PaymentAttemptId for webhook lookup
      // instead of ConnectorTransactionId, so allow config to specify the source
      const idValue =
        data.source === "paymentAttemptID"
          ? `${globalState.get("paymentID")}_1`
          : globalState.get("connectorTransactionID");
      utils.setNormalizedValue(webhookBody, data, idValue);

      // Some connectors (e.g. Mollie) expect form-encoded bodies instead of JSON
      const contentType = webhookConfig.contentType || "application/json";

      // If connector requires webhook signature verification (e.g. WorldPay),
      // compute HMAC-SHA256 of the body and send the signature header
      if (webhookConfig.webhookSecret) {
        const bodyString = JSON.stringify(webhookBody);
        cy.task("hmac_sha256", {
          secret: webhookConfig.webhookSecret,
          message: bodyString,
        }).then((signature) => {
          const customHeaders = {
            [webhookConfig.signatureHeader]: `${webhookConfig.signaturePrefix}${signature}`,
          };
          // Pass stringified body to ensure signed bytes match sent bytes
          cy.IncomingWebhookTest(
            globalState,
            bodyString,
            contentType,
            customHeaders
          );
        });
      } else {
        cy.IncomingWebhookTest(globalState, webhookBody, contentType);
      }
    });

    it("Retrieve Payment Call Test", () => {
      cy.retrievePaymentCallTest({ globalState, expectedIntentStatus });
    });
  });
});
