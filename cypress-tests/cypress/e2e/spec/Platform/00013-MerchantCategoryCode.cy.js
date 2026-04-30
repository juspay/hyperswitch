import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";

let globalState;

describe("Merchant Category Code (MCC) Tests", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  // ============================================
  // CONTEXT 1: Create Business Profile with MCC
  // ============================================
  context("Create Business Profile with MCC", () => {
    it("should create business profile with MCC 5411 on create", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      globalState.set("merchantId", globalState.get("connectedMerchantId1"));
      globalState.set("apiKey", globalState.get("apiKeyCm1"));

      const profileName = `mcc_profile_${Date.now()}_5411`;
      const createBody = {
        ...fixtures.businessProfile.bpCreate,
        profile_name: profileName,
        merchant_category_code: "5411",
      };

      cy.createBusinessProfileTest(createBody, globalState, "mcc5411");

      cy.then(() => {
        globalState.set("mcc5411MerchantId", globalState.get("merchantId"));
        globalState.set("mcc5411ApiKey", globalState.get("apiKey"));
        globalState.set("mcc5411ProfileId", globalState.get("mcc5411Id"));
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });

    it("should create business profile with MCC 8111 on create", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      globalState.set("merchantId", globalState.get("connectedMerchantId1"));
      globalState.set("apiKey", globalState.get("apiKeyCm1"));

      const profileName = `mcc_profile_${Date.now()}_8111`;
      const createBody = {
        ...fixtures.businessProfile.bpCreate,
        profile_name: profileName,
        merchant_category_code: "8111",
      };

      cy.createBusinessProfileTest(createBody, globalState, "mcc8111");

      cy.then(() => {
        globalState.set("mcc8111MerchantId", globalState.get("merchantId"));
        globalState.set("mcc8111ApiKey", globalState.get("apiKey"));
        globalState.set("mcc8111ProfileId", globalState.get("mcc8111Id"));
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });

    it("should create business profile with MCC 7011 on create", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      globalState.set("merchantId", globalState.get("connectedMerchantId1"));
      globalState.set("apiKey", globalState.get("apiKeyCm1"));

      const profileName = `mcc_profile_${Date.now()}_7011`;
      const createBody = {
        ...fixtures.businessProfile.bpCreate,
        profile_name: profileName,
        merchant_category_code: "7011",
      };

      cy.createBusinessProfileTest(createBody, globalState, "mcc7011");

      cy.then(() => {
        globalState.set("mcc7011MerchantId", globalState.get("merchantId"));
        globalState.set("mcc7011ApiKey", globalState.get("apiKey"));
        globalState.set("mcc7011ProfileId", globalState.get("mcc7011Id"));
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });

    it("should create business profile with MCC 5912 on create", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      globalState.set("merchantId", globalState.get("connectedMerchantId1"));
      globalState.set("apiKey", globalState.get("apiKeyCm1"));

      const profileName = `mcc_profile_${Date.now()}_5912`;
      const createBody = {
        ...fixtures.businessProfile.bpCreate,
        profile_name: profileName,
        merchant_category_code: "5912",
      };

      cy.createBusinessProfileTest(createBody, globalState, "mcc5912");

      cy.then(() => {
        globalState.set("mcc5912MerchantId", globalState.get("merchantId"));
        globalState.set("mcc5912ApiKey", globalState.get("apiKey"));
        globalState.set("mcc5912ProfileId", globalState.get("mcc5912Id"));
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });

    it("should create business profile with MCC 5331 on create", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      globalState.set("merchantId", globalState.get("connectedMerchantId1"));
      globalState.set("apiKey", globalState.get("apiKeyCm1"));

      const profileName = `mcc_profile_${Date.now()}_5331`;
      const createBody = {
        ...fixtures.businessProfile.bpCreate,
        profile_name: profileName,
        merchant_category_code: "5331",
      };

      cy.createBusinessProfileTest(createBody, globalState, "mcc5331");

      cy.then(() => {
        globalState.set("mcc5331MerchantId", globalState.get("merchantId"));
        globalState.set("mcc5331ApiKey", globalState.get("apiKey"));
        globalState.set("mcc5331ProfileId", globalState.get("mcc5331Id"));
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });

    it("should create business profile with MCC 7832 on create", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      globalState.set("merchantId", globalState.get("connectedMerchantId1"));
      globalState.set("apiKey", globalState.get("apiKeyCm1"));

      const profileName = `mcc_profile_${Date.now()}_7832`;
      const createBody = {
        ...fixtures.businessProfile.bpCreate,
        profile_name: profileName,
        merchant_category_code: "7832",
      };

      cy.createBusinessProfileTest(createBody, globalState, "mcc7832");

      cy.then(() => {
        globalState.set("mcc7832MerchantId", globalState.get("merchantId"));
        globalState.set("mcc7832ApiKey", globalState.get("apiKey"));
        globalState.set("mcc7832ProfileId", globalState.get("mcc7832Id"));
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });

    it("should create business profile with MCC 4814 on create", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      globalState.set("merchantId", globalState.get("connectedMerchantId1"));
      globalState.set("apiKey", globalState.get("apiKeyCm1"));

      const profileName = `mcc_profile_${Date.now()}_4814`;
      const createBody = {
        ...fixtures.businessProfile.bpCreate,
        profile_name: profileName,
        merchant_category_code: "4814",
      };

      cy.createBusinessProfileTest(createBody, globalState, "mcc4814");

      cy.then(() => {
        globalState.set("mcc4814MerchantId", globalState.get("merchantId"));
        globalState.set("mcc4814ApiKey", globalState.get("apiKey"));
        globalState.set("mcc4814ProfileId", globalState.get("mcc4814Id"));
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });
  });

  // ============================================
  // CONTEXT 2: Retrieve and Verify MCC
  // ============================================
  context("Retrieve and Verify MCC", () => {
    it("should verify MCC 5411 persisted on business profile", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      globalState.set("merchantId", globalState.get("mcc5411MerchantId"));
      globalState.set("apiKey", globalState.get("mcc5411ApiKey"));
      globalState.set("profileId", globalState.get("mcc5411ProfileId"));

      cy.verifyBusinessProfileMcc(globalState, null, "5411");

      cy.then(() => {
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });

    it("should verify MCC 8111 persisted on business profile", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      globalState.set("merchantId", globalState.get("mcc8111MerchantId"));
      globalState.set("apiKey", globalState.get("mcc8111ApiKey"));
      globalState.set("profileId", globalState.get("mcc8111ProfileId"));

      cy.verifyBusinessProfileMcc(globalState, null, "8111");

      cy.then(() => {
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });

    it("should verify MCC 7011 persisted on business profile", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      globalState.set("merchantId", globalState.get("mcc7011MerchantId"));
      globalState.set("apiKey", globalState.get("mcc7011ApiKey"));
      globalState.set("profileId", globalState.get("mcc7011ProfileId"));

      cy.verifyBusinessProfileMcc(globalState, null, "7011");

      cy.then(() => {
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });

    it("should verify MCC 5912 persisted on business profile", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      globalState.set("merchantId", globalState.get("mcc5912MerchantId"));
      globalState.set("apiKey", globalState.get("mcc5912ApiKey"));
      globalState.set("profileId", globalState.get("mcc5912ProfileId"));

      cy.verifyBusinessProfileMcc(globalState, null, "5912");

      cy.then(() => {
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });

    it("should verify MCC 5331 persisted on business profile", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      globalState.set("merchantId", globalState.get("mcc5331MerchantId"));
      globalState.set("apiKey", globalState.get("mcc5331ApiKey"));
      globalState.set("profileId", globalState.get("mcc5331ProfileId"));

      cy.verifyBusinessProfileMcc(globalState, null, "5331");

      cy.then(() => {
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });

    it("should verify MCC 7832 persisted on business profile", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      globalState.set("merchantId", globalState.get("mcc7832MerchantId"));
      globalState.set("apiKey", globalState.get("mcc7832ApiKey"));
      globalState.set("profileId", globalState.get("mcc7832ProfileId"));

      cy.verifyBusinessProfileMcc(globalState, null, "7832");

      cy.then(() => {
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });

    it("should verify MCC 4814 persisted on business profile", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      globalState.set("merchantId", globalState.get("mcc4814MerchantId"));
      globalState.set("apiKey", globalState.get("mcc4814ApiKey"));
      globalState.set("profileId", globalState.get("mcc4814ProfileId"));

      cy.verifyBusinessProfileMcc(globalState, null, "4814");

      cy.then(() => {
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });
  });

  // ============================================
  // CONTEXT 3: Update Business Profile MCC
  // ============================================
  context("Update Business Profile MCC", () => {
    it("should update MCC from 5411 to 5812 on business profile", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      globalState.set("merchantId", globalState.get("mcc5411MerchantId"));
      globalState.set("apiKey", globalState.get("mcc5411ApiKey"));

      const updateBody = {
        ...fixtures.businessProfile.bpUpdate,
        merchant_category_code: "5812",
      };

      cy.UpdateBusinessProfileTest(
        updateBody,
        true,
        true,
        true,
        true,
        true,
        globalState,
        "mcc5411"
      );

      cy.verifyBusinessProfileMcc(
        globalState,
        globalState.get("mcc5411ProfileId"),
        "5812"
      );

      cy.then(() => {
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });

    it("should update MCC from 8111 to 5541 on business profile", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      globalState.set("merchantId", globalState.get("mcc8111MerchantId"));
      globalState.set("apiKey", globalState.get("mcc8111ApiKey"));

      const updateBody = {
        ...fixtures.businessProfile.bpUpdate,
        merchant_category_code: "5541",
      };

      cy.UpdateBusinessProfileTest(
        updateBody,
        true,
        true,
        true,
        true,
        true,
        globalState,
        "mcc8111"
      );

      cy.verifyBusinessProfileMcc(
        globalState,
        globalState.get("mcc8111ProfileId"),
        "5541"
      );

      cy.then(() => {
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });

    it("should update MCC from 7011 to 9999 on business profile", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      globalState.set("merchantId", globalState.get("mcc7011MerchantId"));
      globalState.set("apiKey", globalState.get("mcc7011ApiKey"));

      const updateBody = {
        ...fixtures.businessProfile.bpUpdate,
        merchant_category_code: "9999",
      };

      cy.UpdateBusinessProfileTest(
        updateBody,
        true,
        true,
        true,
        true,
        true,
        globalState,
        "mcc7011"
      );

      cy.verifyBusinessProfileMcc(
        globalState,
        globalState.get("mcc7011ProfileId"),
        "9999"
      );

      cy.then(() => {
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });

    it("should update MCC from 5912 to 5333 on business profile", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      globalState.set("merchantId", globalState.get("mcc5912MerchantId"));
      globalState.set("apiKey", globalState.get("mcc5912ApiKey"));

      const updateBody = {
        ...fixtures.businessProfile.bpUpdate,
        merchant_category_code: "5333",
      };

      cy.UpdateBusinessProfileTest(
        updateBody,
        true,
        true,
        true,
        true,
        true,
        globalState,
        "mcc5912"
      );

      cy.verifyBusinessProfileMcc(
        globalState,
        globalState.get("mcc5912ProfileId"),
        "5333"
      );

      cy.then(() => {
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });

    it("should update MCC from 5331 to 5412 on business profile", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      globalState.set("merchantId", globalState.get("mcc5331MerchantId"));
      globalState.set("apiKey", globalState.get("mcc5331ApiKey"));

      const updateBody = {
        ...fixtures.businessProfile.bpUpdate,
        merchant_category_code: "5412",
      };

      cy.UpdateBusinessProfileTest(
        updateBody,
        true,
        true,
        true,
        true,
        true,
        globalState,
        "mcc5331"
      );

      cy.verifyBusinessProfileMcc(
        globalState,
        globalState.get("mcc5331ProfileId"),
        "5412"
      );

      cy.then(() => {
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });

    it("should update MCC from 7832 to 7829 on business profile", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      globalState.set("merchantId", globalState.get("mcc7832MerchantId"));
      globalState.set("apiKey", globalState.get("mcc7832ApiKey"));

      const updateBody = {
        ...fixtures.businessProfile.bpUpdate,
        merchant_category_code: "7829",
      };

      cy.UpdateBusinessProfileTest(
        updateBody,
        true,
        true,
        true,
        true,
        true,
        globalState,
        "mcc7832"
      );

      cy.verifyBusinessProfileMcc(
        globalState,
        globalState.get("mcc7832ProfileId"),
        "7829"
      );

      cy.then(() => {
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });

    it("should update MCC from 4814 to 4899 on business profile", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      globalState.set("merchantId", globalState.get("mcc4814MerchantId"));
      globalState.set("apiKey", globalState.get("mcc4814ApiKey"));

      const updateBody = {
        ...fixtures.businessProfile.bpUpdate,
        merchant_category_code: "4899",
      };

      cy.UpdateBusinessProfileTest(
        updateBody,
        true,
        true,
        true,
        true,
        true,
        globalState,
        "mcc4814"
      );

      cy.verifyBusinessProfileMcc(
        globalState,
        globalState.get("mcc4814ProfileId"),
        "4899"
      );

      cy.then(() => {
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });
  });

  // ============================================
  // CONTEXT 4: Multiple MCC Update Cycles
  // ============================================
  context("Multiple MCC Update Cycles", () => {
    it("should update MCC from 5812 back to 5411", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      globalState.set("merchantId", globalState.get("mcc5411MerchantId"));
      globalState.set("apiKey", globalState.get("mcc5411ApiKey"));

      const updateBody = {
        ...fixtures.businessProfile.bpUpdate,
        merchant_category_code: "5411",
      };

      cy.UpdateBusinessProfileTest(
        updateBody,
        true,
        true,
        true,
        true,
        true,
        globalState,
        "mcc5411"
      );

      cy.verifyBusinessProfileMcc(
        globalState,
        globalState.get("mcc5411ProfileId"),
        "5411"
      );

      cy.then(() => {
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });

    it("should update MCC from 5541 back to 8111", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      globalState.set("merchantId", globalState.get("mcc8111MerchantId"));
      globalState.set("apiKey", globalState.get("mcc8111ApiKey"));

      const updateBody = {
        ...fixtures.businessProfile.bpUpdate,
        merchant_category_code: "8111",
      };

      cy.UpdateBusinessProfileTest(
        updateBody,
        true,
        true,
        true,
        true,
        true,
        globalState,
        "mcc8111"
      );

      cy.verifyBusinessProfileMcc(
        globalState,
        globalState.get("mcc8111ProfileId"),
        "8111"
      );

      cy.then(() => {
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });

    it("should update MCC from 9999 back to 7011", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      globalState.set("merchantId", globalState.get("mcc7011MerchantId"));
      globalState.set("apiKey", globalState.get("mcc7011ApiKey"));

      const updateBody = {
        ...fixtures.businessProfile.bpUpdate,
        merchant_category_code: "7011",
      };

      cy.UpdateBusinessProfileTest(
        updateBody,
        true,
        true,
        true,
        true,
        true,
        globalState,
        "mcc7011"
      );

      cy.verifyBusinessProfileMcc(
        globalState,
        globalState.get("mcc7011ProfileId"),
        "7011"
      );

      cy.then(() => {
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });

    it("should update MCC from 5333 to 5921 and verify", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      globalState.set("merchantId", globalState.get("mcc5912MerchantId"));
      globalState.set("apiKey", globalState.get("mcc5912ApiKey"));

      const updateBody = {
        ...fixtures.businessProfile.bpUpdate,
        merchant_category_code: "5921",
      };

      cy.UpdateBusinessProfileTest(
        updateBody,
        true,
        true,
        true,
        true,
        true,
        globalState,
        "mcc5912"
      );

      cy.verifyBusinessProfileMcc(
        globalState,
        globalState.get("mcc5912ProfileId"),
        "5921"
      );

      cy.then(() => {
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });

    it("should update MCC from 5412 to 5300 and verify", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      globalState.set("merchantId", globalState.get("mcc5331MerchantId"));
      globalState.set("apiKey", globalState.get("mcc5331ApiKey"));

      const updateBody = {
        ...fixtures.businessProfile.bpUpdate,
        merchant_category_code: "5300",
      };

      cy.UpdateBusinessProfileTest(
        updateBody,
        true,
        true,
        true,
        true,
        true,
        globalState,
        "mcc5331"
      );

      cy.verifyBusinessProfileMcc(
        globalState,
        globalState.get("mcc5331ProfileId"),
        "5300"
      );

      cy.then(() => {
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });

    it("should update MCC from 7829 to 7996 and verify", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      globalState.set("merchantId", globalState.get("mcc7832MerchantId"));
      globalState.set("apiKey", globalState.get("mcc7832ApiKey"));

      const updateBody = {
        ...fixtures.businessProfile.bpUpdate,
        merchant_category_code: "7996",
      };

      cy.UpdateBusinessProfileTest(
        updateBody,
        true,
        true,
        true,
        true,
        true,
        globalState,
        "mcc7832"
      );

      cy.verifyBusinessProfileMcc(
        globalState,
        globalState.get("mcc7832ProfileId"),
        "7996"
      );

      cy.then(() => {
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });

    it("should update MCC from 4899 to 4816 and verify", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      globalState.set("merchantId", globalState.get("mcc4814MerchantId"));
      globalState.set("apiKey", globalState.get("mcc4814ApiKey"));

      const updateBody = {
        ...fixtures.businessProfile.bpUpdate,
        merchant_category_code: "4816",
      };

      cy.UpdateBusinessProfileTest(
        updateBody,
        true,
        true,
        true,
        true,
        true,
        globalState,
        "mcc4814"
      );

      cy.verifyBusinessProfileMcc(
        globalState,
        globalState.get("mcc4814ProfileId"),
        "4816"
      );

      cy.then(() => {
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });
  });

  // ============================================
  // CONTEXT 5: Additional MCC Validation Tests
  // ============================================
  context("Additional MCC Validation Tests", () => {
    it("should create profile with MCC 4112 and verify persistence", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      globalState.set("merchantId", globalState.get("connectedMerchantId1"));
      globalState.set("apiKey", globalState.get("apiKeyCm1"));

      const profileName = `mcc_profile_${Date.now()}_4112`;
      const createBody = {
        ...fixtures.businessProfile.bpCreate,
        profile_name: profileName,
        merchant_category_code: "4112",
      };

      cy.createBusinessProfileTest(createBody, globalState, "mcc4112");
      cy.verifyBusinessProfileMcc(globalState, null, "4112");

      cy.then(() => {
        globalState.set("mcc4112MerchantId", globalState.get("merchantId"));
        globalState.set("mcc4112ApiKey", globalState.get("apiKey"));
        globalState.set("mcc4112ProfileId", globalState.get("mcc4112Id"));
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });

    it("should create profile with MCC 4225 and verify persistence", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      globalState.set("merchantId", globalState.get("connectedMerchantId1"));
      globalState.set("apiKey", globalState.get("apiKeyCm1"));

      const profileName = `mcc_profile_${Date.now()}_4225`;
      const createBody = {
        ...fixtures.businessProfile.bpCreate,
        profile_name: profileName,
        merchant_category_code: "4225",
      };

      cy.createBusinessProfileTest(createBody, globalState, "mcc4225");
      cy.verifyBusinessProfileMcc(globalState, null, "4225");

      cy.then(() => {
        globalState.set("mcc4225MerchantId", globalState.get("merchantId"));
        globalState.set("mcc4225ApiKey", globalState.get("apiKey"));
        globalState.set("mcc4225ProfileId", globalState.get("mcc4225Id"));
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });

    it("should create profile with MCC 5542 and verify persistence", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      globalState.set("merchantId", globalState.get("connectedMerchantId1"));
      globalState.set("apiKey", globalState.get("apiKeyCm1"));

      const profileName = `mcc_profile_${Date.now()}_5542`;
      const createBody = {
        ...fixtures.businessProfile.bpCreate,
        profile_name: profileName,
        merchant_category_code: "5542",
      };

      cy.createBusinessProfileTest(createBody, globalState, "mcc5542");
      cy.verifyBusinessProfileMcc(globalState, null, "5542");

      cy.then(() => {
        globalState.set("mcc5542MerchantId", globalState.get("merchantId"));
        globalState.set("mcc5542ApiKey", globalState.get("apiKey"));
        globalState.set("mcc5542ProfileId", globalState.get("mcc5542Id"));
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });

    it("should update MCC 4112 to 7399 and verify", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      globalState.set("merchantId", globalState.get("mcc4112MerchantId"));
      globalState.set("apiKey", globalState.get("mcc4112ApiKey"));

      const updateBody = {
        ...fixtures.businessProfile.bpUpdate,
        merchant_category_code: "7399",
      };

      cy.UpdateBusinessProfileTest(
        updateBody,
        true,
        true,
        true,
        true,
        true,
        globalState,
        "mcc4112"
      );

      cy.verifyBusinessProfileMcc(
        globalState,
        globalState.get("mcc4112ProfileId"),
        "7399"
      );

      cy.then(() => {
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });

    it("should update MCC 4225 to 5499 and verify", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      globalState.set("merchantId", globalState.get("mcc4225MerchantId"));
      globalState.set("apiKey", globalState.get("mcc4225ApiKey"));

      const updateBody = {
        ...fixtures.businessProfile.bpUpdate,
        merchant_category_code: "5499",
      };

      cy.UpdateBusinessProfileTest(
        updateBody,
        true,
        true,
        true,
        true,
        true,
        globalState,
        "mcc4225"
      );

      cy.verifyBusinessProfileMcc(
        globalState,
        globalState.get("mcc4225ProfileId"),
        "5499"
      );

      cy.then(() => {
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });

    it("should update MCC 5542 to 7538 and verify", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      globalState.set("merchantId", globalState.get("mcc5542MerchantId"));
      globalState.set("apiKey", globalState.get("mcc5542ApiKey"));

      const updateBody = {
        ...fixtures.businessProfile.bpUpdate,
        merchant_category_code: "7538",
      };

      cy.UpdateBusinessProfileTest(
        updateBody,
        true,
        true,
        true,
        true,
        true,
        globalState,
        "mcc5542"
      );

      cy.verifyBusinessProfileMcc(
        globalState,
        globalState.get("mcc5542ProfileId"),
        "7538"
      );

      cy.then(() => {
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });

    it("should verify MCC 5921 persists correctly", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      globalState.set("merchantId", globalState.get("mcc5912MerchantId"));
      globalState.set("apiKey", globalState.get("mcc5912ApiKey"));
      globalState.set("profileId", globalState.get("mcc5912ProfileId"));

      cy.verifyBusinessProfileMcc(globalState, null, "5921");

      cy.then(() => {
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });

    it("should create profile with MCC 9399 and verify persistence", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      globalState.set("merchantId", globalState.get("connectedMerchantId1"));
      globalState.set("apiKey", globalState.get("apiKeyCm1"));

      const profileName = `mcc_profile_${Date.now()}_9399`;
      const createBody = {
        ...fixtures.businessProfile.bpCreate,
        profile_name: profileName,
        merchant_category_code: "9399",
      };

      cy.createBusinessProfileTest(createBody, globalState, "mcc9399");
      cy.verifyBusinessProfileMcc(globalState, null, "9399");

      cy.then(() => {
        globalState.set("mcc9399MerchantId", globalState.get("merchantId"));
        globalState.set("mcc9399ApiKey", globalState.get("apiKey"));
        globalState.set("mcc9399ProfileId", globalState.get("mcc9399Id"));
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });

    it("should create profile with MCC 8999 and verify persistence", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      globalState.set("merchantId", globalState.get("connectedMerchantId1"));
      globalState.set("apiKey", globalState.get("apiKeyCm1"));

      const profileName = `mcc_profile_${Date.now()}_8999`;
      const createBody = {
        ...fixtures.businessProfile.bpCreate,
        profile_name: profileName,
        merchant_category_code: "8999",
      };

      cy.createBusinessProfileTest(createBody, globalState, "mcc8999");
      cy.verifyBusinessProfileMcc(globalState, null, "8999");

      cy.then(() => {
        globalState.set("mcc8999MerchantId", globalState.get("merchantId"));
        globalState.set("mcc8999ApiKey", globalState.get("apiKey"));
        globalState.set("mcc8999ProfileId", globalState.get("mcc8999Id"));
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });

    it("should create profile with MCC 6012 and verify persistence", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      globalState.set("merchantId", globalState.get("connectedMerchantId1"));
      globalState.set("apiKey", globalState.get("apiKeyCm1"));

      const profileName = `mcc_profile_${Date.now()}_6012`;
      const createBody = {
        ...fixtures.businessProfile.bpCreate,
        profile_name: profileName,
        merchant_category_code: "6012",
      };

      cy.createBusinessProfileTest(createBody, globalState, "mcc6012");
      cy.verifyBusinessProfileMcc(globalState, null, "6012");

      cy.then(() => {
        globalState.set("mcc6012MerchantId", globalState.get("merchantId"));
        globalState.set("mcc6012ApiKey", globalState.get("apiKey"));
        globalState.set("mcc6012ProfileId", globalState.get("mcc6012Id"));
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });

    it("should update MCC 9399 to 9999 and verify", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      globalState.set("merchantId", globalState.get("mcc9399MerchantId"));
      globalState.set("apiKey", globalState.get("mcc9399ApiKey"));

      const updateBody = {
        ...fixtures.businessProfile.bpUpdate,
        merchant_category_code: "9999",
      };

      cy.UpdateBusinessProfileTest(
        updateBody,
        true,
        true,
        true,
        true,
        true,
        globalState,
        "mcc9399"
      );

      cy.verifyBusinessProfileMcc(
        globalState,
        globalState.get("mcc9399ProfileId"),
        "9999"
      );

      cy.then(() => {
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });

    it("should update MCC 8999 to 0 and verify", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      globalState.set("merchantId", globalState.get("mcc8999MerchantId"));
      globalState.set("apiKey", globalState.get("mcc8999ApiKey"));

      const updateBody = {
        ...fixtures.businessProfile.bpUpdate,
        merchant_category_code: "0",
      };

      cy.UpdateBusinessProfileTest(
        updateBody,
        true,
        true,
        true,
        true,
        true,
        globalState,
        "mcc8999"
      );

      cy.verifyBusinessProfileMcc(
        globalState,
        globalState.get("mcc8999ProfileId"),
        "0"
      );

      cy.then(() => {
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });

    it("should update MCC 6012 to 7299 and verify", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      globalState.set("merchantId", globalState.get("mcc6012MerchantId"));
      globalState.set("apiKey", globalState.get("mcc6012ApiKey"));

      const updateBody = {
        ...fixtures.businessProfile.bpUpdate,
        merchant_category_code: "7299",
      };

      cy.UpdateBusinessProfileTest(
        updateBody,
        true,
        true,
        true,
        true,
        true,
        globalState,
        "mcc6012"
      );

      cy.verifyBusinessProfileMcc(
        globalState,
        globalState.get("mcc6012ProfileId"),
        "7299"
      );

      cy.then(() => {
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });
  });

  // ============================================
  // CONTEXT 6: Cleanup - Delete Profiles
  // ============================================
  context("Cleanup - Delete Profiles with MCC", () => {
    it("should delete business profile with MCC 5411", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      globalState.set("merchantId", globalState.get("mcc5411MerchantId"));
      globalState.set("apiKey", globalState.get("mcc5411ApiKey"));
      globalState.set("profileId", globalState.get("mcc5411ProfileId"));

      cy.deleteBusinessProfileTest(globalState);

      cy.then(() => {
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });

    it("should delete business profile with MCC 8111", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      globalState.set("merchantId", globalState.get("mcc8111MerchantId"));
      globalState.set("apiKey", globalState.get("mcc8111ApiKey"));
      globalState.set("profileId", globalState.get("mcc8111ProfileId"));

      cy.deleteBusinessProfileTest(globalState);

      cy.then(() => {
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });

    it("should delete business profile with MCC 7011", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      globalState.set("merchantId", globalState.get("mcc7011MerchantId"));
      globalState.set("apiKey", globalState.get("mcc7011ApiKey"));
      globalState.set("profileId", globalState.get("mcc7011ProfileId"));

      cy.deleteBusinessProfileTest(globalState);

      cy.then(() => {
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });

    it("should delete business profile with MCC 5921", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      globalState.set("merchantId", globalState.get("mcc5912MerchantId"));
      globalState.set("apiKey", globalState.get("mcc5912ApiKey"));
      globalState.set("profileId", globalState.get("mcc5912ProfileId"));

      cy.deleteBusinessProfileTest(globalState);

      cy.then(() => {
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });

    it("should delete business profile with MCC 5300", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      globalState.set("merchantId", globalState.get("mcc5331MerchantId"));
      globalState.set("apiKey", globalState.get("mcc5331ApiKey"));
      globalState.set("profileId", globalState.get("mcc5331ProfileId"));

      cy.deleteBusinessProfileTest(globalState);

      cy.then(() => {
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });

    it("should delete business profile with MCC 7996", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      globalState.set("merchantId", globalState.get("mcc7832MerchantId"));
      globalState.set("apiKey", globalState.get("mcc7832ApiKey"));
      globalState.set("profileId", globalState.get("mcc7832ProfileId"));

      cy.deleteBusinessProfileTest(globalState);

      cy.then(() => {
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });

    it("should delete business profile with MCC 4816", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      globalState.set("merchantId", globalState.get("mcc4814MerchantId"));
      globalState.set("apiKey", globalState.get("mcc4814ApiKey"));
      globalState.set("profileId", globalState.get("mcc4814ProfileId"));

      cy.deleteBusinessProfileTest(globalState);

      cy.then(() => {
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });

    it("should delete remaining MCC test profiles", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      // Delete profile 7399
      globalState.set("merchantId", globalState.get("mcc4112MerchantId"));
      globalState.set("apiKey", globalState.get("mcc4112ApiKey"));
      globalState.set("profileId", globalState.get("mcc4112ProfileId"));
      cy.deleteBusinessProfileTest(globalState);

      // Delete profile 5499
      cy.then(() => {
        globalState.set("merchantId", globalState.get("mcc4225MerchantId"));
        globalState.set("apiKey", globalState.get("mcc4225ApiKey"));
        globalState.set("profileId", globalState.get("mcc4225ProfileId"));
      });

      cy.deleteBusinessProfileTest(globalState);

      // Delete profile 7538
      cy.then(() => {
        globalState.set("merchantId", globalState.get("mcc5542MerchantId"));
        globalState.set("apiKey", globalState.get("mcc5542ApiKey"));
        globalState.set("profileId", globalState.get("mcc5542ProfileId"));
      });

      cy.deleteBusinessProfileTest(globalState);

      // Delete profile 9999
      cy.then(() => {
        globalState.set("merchantId", globalState.get("mcc9399MerchantId"));
        globalState.set("apiKey", globalState.get("mcc9399ApiKey"));
        globalState.set("profileId", globalState.get("mcc9399ProfileId"));
      });

      cy.deleteBusinessProfileTest(globalState);

      // Delete profile 0
      cy.then(() => {
        globalState.set("merchantId", globalState.get("mcc8999MerchantId"));
        globalState.set("apiKey", globalState.get("mcc8999ApiKey"));
        globalState.set("profileId", globalState.get("mcc8999ProfileId"));
      });

      cy.deleteBusinessProfileTest(globalState);

      // Delete profile 7299
      cy.then(() => {
        globalState.set("merchantId", globalState.get("mcc6012MerchantId"));
        globalState.set("apiKey", globalState.get("mcc6012ApiKey"));
        globalState.set("profileId", globalState.get("mcc6012ProfileId"));
      });

      cy.deleteBusinessProfileTest(globalState);

      cy.then(() => {
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });
  });
});
