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
      globalState.set(
        "merchantConnectorId",
        globalState.get("connectorId_CM1")
      );
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
      globalState.set(
        "merchantConnectorId",
        globalState.get("connectorId_CM2")
      );
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

  context("Create Connector for Standard Merchant", () => {
    it("create-connector-for-standard-merchant", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");
      const savedProfileId = globalState.get("profileId");
      const savedConnectorId = globalState.get("connectorId");

      globalState.set("merchantId", globalState.get("standardMerchantId"));
      globalState.set("apiKey", globalState.get("apiKey_SM"));
      globalState.set("profileId", globalState.get("profileId_SM"));
      globalState.set("connectorId", "stripe");

      cy.createConnectorCallTest(
        "payment_processor",
        fixtures.createConnectorBody,
        payment_methods_enabled,
        globalState
      );

      cy.then(() => {
        globalState.set(
          "connectorId_SM",
          globalState.get("merchantConnectorId")
        );
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
        globalState.set("profileId", savedProfileId);
        globalState.set("connectorId", savedConnectorId);
      });
    });

    it("retrieve-connector-for-standard-merchant", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");
      const savedMerchantConnectorId = globalState.get("merchantConnectorId");
      const savedConnectorId = globalState.get("connectorId");

      globalState.set("merchantId", globalState.get("standardMerchantId"));
      globalState.set("apiKey", globalState.get("apiKey_SM"));
      globalState.set("merchantConnectorId", globalState.get("connectorId_SM"));
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

  context("Platform Creates Connector for Connected Merchant 1", () => {
    it("platform-create-connector-for-cm1", () => {
      const savedMerchantId = globalState.get("merchantId");

      globalState.set("merchantId", globalState.get("connectedMerchantId_1"));

      cy.createConnectorWithHeaderCallTest(
        {
          connector_type: "payment_processor",
          connector_name: "stripe",
          connector_label: "stripe_platform_for_cm1",
          ...fixtures.createConnectorBody,
          payment_methods_enabled,
          profile_id: globalState.get("profileId_CM1"),
        },
        globalState.get("apiKey"),
        globalState.get("connectedMerchantId_1"),
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
      const connectorIdCM1 = globalState.get("connectorId_CM1");
      const connectorIdCM2 = globalState.get("connectorId_CM2");

      expect(connectorIdCM1).to.not.equal(connectorIdCM2);
    });
  });

  context("Platform Cannot Create Connector for Standard Merchant", () => {
    it("platform-cannot-create-connector-for-standard-merchant", () => {
      const savedMerchantId = globalState.get("merchantId");

      globalState.set("merchantId", globalState.get("standardMerchantId"));

      cy.createConnectorWithHeaderCallTest(
        {
          connector_type: "payment_processor",
          connector_name: "stripe",
          connector_label: "stripe_platform_for_sm",
          ...fixtures.createConnectorBody,
          payment_methods_enabled,
          profile_id: globalState.get("profileId_SM"),
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

      cy.createConnectorCallTest(
        "payment_processor",
        {
          connector_name: "stripe",
          connector_label: "stripe_platform_test",
          ...fixtures.createConnectorBody,
          payment_methods_enabled,
        },
        payment_methods_enabled,
        globalState,
        "profile",
        "merchantConnector",
        400,
        globalState.get("apiKey")
      );

      cy.then(() => {
        globalState.set("merchantId", savedMerchantId);
        globalState.set("profileId", savedProfileId);
      });
    });
  });
});
