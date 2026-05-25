import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";

let globalState;

describe("Profile Setup for Merchants", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Platform Merchant Cannot Create Profile For Self", () => {
    it("platform-merchant-cannot-create-profile-for-self", () => {
      const savedMerchantId = globalState.get("merchantId");
      globalState.set("merchantId", globalState.get("platformMerchantId"));

      cy.createBusinessProfileTest(
        fixtures.businessProfile.bpCreate,
        globalState,
        "profile", // profilePrefix
        400 // expectedStatus
      );

      cy.then(() => {
        globalState.set("merchantId", savedMerchantId);
      });
    });
  });

  context("Connected Merchant 1 Creates Profile", () => {
    it("cm1-creates-profile", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      globalState.set("merchantId", globalState.get("connectedMerchantId1"));
      globalState.set("apiKey", globalState.get("apiKeyCm1"));

      cy.createBusinessProfileTest(
        fixtures.businessProfile.bpCreate,
        globalState
      );

      cy.then(() => {
        globalState.set("profileIdCm1New", globalState.get("profileId"));
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });
  });

  context("Platform Creates Profile For Connected Merchant 2", () => {
    it("platform-creates-profile-for-cm2", () => {
      const savedMerchantId = globalState.get("merchantId");
      globalState.set("merchantId", globalState.get("connectedMerchantId2"));

      cy.createBusinessProfileWithHeaderCall(
        fixtures.businessProfile.bpCreate,
        globalState.get("apiKey"),
        globalState.get("connectedMerchantId2"),
        globalState,
        200,
        "profileIdCm2New"
      );

      cy.then(() => {
        globalState.set("merchantId", savedMerchantId);
      });
    });
  });

  context("Platform Cannot Create Profile For Standard Merchant", () => {
    it("platform-cannot-create-profile-for-standard-merchant", () => {
      const savedMerchantId = globalState.get("merchantId");
      globalState.set("merchantId", globalState.get("standardMerchantId"));

      cy.createBusinessProfileTest(
        fixtures.businessProfile.bpCreate,
        globalState,
        "profile", // profilePrefix
        400 // expectedStatus
      );

      cy.then(() => {
        globalState.set("merchantId", savedMerchantId);
      });
    });
  });

  context("Standard Merchant Creates Profile", () => {
    it("standard-merchant-creates-profile", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      globalState.set("merchantId", globalState.get("standardMerchantId"));
      globalState.set("apiKey", globalState.get("apiKeySm"));

      cy.createBusinessProfileTest(
        fixtures.businessProfile.bpCreate,
        globalState
      );

      cy.then(() => {
        globalState.set("profileIdSmNew", globalState.get("profileId"));
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });
  });
});
