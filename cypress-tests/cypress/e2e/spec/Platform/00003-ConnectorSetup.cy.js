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

      globalState.set("merchantId", globalState.get("connectedMerchantId1"));
      globalState.set("apiKey", globalState.get("apiKeyCm1"));
      globalState.set("profileId", globalState.get("profileIdCm1"));

      cy.createConnectorCallTest(
        "payment_processor",
        fixtures.createConnectorBody,
        payment_methods_enabled,
        globalState
      );

      cy.then(() => {
        globalState.set(
          "connectorIdCm1",
          globalState.get("merchantConnectorId")
        );
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
        globalState.set("profileId", savedProfileId);
      });
    });

    it("retrieve-connector-for-cm1", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");
      const savedMerchantConnectorId = globalState.get("merchantConnectorId");

      globalState.set("merchantId", globalState.get("connectedMerchantId1"));
      globalState.set("apiKey", globalState.get("apiKeyCm1"));
      globalState.set("merchantConnectorId", globalState.get("connectorIdCm1"));

      cy.connectorRetrieveCall(globalState);

      cy.then(() => {
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
        globalState.set("merchantConnectorId", savedMerchantConnectorId);
      });
    });
  });

  context("Create Connector for Connected Merchant 2", () => {
    it("create-connector-for-cm2", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");
      const savedProfileId = globalState.get("profileId");

      globalState.set("merchantId", globalState.get("connectedMerchantId2"));
      globalState.set("apiKey", globalState.get("apiKeyCm2"));
      globalState.set("profileId", globalState.get("profileIdCm2"));

      cy.createConnectorCallTest(
        "payment_processor",
        fixtures.createConnectorBody,
        payment_methods_enabled,
        globalState
      );

      cy.then(() => {
        globalState.set(
          "connectorIdCm2",
          globalState.get("merchantConnectorId")
        );
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
        globalState.set("profileId", savedProfileId);
      });
    });

    it("retrieve-connector-for-cm2", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");
      const savedMerchantConnectorId = globalState.get("merchantConnectorId");

      globalState.set("merchantId", globalState.get("connectedMerchantId2"));
      globalState.set("apiKey", globalState.get("apiKeyCm2"));
      globalState.set("merchantConnectorId", globalState.get("connectorIdCm2"));

      cy.connectorRetrieveCall(globalState);

      cy.then(() => {
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
        globalState.set("merchantConnectorId", savedMerchantConnectorId);
      });
    });
  });

  context("Create Connector for Standard Merchant", () => {
    it("create-connector-for-standard-merchant", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");
      const savedProfileId = globalState.get("profileId");

      globalState.set("merchantId", globalState.get("standardMerchantId"));
      globalState.set("apiKey", globalState.get("apiKeySm"));
      globalState.set("profileId", globalState.get("profileIdSm"));

      cy.createConnectorCallTest(
        "payment_processor",
        fixtures.createConnectorBody,
        payment_methods_enabled,
        globalState
      );

      cy.then(() => {
        globalState.set(
          "connectorIdSm",
          globalState.get("merchantConnectorId")
        );
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
        globalState.set("profileId", savedProfileId);
      });
    });

    it("retrieve-connector-for-standard-merchant", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");
      const savedMerchantConnectorId = globalState.get("merchantConnectorId");

      globalState.set("merchantId", globalState.get("standardMerchantId"));
      globalState.set("apiKey", globalState.get("apiKeySm"));
      globalState.set("merchantConnectorId", globalState.get("connectorIdSm"));

      cy.connectorRetrieveCall(globalState);

      cy.then(() => {
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
        globalState.set("merchantConnectorId", savedMerchantConnectorId);
      });
    });
  });

  context("Platform Creates Connector for Connected Merchant 1", () => {
    it("platform-create-connector-for-cm1", () => {
      const savedMerchantId = globalState.get("merchantId");

      globalState.set("merchantId", globalState.get("connectedMerchantId1"));

      const connectorId = globalState.get("connectorId");

      cy.createConnectorWithHeaderCall(
        {
          connector_type: "payment_processor",
          connector_name: connectorId,
          connector_label: `${connectorId}_platform_for_cm1`,
          ...fixtures.createConnectorBody,
          payment_methods_enabled,
          profile_id: globalState.get("profileIdCm1"),
        },
        globalState.get("apiKey"),
        globalState.get("connectedMerchantId1"),
        globalState,
        200
      );

      cy.then(() => {
        globalState.set("merchantId", savedMerchantId);
      });
    });
  });

  context("Verify Connectors Are Separate", () => {
    it("verify-cm1-and-cm2-have-different-connectors", () => {
      const connectorIdCm1 = globalState.get("connectorIdCm1");
      const connectorIdCm2 = globalState.get("connectorIdCm2");

      expect(connectorIdCm1).to.not.equal(connectorIdCm2);
    });
  });

  context("Platform Cannot Create Connector for Standard Merchant", () => {
    it("platform-cannot-create-connector-for-standard-merchant", () => {
      const savedMerchantId = globalState.get("merchantId");

      globalState.set("merchantId", globalState.get("standardMerchantId"));

      const connectorId = globalState.get("connectorId");

      cy.createConnectorWithHeaderCall(
        {
          connector_type: "payment_processor",
          connector_name: connectorId,
          connector_label: `${connectorId}_platform_for_sm`,
          ...fixtures.createConnectorBody,
          payment_methods_enabled,
          profile_id: globalState.get("profileIdSm"),
        },
        globalState.get("apiKey"),
        globalState.get("standardMerchantId"),
        globalState,
        401
      );

      cy.then(() => {
        globalState.set("merchantId", savedMerchantId);
      });
    });
  });

  context("Platform Merchant Cannot Create Connector", () => {
    it("platform-merchant-cannot-create-connector", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedProfileId = globalState.get("profileId");

      globalState.set("merchantId", globalState.get("platformMerchantId"));
      globalState.set("profileId", globalState.get("profileId"));

      const connectorId = globalState.get("connectorId");

      cy.createConnectorCallTest(
        "payment_processor",
        {
          connector_name: connectorId,
          connector_label: `${connectorId}_platform_test`,
          ...fixtures.createConnectorBody,
          payment_methods_enabled,
        }, // createConnectorBody
        payment_methods_enabled,
        globalState,
        "profile", // profilePrefix
        "merchantConnector", // mcaPrefix
        400 // expectedStatus
      );

      cy.then(() => {
        globalState.set("merchantId", savedMerchantId);
        globalState.set("profileId", savedProfileId);
      });
    });
  });
});
