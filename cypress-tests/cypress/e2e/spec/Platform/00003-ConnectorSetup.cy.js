import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import { payment_methods_enabled } from "../../configs/Payment/Commons";

let globalState;

describe("Connector Setup for Connected Merchants", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Create Connector for Connected Merchant 1", () => {
    it("create-connector-for-cm1", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");
      const savedProfileId = globalState.get("profileId");
      const savedConnectorId = globalState.get("connectorId");

      globalState.set("merchantId", globalState.get("connectedMerchantId_1"));
      globalState.set("apiKey", globalState.get("apiKey_CM1"));
      globalState.set("profileId", globalState.get("profileId_CM1"));
      globalState.set("connectorId", "stripe");

      cy.createConnectorCallTest(
        "payment_processor",
        fixtures.createConnectorBody,
        payment_methods_enabled,
        globalState
      );

      cy.then(() => {
        globalState.set(
          "connectorId_CM1",
          globalState.get("merchantConnectorId")
        );
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
        globalState.set("profileId", savedProfileId);
        globalState.set("connectorId", savedConnectorId);
      });
    });

    it("retrieve-connector-for-cm1", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");
      const savedMerchantConnectorId = globalState.get("merchantConnectorId");
      const savedConnectorId = globalState.get("connectorId");

      globalState.set("merchantId", globalState.get("connectedMerchantId_1"));
      globalState.set("apiKey", globalState.get("apiKey_CM1"));
      globalState.set("merchantConnectorId", globalState.get("connectorId_CM1"));
      globalState.set("connectorId", "stripe");

      cy.connectorRetrieveCall(globalState);

      cy.then(() => {
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
        globalState.set("merchantConnectorId", savedMerchantConnectorId);
        globalState.set("connectorId", savedConnectorId);
      });
    });
  });

  context("Create Connector for Connected Merchant 2", () => {
    it("create-connector-for-cm2", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");
      const savedProfileId = globalState.get("profileId");
      const savedConnectorId = globalState.get("connectorId");

      globalState.set("merchantId", globalState.get("connectedMerchantId_2"));
      globalState.set("apiKey", globalState.get("apiKey_CM2"));
      globalState.set("profileId", globalState.get("profileId_CM2"));
      globalState.set("connectorId", "stripe");

      cy.createConnectorCallTest(
        "payment_processor",
        fixtures.createConnectorBody,
        payment_methods_enabled,
        globalState
      );

      cy.then(() => {
        globalState.set(
          "connectorId_CM2",
          globalState.get("merchantConnectorId")
        );
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
        globalState.set("profileId", savedProfileId);
        globalState.set("connectorId", savedConnectorId);
      });
    });

    it("retrieve-connector-for-cm2", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");
      const savedMerchantConnectorId = globalState.get("merchantConnectorId");
      const savedConnectorId = globalState.get("connectorId");

      globalState.set("merchantId", globalState.get("connectedMerchantId_2"));
      globalState.set("apiKey", globalState.get("apiKey_CM2"));
      globalState.set("merchantConnectorId", globalState.get("connectorId_CM2"));
      globalState.set("connectorId", "stripe");

      cy.connectorRetrieveCall(globalState);

      cy.then(() => {
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
        globalState.set("merchantConnectorId", savedMerchantConnectorId);
        globalState.set("connectorId", savedConnectorId);
      });
    });
  });

  context("Verify Connectors Are Separate", () => {
    it("verify-cm1-and-cm2-have-different-connectors", () => {
      const connectorIdCM1 = globalState.get("connectorId_CM1");
      const connectorIdCM2 = globalState.get("connectorId_CM2");

      expect(connectorIdCM1).to.not.equal(connectorIdCM2);
    });
  });
});
