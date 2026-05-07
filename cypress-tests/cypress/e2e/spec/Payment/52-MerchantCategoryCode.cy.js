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
      // Use a valid MCC code (5411 = Grocery Stores)
      const bpCreateWithMcc = {
        ...fixtures.businessProfile.bpCreate,
        merchant_category_code: "5411",
      };

      cy.createBusinessProfileWithMccTest(
        bpCreateWithMcc,
        globalState,
        "mccProfile"
      );
    });
  });

  context("Verify Merchant Category Code in Response", () => {
    it("verifies-mcc-in-profile-response", () => {
      cy.retrieveBusinessProfileTest(globalState, "mccProfile");
    });
  });

  context("Update Profile with Different Merchant Category Code", () => {
    it("updates-business-profile-with-new-mcc", () => {
      // Update to a different valid MCC (7011 = Hotels)
      const bpUpdateWithMcc = {
        ...fixtures.businessProfile.bpUpdate,
        merchant_category_code: "7011",
      };

      cy.updateBusinessProfileWithMccTest(
        bpUpdateWithMcc,
        globalState,
        "mccProfile"
      );
    });
  });

  context("Verify Updated Merchant Category Code Persisted", () => {
    it("verifies-updated-mcc-persisted", () => {
      cy.retrieveAndVerifyBusinessProfileMccTest(
        globalState,
        "mccProfile",
        "7011"
      );
    });
  });

  context("Create Profile with Different Valid MCC Codes", () => {
    [
      { code: "5411", name: "Grocery Stores" },
      { code: "8111", name: "Legal Services" },
      { code: "7011", name: "Hotels" },
    ].forEach(({ code, name }) => {
      it(`creates-profile-with-mcc-${code}-${name.replace(/\s+/g, "-").toLowerCase()}`, () => {
        const bpCreateWithMcc = {
          ...fixtures.businessProfile.bpCreate,
          merchant_category_code: code,
        };

        cy.createBusinessProfileWithMccTest(
          bpCreateWithMcc,
          globalState,
          `mccProfile${code}`
        );
      });
    });
  });

  context("Negative Test - Invalid MCC Format", () => {
    it("rejects-invalid-mcc-format", () => {
      // Invalid MCC (not 4 digits)
      const bpCreateWithInvalidMcc = {
        ...fixtures.businessProfile.bpCreate,
        merchant_category_code: "12345", // 5 digits - invalid
      };

      cy.createBusinessProfileTest(
        bpCreateWithInvalidMcc,
        globalState,
        "invalidMccProfile",
        400 // Expect 400 Bad Request
      );
    });
  });

  context("Negative Test - Non-numeric MCC", () => {
    it("rejects-non-numeric-mcc", () => {
      // Invalid MCC (non-numeric)
      const bpCreateWithInvalidMcc = {
        ...fixtures.businessProfile.bpCreate,
        merchant_category_code: "ABCD",
      };

      cy.createBusinessProfileTest(
        bpCreateWithInvalidMcc,
        globalState,
        "invalidMccProfile2",
        400 // Expect 400 Bad Request
      );
    });
  });

  context("Cleanup - Delete Created Business Profiles", () => {
    it("deletes-mcc-test-profiles", () => {
      // Delete the main MCC test profile
      cy.deleteBusinessProfileTest(globalState);
    });
  });
});
