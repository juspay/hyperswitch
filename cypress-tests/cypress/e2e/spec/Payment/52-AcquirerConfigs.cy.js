import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";

let globalState;

const acquirerConfigCreateVisa = {
  acquirer_assigned_merchant_id: "M123456789",
  merchant_name: "NewAge Retailer",
  network: "Visa",
  acquirer_bin: "456789",
  acquirer_ica: "401288",
  acquirer_fraud_rate: 0.01,
  acquirer_country_code: "US",
  is_default: true,
};

const acquirerConfigCreateMastercard = {
  acquirer_assigned_merchant_id: "M555444333",
  merchant_name: "Mastercard Retailer",
  network: "Mastercard",
  acquirer_bin: "555555",
  acquirer_country_code: "US",
  is_default: false,
};

const acquirerConfigUpdate = {
  network: "Visa",
  acquirer_assigned_merchant_id: "M987654321",
  merchant_name: "Updated Retailer",
  acquirer_bin: "987654",
  acquirer_ica: "501288",
  acquirer_fraud_rate: 0.02,
  acquirer_country_code: "US",
  is_default: true,
};

const acquirerConfigErrorNonExistentProfile = {
  profile_id: "pro_nonexistent_12345",
  acquirer_assigned_merchant_id: "M999999999",
  merchant_name: "Invalid Profile",
  network: "Visa",
  acquirer_bin: "999999",
};

const acquirerConfigErrorUpdateNonExistentId = {
  network: "Visa",
  acquirer_bin: "111111",
};

const acquirerConfigErrorNoNetwork = {
  acquirer_bin: "222222",
};

describe("Acquirer-specific configurations", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  after("cleanup business profile", () => {
    cy.deleteBusinessProfileTest(globalState);
  });

  context(
    "Create, Update, and Verify Visa Acquirer Config (is_default=true)",
    () => {
      it("Create Business Profile", () => {
        cy.createBusinessProfileTest(
          fixtures.businessProfile.bpCreate,
          globalState
        );
      });

      it("Create Acquirer Config (Visa, is_default=true)", () => {
        const body = { ...acquirerConfigCreateVisa };
        cy.createAcquirerConfigTest(body, globalState);
      });

      it("Update Acquirer Config", () => {
        const body = { ...acquirerConfigUpdate };
        body.acquirer_bin = Cypress._.random(100000, 999999).toString();
        globalState.set("visaAcquirerBin", body.acquirer_bin);
        cy.updateAcquirerConfigTest(body, globalState);
      });

      it("Retrieve Business Profile — Verify acquirer_config_bucket populated", () => {
        cy.verifyBusinessProfileAcquirerConfigTest(globalState);
      });
    }
  );

  context("Error Cases — Update (requires existing Visa acquirer)", () => {
    beforeEach(function () {
      if (!globalState.get("profileAcquirerId")) {
        this.skip();
      }
    });

    it("Update with non-existent profile_acquirer_id → 404 HE_02", () => {
      const body = {
        ...acquirerConfigErrorUpdateNonExistentId,
      };
      cy.updateAcquirerConfigTest(
        body,
        globalState,
        404,
        "profile",
        "pro_acq_nonexistent_12345"
      );
    });

    it("Update without network field → 422 IR_06", () => {
      const body = {
        ...acquirerConfigErrorNoNetwork,
      };
      cy.updateAcquirerConfigTest(body, globalState, 422);
    });
  });

  context(
    "Create Second Acquirer Config (Mastercard, is_default=false)",
    () => {
      beforeEach(function () {
        if (
          !globalState.get("profileId") ||
          !globalState.get("profileAcquirerId")
        ) {
          this.skip();
        }
      });

      it("Create Acquirer Config (Mastercard)", () => {
        globalState.set("visaAcquirerId", globalState.get("profileAcquirerId"));
        const body = { ...acquirerConfigCreateMastercard };
        cy.createAcquirerConfigTest(
          body,
          globalState,
          200,
          "profile",
          "mastercardAcquirerId"
        );
      });

      it("Retrieve Business Profile — Verify both acquirer configs", () => {
        cy.verifyBusinessProfileAcquirerConfigTest(
          globalState,
          "profile",
          "verifyMultipleAcquirerConfigs"
        );
      });
    }
  );

  context("Error Cases — Create (independent)", () => {
    it("Create with non-existent profile_id → 404 HE_02", () => {
      const body = {
        ...acquirerConfigErrorNonExistentProfile,
      };
      cy.createAcquirerConfigTest(body, globalState, 404);
    });
  });
});
