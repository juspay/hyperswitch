import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";
import { payment_methods_enabled } from "../../configs/Payment/Commons";

let globalState;
let connector;

describe("Outgoing Webhook Event Configuration Tests", () => {
  before("seed global state", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        connector = globalState.get("connectorId");

        if (
          utils.shouldIncludeConnector(
            connector,
            utils.CONNECTOR_LISTS.INCLUDE.OUTGOING_WEBHOOK_EVENT_CONFIG
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

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  it("merchant-create-call-test", () => {
    cy.merchantCreateCallTest(fixtures.merchantCreateBody, globalState);
  });

  it("api-key-create-call-test", () => {
    cy.apiKeyCreateTest(fixtures.apiKeyCreateBody, globalState);
  });

  it("customer-create-call-test", () => {
    cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
  });

  it("Create merchant connector account", () => {
    const connectorBody = structuredClone(fixtures.createConnectorBody);

    cy.createConnectorCallTest(
      "payment_processor",
      connectorBody,
      payment_methods_enabled,
      globalState
    );
  });

  context("Payment Event Gating — Enabled Statuses", () => {
    it("configure payment events to succeeded only", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["OutgoingWebhookEventConfig"]["PaymentOnlySucceeded"];
      const webhookDetails = structuredClone(data.Request.webhook_details);
      webhookDetails.webhook_url = `${utils.getMockServerBaseUrl()}/webhook`;

      cy.updateBusinessProfileWebhookConfigTest(
        { webhook_details: webhookDetails },
        globalState
      );
    });

    it("reset captured outgoing webhooks", () => {
      cy.resetCapturedOutgoingWebhooksTest();
    });

    it("create and confirm payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSAutoCapture"];

      cy.createConfirmPaymentTest(
        fixtures.createConfirmPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
    });

    it("assert payment_succeeded webhook emitted", () => {
      const delayData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["OutgoingWebhookEventConfig"]["WebhookDeliveryDelay"];

      cy.getCapturedOutgoingWebhooksTest(
        globalState,
        {
          expectedCount: 1,
          eventType: "payment_succeeded",
          contentType: "payment_details",
          objectId: globalState.get("paymentID"),
        },
        delayData
      );
    });
  });

  context("Payment Event Gating — Suppressed Statuses", () => {
    it("configure payment events to exclude succeeded", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["OutgoingWebhookEventConfig"]["PaymentExcludesSucceeded"];
      const webhookDetails = structuredClone(data.Request.webhook_details);
      webhookDetails.webhook_url = `${utils.getMockServerBaseUrl()}/webhook`;

      cy.updateBusinessProfileWebhookConfigTest(
        { webhook_details: webhookDetails },
        globalState
      );
    });

    it("reset captured outgoing webhooks", () => {
      cy.resetCapturedOutgoingWebhooksTest();
    });

    it("create and confirm payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSAutoCapture"];

      cy.createConfirmPaymentTest(
        fixtures.createConfirmPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
    });

    it("assert no payment webhook emitted", () => {
      const delayData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["OutgoingWebhookEventConfig"]["WebhookDeliveryDelay"];

      cy.getCapturedOutgoingWebhooksTest(
        globalState,
        {
          expectedCount: 0,
        },
        delayData
      );
    });
  });

  context("Payment Event Gating — Re-enable Control", () => {
    it("re-enable payment events to succeeded only", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["OutgoingWebhookEventConfig"]["PaymentOnlySucceeded"];
      const webhookDetails = structuredClone(data.Request.webhook_details);
      webhookDetails.webhook_url = `${utils.getMockServerBaseUrl()}/webhook`;

      cy.updateBusinessProfileWebhookConfigTest(
        { webhook_details: webhookDetails },
        globalState
      );
    });

    it("reset captured outgoing webhooks", () => {
      cy.resetCapturedOutgoingWebhooksTest();
    });

    it("create and confirm payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSAutoCapture"];

      cy.createConfirmPaymentTest(
        fixtures.createConfirmPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
    });

    it("assert payment_succeeded webhook emitted again", () => {
      const delayData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["OutgoingWebhookEventConfig"]["WebhookDeliveryDelay"];

      cy.getCapturedOutgoingWebhooksTest(
        globalState,
        {
          expectedCount: 1,
          eventType: "payment_succeeded",
          contentType: "payment_details",
          objectId: globalState.get("paymentID"),
        },
        delayData
      );
    });
  });

  context("Refund Event Gating — Enabled Statuses", () => {
    it("configure refund events to success only and suppress payment events", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["OutgoingWebhookEventConfig"]["RefundOnlySuccess"];
      const webhookDetails = structuredClone(data.Request.webhook_details);
      webhookDetails.webhook_url = `${utils.getMockServerBaseUrl()}/webhook`;

      cy.updateBusinessProfileWebhookConfigTest(
        { webhook_details: webhookDetails },
        globalState
      );
    });

    it("create and confirm payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSAutoCapture"];

      cy.createConfirmPaymentTest(
        fixtures.createConfirmPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
    });

    it("reset captured outgoing webhooks", () => {
      cy.resetCapturedOutgoingWebhooksTest();
    });

    it("refund the payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["Refund"];

      cy.refundCallTest(fixtures.refundBody, data, globalState);
    });

    it("assert refund_succeeded webhook emitted", () => {
      const delayData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["OutgoingWebhookEventConfig"]["WebhookDeliveryDelay"];

      cy.getCapturedOutgoingWebhooksTest(
        globalState,
        {
          expectedCount: 1,
          eventType: "refund_succeeded",
          contentType: "refund_details",
          objectId: globalState.get("refundId"),
        },
        delayData
      );
    });
  });

  context("Refund Event Gating — Suppressed Statuses", () => {
    it("configure refund events to exclude success", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["OutgoingWebhookEventConfig"]["RefundExcludesSuccess"];
      const webhookDetails = structuredClone(data.Request.webhook_details);
      webhookDetails.webhook_url = `${utils.getMockServerBaseUrl()}/webhook`;

      cy.updateBusinessProfileWebhookConfigTest(
        { webhook_details: webhookDetails },
        globalState
      );
    });

    it("reset captured outgoing webhooks", () => {
      cy.resetCapturedOutgoingWebhooksTest();
    });

    it("refund the payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["Refund"];

      cy.refundCallTest(fixtures.refundBody, data, globalState);
    });

    it("assert no refund webhook emitted", () => {
      const delayData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["OutgoingWebhookEventConfig"]["WebhookDeliveryDelay"];

      cy.getCapturedOutgoingWebhooksTest(
        globalState,
        {
          expectedCount: 0,
        },
        delayData
      );
    });
  });

  context(
    "PENDING_SERVER_REBUILD — Dispute, Mandate and Invoice Event Gating",
    () => {
      it("configure dispute events to opened only", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["OutgoingWebhookEventConfig"]["DisputeOnlyOpened"];

        if (!utils.should_continue_further(data)) {
          cy.task(
            "cli_log",
            "TRIGGER_SKIP enabled for dispute event gating config (PENDING_SERVER_REBUILD)"
          );
          return;
        }

        const webhookDetails = structuredClone(data.Request.webhook_details);
        webhookDetails.webhook_url = `${utils.getMockServerBaseUrl()}/webhook`;

        cy.updateBusinessProfileWebhookConfigTest(
          { webhook_details: webhookDetails },
          globalState
        );
      });

      it("configure mandate events to active only", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["OutgoingWebhookEventConfig"]["MandateOnlyActive"];

        if (!utils.should_continue_further(data)) {
          cy.task(
            "cli_log",
            "TRIGGER_SKIP enabled for mandate event gating config (PENDING_SERVER_REBUILD)"
          );
          return;
        }

        const webhookDetails = structuredClone(data.Request.webhook_details);
        webhookDetails.webhook_url = `${utils.getMockServerBaseUrl()}/webhook`;

        cy.updateBusinessProfileWebhookConfigTest(
          { webhook_details: webhookDetails },
          globalState
        );
      });

      it("configure invoice events to paid only", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["OutgoingWebhookEventConfig"]["InvoiceOnlyPaid"];

        if (!utils.should_continue_further(data)) {
          cy.task(
            "cli_log",
            "TRIGGER_SKIP enabled for invoice event gating config (PENDING_SERVER_REBUILD)"
          );
          return;
        }

        const webhookDetails = structuredClone(data.Request.webhook_details);
        webhookDetails.webhook_url = `${utils.getMockServerBaseUrl()}/webhook`;

        cy.updateBusinessProfileWebhookConfigTest(
          { webhook_details: webhookDetails },
          globalState
        );
      });
    }
  );
});
