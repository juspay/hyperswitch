import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails from "../../configs/Payment/Utils";
import { cardCreditEnabled } from "../../configs/PaymentMethodList/Commons";

let globalState;

function startHostedManualPayment() {
  const connectorDetails = getConnectorDetails("elavon_pg")["card_pm"];

  cy.createPaymentIntentTest(
    structuredClone(fixtures.createPaymentBody),
    connectorDetails["PaymentIntent"],
    "three_ds",
    "manual",
    globalState
  );
  cy.paymentMethodsCallTest(globalState);
  cy.confirmCallTest(
    structuredClone(fixtures.confirmBody),
    connectorDetails["3DSManualCapture"],
    true,
    globalState
  );
  cy.handleRedirection(globalState, fixtures.confirmBody["return_url"]);
  cy.retrievePaymentCallTest({
    globalState,
    data: connectorDetails["SyncPayment"],
    expectedIntentStatus: "requires_capture",
  });
}

function syncRefundThroughConnector(data) {
  const refundId = globalState.get("refundId");

  // Successful refunds are terminal, so force_sync alone is served from
  // storage. all_keys_required deliberately exercises UCS RSync.
  cy.request({
    method: "GET",
    url: `${globalState.get("baseUrl")}/refunds/${refundId}?force_sync=true&all_keys_required=true`,
    headers: {
      "Content-Type": "application/json",
      "api-key": globalState.get("apiKey"),
    },
    failOnStatusCode: false,
  }).then((response) => {
    expect(response.status).to.equal(data.Response.status);
    expect(response.headers["content-type"]).to.include("application/json");
    for (const key in data.Response.body) {
      expect(response.body[key]).to.equal(data.Response.body[key]);
    }
    expect(response.body.refund_id).to.equal(refundId);
  });
}

describe("Elavon PG hosted UCS flows", () => {
  before("initialize", function () {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      if (Cypress.env("CONNECTOR") !== "elavon_pg") {
        cy.log("Skipping Elavon PG hosted flow tests");
        this.skip();
      }
    });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("setup", () => {
    it("creates the merchant, API key, connector, customer, and UCS flag", () => {
      cy.merchantCreateCallTest(
        structuredClone(fixtures.merchantCreateBody),
        globalState
      );
      cy.apiKeyCreateTest(
        structuredClone(fixtures.apiKeyCreateBody),
        globalState
      );
      cy.createNamedConnectorCallTest(
        "payment_processor",
        structuredClone(fixtures.createConnectorBody),
        cardCreditEnabled,
        globalState,
        "elavon_pg",
        "elavon_pg_default"
      );
      cy.createCustomerCallTest(
        structuredClone(fixtures.customerCreateBody),
        globalState
      );
      cy.setupConfigs(globalState, "ucs_enabled", "true");
    });
  });

  context("hosted authorize, sync, capture, refund, and refund sync", () => {
    it("completes the HS to UCS to Elavon PG lifecycle", () => {
      const connectorDetails = getConnectorDetails("elavon_pg")["card_pm"];

      startHostedManualPayment();
      cy.captureCallTest(
        structuredClone(fixtures.captureBody),
        connectorDetails["Capture"],
        globalState
      );
      cy.refundCallTest(
        structuredClone(fixtures.refundBody),
        connectorDetails["Refund"],
        globalState
      );
      syncRefundThroughConnector(connectorDetails["SyncRefund"]);
    });
  });

  context("hosted authorize and void", () => {
    it("voids an authorized Elavon PG payment", () => {
      const connectorDetails = getConnectorDetails("elavon_pg")["card_pm"];

      startHostedManualPayment();
      cy.voidCallTest(
        structuredClone(fixtures.voidBody),
        connectorDetails["Void"],
        globalState
      );
    });
  });

  context("cleanup", () => {
    it("removes the UCS flag", () => {
      cy.setConfigs(globalState, "ucs_enabled", "true", "DELETE");
    });
  });
});
