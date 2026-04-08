import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";

let globalState;

describe("Platform Setup & Connected Merchant Onboarding", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Create Platform Merchant", () => {
    it("create-platform-merchant", () => {
      const merchantCreateBody = {
        ...fixtures.merchantCreateBody,
        merchant_account_type: "platform",
      };

      cy.merchantCreateCallTest(merchantCreateBody, globalState, {
        expectedMerchantAccountType: "platform",
      });
      cy.then(() => {
        globalState.set("platformMerchantId", globalState.get("merchantId"));
        globalState.set(
          "platformPublishableKey",
          globalState.get("publishableKey")
        );
      });
    });

    it("retrieve-platform-merchant", () => {
      cy.merchantRetrieveCall(globalState);
    });

    it("create-api-key-for-platform-merchant", () => {
      const savedMerchantId = globalState.get("merchantId");
      globalState.set("merchantId", globalState.get("platformMerchantId"));

      cy.apiKeyCreateTest(fixtures.apiKeyCreateBody, globalState);

      cy.then(() => {
        globalState.set("platformApiKey", globalState.get("apiKey"));
        globalState.set("merchantId", savedMerchantId);
      });
    });
  });

  context("Create Connected Merchant 1", () => {
    it("create-connected-merchant-1", () => {
      const merchantCreateBody = {
        ...fixtures.merchantCreateBody,
        merchant_account_type: "connected",
        merchant_id: `cypress_connected_merchant_1_${Date.now()}`,
        merchant_name: "Connected Merchant 1",
        organization_id: globalState.get("organizationId"),
      };

      cy.merchantCreateCallTest(merchantCreateBody, globalState, {
        expectedMerchantAccountType: "connected",
        merchantIdStateKey: "connectedMerchantId1",
        profileIdStateKey: "profileIdCm1",
        publishableKeyStateKey: "publishableKeyCm1",
      });
    });

    it("create-api-key-for-connected-merchant-1", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");
      globalState.set("merchantId", globalState.get("connectedMerchantId1"));

      cy.apiKeyCreateTest(fixtures.apiKeyCreateBody, globalState);

      cy.then(() => {
        globalState.set("apiKeyCm1", globalState.get("apiKey"));
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });
  });

  context("Create Connected Merchant 2 using Platform API Key", () => {
    it("create-connected-merchant-2-using-platform-api-key", () => {
      const savedAdminApiKey = globalState.get("adminApiKey");
      globalState.set("adminApiKey", globalState.get("apiKey"));

      const merchantCreateBody = {
        ...fixtures.merchantCreateBody,
        merchant_account_type: "connected",
        merchant_id: `cypress_connected_merchant_2_${Date.now()}`,
        merchant_name: "Connected Merchant 2",
        organization_id: globalState.get("organizationId"),
      };

      cy.merchantCreateCallTest(merchantCreateBody, globalState, {
        expectedMerchantAccountType: "connected",
        merchantIdStateKey: "connectedMerchantId2",
        profileIdStateKey: "profileIdCm2",
        publishableKeyStateKey: "publishableKeyCm2",
      });

      cy.then(() => {
        globalState.set("adminApiKey", savedAdminApiKey);
      });
    });

    it("create-api-key-for-connected-merchant-2-platform-key", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");
      const savedAdminApiKey = globalState.get("adminApiKey");
      globalState.set("merchantId", globalState.get("connectedMerchantId2"));
      globalState.set("adminApiKey", savedApiKey);

      cy.apiKeyCreateTest(fixtures.apiKeyCreateBody, globalState);

      cy.then(() => {
        globalState.set("apiKeyCm2", globalState.get("apiKey"));
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
        globalState.set("adminApiKey", savedAdminApiKey);
      });
    });
  });

  context("Create Standard Merchant", () => {
    it("create-standard-merchant", () => {
      const merchantCreateBody = {
        ...fixtures.merchantCreateBody,
        merchant_id: `cypress_standard_merchant_${Date.now()}`,
        merchant_name: "Standard Merchant",
        organization_id: globalState.get("organizationId"),
      };

      cy.merchantCreateCallTest(merchantCreateBody, globalState, {
        expectedMerchantAccountType: "standard",
        merchantIdStateKey: "standardMerchantId",
        profileIdStateKey: "profileIdSm",
      });
    });

    it("create-api-key-for-standard-merchant", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");
      globalState.set("merchantId", globalState.get("standardMerchantId"));

      cy.apiKeyCreateTest(fixtures.apiKeyCreateBody, globalState);

      cy.then(() => {
        globalState.set("apiKeySm", globalState.get("apiKey"));
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });
  });

  context("Create Standard Merchant using Platform API Key", () => {
    it("create-standard-merchant-using-platform-api-key", () => {
      const savedAdminApiKey = globalState.get("adminApiKey");
      globalState.set("adminApiKey", globalState.get("apiKey"));

      const merchantCreateBody = {
        ...fixtures.merchantCreateBody,
        merchant_id: `cypress_standard_merchant_platform_${Date.now()}`,
        merchant_name: "Standard Merchant (Platform Key)",
        organization_id: globalState.get("organizationId"),
      };

      cy.merchantCreateCallTest(merchantCreateBody, globalState, {
        expectedMerchantAccountType: "standard",
        merchantIdStateKey: "standardMerchantIdPlatformKey",
        profileIdStateKey: "profileIdSmPlatformKey",
      });

      cy.then(() => {
        globalState.set("adminApiKey", savedAdminApiKey);
      });
    });

    it("create-api-key-for-standard-merchant-platform-key", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");
      const savedAdminApiKey = globalState.get("adminApiKey");
      globalState.set(
        "merchantId",
        globalState.get("standardMerchantIdPlatformKey")
      );
      globalState.set("adminApiKey", savedApiKey);

      cy.apiKeyCreateTest(fixtures.apiKeyCreateBody, globalState);

      cy.then(() => {
        globalState.set("apiKeySmPlatformKey", globalState.get("apiKey"));
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
        globalState.set("adminApiKey", savedAdminApiKey);
      });
    });
  });

  context("Verify Merchants in Organization", () => {
    it("list-merchants-includes-all-merchants", () => {
      cy.merchantListByOrgCall(globalState, [
        { merchantIdKey: "platformMerchantId", expectedType: "platform" },
        { merchantIdKey: "connectedMerchantId1", expectedType: "connected" },
        { merchantIdKey: "connectedMerchantId2", expectedType: "connected" },
        { merchantIdKey: "standardMerchantId", expectedType: "standard" },
        {
          merchantIdKey: "standardMerchantIdPlatformKey",
          expectedType: "standard",
        },
      ]);
    });
  });
});
