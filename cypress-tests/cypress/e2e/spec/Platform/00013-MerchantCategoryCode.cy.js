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

  afterEach("flush global state after each test", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Create Profile with Merchant Category Code", () => {
    it("creates-business-profile-with-mcc", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      // Use a valid MCC code (5411 = Grocery Stores)
      const bpCreateWithMcc = {
        ...fixtures.businessProfile.bpCreate,
        merchant_category_code: "5411",
      };

      globalState.set("merchantId", globalState.get("standardMerchantId"));
      globalState.set("apiKey", globalState.get("apiKeySm"));

      cy.createBusinessProfileWithMccTest(
        bpCreateWithMcc,
        globalState,
        "mccProfile"
      );

      cy.then(() => {
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });
  });

  context("Verify Merchant Category Code in Response", () => {
    it("verifies-mcc-in-profile-response", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      globalState.set("merchantId", globalState.get("standardMerchantId"));
      globalState.set("apiKey", globalState.get("apiKeySm"));

      cy.retrieveBusinessProfileTest(
        globalState,
        "mccProfile"
      );

      cy.then(() => {
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });
  });

  context("Update Profile with Different Merchant Category Code", () => {
    it("updates-business-profile-with-new-mcc", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      // Update to a different valid MCC (7011 = Hotels)
      const bpUpdateWithMcc = {
        ...fixtures.businessProfile.bpUpdate,
        merchant_category_code: "7011",
      };

      globalState.set("merchantId", globalState.get("standardMerchantId"));
      globalState.set("apiKey", globalState.get("apiKeySm"));

      cy.updateBusinessProfileWithMccTest(
        bpUpdateWithMcc,
        globalState,
        "mccProfile"
      );

      cy.then(() => {
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });
  });

  context("Verify Updated Merchant Category Code Persisted", () => {
    it("verifies-updated-mcc-persisted", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      globalState.set("merchantId", globalState.get("standardMerchantId"));
      globalState.set("apiKey", globalState.get("apiKeySm"));

      cy.retrieveAndVerifyBusinessProfileMccTest(
        globalState,
        "mccProfile",
        "7011"
      );

      cy.then(() => {
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });
  });

  context("Create Profile with Different Valid MCC Codes", () => {
    [
      { code: "5411", name: "Grocery Stores" },
      { code: "8111", name: "Legal Services" },
      { code: "7011", name: "Hotels" },
    ].forEach(({ code, name }) => {
      it(`creates-profile-with-mcc-${code}-${name.replace(/\s+/g, '-').toLowerCase()}`, () => {
        const savedMerchantId = globalState.get("merchantId");
        const savedApiKey = globalState.get("apiKey");

        const bpCreateWithMcc = {
          ...fixtures.businessProfile.bpCreate,
          merchant_category_code: code,
        };

        globalState.set("merchantId", globalState.get("standardMerchantId"));
        globalState.set("apiKey", globalState.get("apiKeySm"));

        cy.createBusinessProfileWithMccTest(
          bpCreateWithMcc,
          globalState,
          `mccProfile${code}`
        );

        cy.then(() => {
          globalState.set("merchantId", savedMerchantId);
          globalState.set("apiKey", savedApiKey);
        });
      });
    });
  });

  context("Negative Test - Invalid MCC Format", () => {
    it("rejects-invalid-mcc-format", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      // Invalid MCC (not 4 digits)
      const bpCreateWithInvalidMcc = {
        ...fixtures.businessProfile.bpCreate,
        merchant_category_code: "12345", // 5 digits - invalid
      };

      globalState.set("merchantId", globalState.get("standardMerchantId"));
      globalState.set("apiKey", globalState.get("apiKeySm"));

      cy.createBusinessProfileTest(
        bpCreateWithInvalidMcc,
        globalState,
        "invalidMccProfile",
        400 // Expect 400 Bad Request
      );

      cy.then(() => {
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });
  });

  context("Negative Test - Non-numeric MCC", () => {
    it("rejects-non-numeric-mcc", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      // Invalid MCC (non-numeric)
      const bpCreateWithInvalidMcc = {
        ...fixtures.businessProfile.bpCreate,
        merchant_category_code: "ABCD",
      };

      globalState.set("merchantId", globalState.get("standardMerchantId"));
      globalState.set("apiKey", globalState.get("apiKeySm"));

      cy.createBusinessProfileTest(
        bpCreateWithInvalidMcc,
        globalState,
        "invalidMccProfile2",
        400 // Expect 400 Bad Request
      );

      cy.then(() => {
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });
  });

  context("Cleanup - Delete Created Business Profiles", () => {
    it("deletes-mcc-test-profiles", () => {
      const savedMerchantId = globalState.get("merchantId");
      const savedApiKey = globalState.get("apiKey");

      globalState.set("merchantId", globalState.get("standardMerchantId"));
      globalState.set("apiKey", globalState.get("apiKeySm"));

      // Delete the main MCC test profile
      cy.deleteBusinessProfileTest(globalState);

      cy.then(() => {
        globalState.set("merchantId", savedMerchantId);
        globalState.set("apiKey", savedApiKey);
      });
    });
  });
});
