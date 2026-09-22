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

      cy.updateBusinessProfileWebhookConfigTest(
        { webhook_details: structuredClone(data.Request.webhook_details) },
        globalState
      );
    });
  });

  context("Payment Event Gating — Suppressed Statuses", () => {
    it("configure payment events to exclude succeeded", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["OutgoingWebhookEventConfig"]["PaymentExcludesSucceeded"];

      cy.updateBusinessProfileWebhookConfigTest(
        { webhook_details: structuredClone(data.Request.webhook_details) },
        globalState
      );
    });
  });

  context("Payment Event Gating — Re-enable Control", () => {
    it("re-enable payment events to succeeded only", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["OutgoingWebhookEventConfig"]["PaymentOnlySucceeded"];

      cy.updateBusinessProfileWebhookConfigTest(
        { webhook_details: structuredClone(data.Request.webhook_details) },
        globalState
      );
    });
  });

  context("Refund Event Gating — Enabled Statuses", () => {
    it("configure refund events to success only", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["OutgoingWebhookEventConfig"]["RefundOnlySuccess"];

      cy.updateBusinessProfileWebhookConfigTest(
        { webhook_details: structuredClone(data.Request.webhook_details) },
        globalState
      );
    });
  });

  context("Refund Event Gating — Suppressed Statuses", () => {
    it("configure refund events to exclude success", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["OutgoingWebhookEventConfig"]["RefundExcludesSuccess"];

      cy.updateBusinessProfileWebhookConfigTest(
        { webhook_details: structuredClone(data.Request.webhook_details) },
        globalState
      );
    });
  });

  context("Dispute, Mandate and Invoice Event Gating", () => {
    it("configure dispute events to opened only", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["OutgoingWebhookEventConfig"]["DisputeOnlyOpened"];

      cy.updateBusinessProfileWebhookConfigTest(
        { webhook_details: structuredClone(data.Request.webhook_details) },
        globalState
      );
    });

    it("configure mandate events to active only", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["OutgoingWebhookEventConfig"]["MandateOnlyActive"];

      cy.updateBusinessProfileWebhookConfigTest(
        { webhook_details: structuredClone(data.Request.webhook_details) },
        globalState
      );
    });

    it("configure invoice events to paid only", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["OutgoingWebhookEventConfig"]["InvoiceOnlyPaid"];

      cy.updateBusinessProfileWebhookConfigTest(
        { webhook_details: structuredClone(data.Request.webhook_details) },
        globalState
      );
    });
  });
});
