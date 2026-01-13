import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;
let connector;
let expected_intent_status;

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
          expected_intent_status = globalState.get("paymentIntentStatus");
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

      // Extract webhook reference ID configuration for the specified connector
      // This config defines how to locate and parse the payment reference ID from connector-specific webhook payloads
      const data =
        getConnectorDetails(connector)["webhook"]["TransactionIdConfig"];

      // Normalize transaction ID
      utils.setNormalizedValue(
        webhookBody,
        data,
        globalState.get("connectorTransactionID")
      );

      cy.IncomingWebhookTest(globalState, webhookBody);
    });

    it("Retrieve Payment Call Test", () => {
      cy.retrievePaymentCallTest(
        globalState,
        null,
        false,
        1,
        expected_intent_status
      );
    });
  });
});
