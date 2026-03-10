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

      cy.createPlatformMerchantCallTest(merchantCreateBody, globalState).then(
        (response) => {
          expect(response.status).to.equal(200);
          expect(response.body).to.have.property(
            "merchant_account_type",
            "platform"
          );
          expect(response.body).to.have.property("organization_id");
        }
      );
    });

    it("retrieve-platform-merchant", () => {
      cy.merchantRetrieveCall(globalState);
    });

    it("create-api-key-for-platform-merchant", () => {
      cy.apiKeyCreateTest(fixtures.apiKeyCreateBody, globalState);
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

      cy.createConnectedMerchantCallTest(merchantCreateBody, globalState).then(
        (response) => {
          expect(response.status).to.equal(200);
          expect(response.body).to.have.property(
            "merchant_account_type",
            "connected"
          );
          globalState.set("connectedMerchantId_1", response.body.merchant_id);
          globalState.set("profileId_CM1", response.body.default_profile);
        }
      );
    });

    it("create-api-key-for-connected-merchant-1", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");
      globalState.set("merchantId", globalState.get("connectedMerchantId_1"));

      cy.apiKeyCreateTest(fixtures.apiKeyCreateBody, globalState);

      cy.then(() => {
        globalState.set("apiKey_CM1", globalState.get("apiKey"));
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });
  });

  context("Create Connected Merchant 2", () => {
    it("create-connected-merchant-2", () => {
      const merchantCreateBody = {
        ...fixtures.merchantCreateBody,
        merchant_account_type: "connected",
        merchant_id: `cypress_connected_merchant_2_${Date.now()}`,
        merchant_name: "Connected Merchant 2",
        organization_id: globalState.get("organizationId"),
      };

      cy.createConnectedMerchantCallTest(merchantCreateBody, globalState).then(
        (response) => {
          expect(response.status).to.equal(200);
          expect(response.body).to.have.property(
            "merchant_account_type",
            "connected"
          );
          globalState.set("connectedMerchantId_2", response.body.merchant_id);
          globalState.set("profileId_CM2", response.body.default_profile);
        }
      );
    });

    it("create-api-key-for-connected-merchant-2", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");
      globalState.set("merchantId", globalState.get("connectedMerchantId_2"));

      cy.apiKeyCreateTest(fixtures.apiKeyCreateBody, globalState);

      cy.then(() => {
        globalState.set("apiKey_CM2", globalState.get("apiKey"));
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
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

      cy.createConnectedMerchantCallTest(merchantCreateBody, globalState).then(
        (response) => {
          expect(response.status).to.equal(200);
          expect(response.body).to.have.property(
            "merchant_account_type",
            "standard"
          );
          globalState.set("standardMerchantId", response.body.merchant_id);
          globalState.set("profileId_SM", response.body.default_profile);
        }
      );
    });

    it("create-api-key-for-standard-merchant", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");
      globalState.set("merchantId", globalState.get("standardMerchantId"));

      cy.apiKeyCreateTest(fixtures.apiKeyCreateBody, globalState);

      cy.then(() => {
        globalState.set("apiKey_SM", globalState.get("apiKey"));
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });
  });

  context("Verify Merchants in Organization", () => {
    it("list-merchants-includes-all-merchants", () => {
      cy.merchantListByOrgCallTest(globalState).then((response) => {
        expect(response.status).to.equal(200);
        expect(response.body).to.be.an("array");
        expect(response.body.length).to.be.at.least(4);

        const platformMerchant = response.body.find(
          (m) => m.merchant_id === globalState.get("platformMerchantId")
        );
        expect(platformMerchant).to.exist;
        expect(platformMerchant.merchant_account_type).to.equal("platform");

        const cm1 = response.body.find(
          (m) => m.merchant_id === globalState.get("connectedMerchantId_1")
        );
        expect(cm1).to.exist;
        expect(cm1.merchant_account_type).to.equal("connected");

        const cm2 = response.body.find(
          (m) => m.merchant_id === globalState.get("connectedMerchantId_2")
        );
        expect(cm2).to.exist;
        expect(cm2.merchant_account_type).to.equal("connected");

        const sm = response.body.find(
          (m) => m.merchant_id === globalState.get("standardMerchantId")
        );
        expect(sm).to.exist;
        expect(sm.merchant_account_type).to.equal("standard");
      });
    });
  });
});
