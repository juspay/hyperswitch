import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";

let globalState;

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
      let shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("Create Business Profile", () => {
        cy.createBusinessProfileTest(
          fixtures.businessProfile.bpCreate,
          globalState
        );
      });

      it("Create Acquirer Config (Visa, is_default=true)", () => {
        const body = { ...fixtures.businessProfile.acquirerConfigCreateVisa };
        cy.createAcquirerConfigTest(body, globalState);
      });

      it("Update Acquirer Config", () => {
        const body = { ...fixtures.businessProfile.acquirerConfigUpdate };
        cy.updateAcquirerConfigTest(body, globalState);
      });

      it("Retrieve Business Profile — Verify acquirer_config_bucket populated", () => {
        cy.retrieveBusinessProfileTest(globalState).then((response) => {
          expect(response.body.acquirer_configs).to.be.an("array");
          expect(response.body.acquirer_configs.length).to.be.greaterThan(0);
          expect(response.body.acquirer_config_bucket).to.not.be.null;
          expect(response.body.acquirer_config_bucket).to.have.property(
            "default_acquirer_config"
          );
          expect(response.body.acquirer_config_bucket).to.have.property(
            "configs"
          );
          expect(
            response.body.acquirer_config_bucket.default_acquirer_config
          ).to.equal(globalState.get("profileAcquirerId"));
          const configs =
            response.body.acquirer_config_bucket.configs[
              globalState.get("profileAcquirerId")
            ];
          expect(configs).to.be.an("array");
          expect(configs[0].network).to.equal("Visa");
          expect(configs[0].acquirer_bin).to.equal("987654"); // acquirer_bin '987654' matches acquirerConfigUpdate fixture (intentional test data)
          expect(configs[0].acquirer_country_code).to.equal("840"); // acquirer_country_code '840' = ISO 3166-1 numeric code for USA (API converts alpha-2 'US' to numeric '840' — see API_TRACE Steps 7/8/10)
        });
      });
    }
  );

  context("Error Cases — Update (requires existing Visa acquirer)", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue || !globalState.get("profileAcquirerId")) {
        this.skip();
      }
    });

    it("Update with non-existent profile_acquirer_id → 404 HE_02", () => {
      const body = {
        ...fixtures.businessProfile.acquirerConfigErrorUpdateNonExistentId,
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
        ...fixtures.businessProfile.acquirerConfigErrorNoNetwork,
      };
      cy.updateAcquirerConfigTest(body, globalState, 422);
    });
  });

  context(
    "Create Second Acquirer Config (Mastercard, is_default=false)",
    () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (
          !shouldContinue ||
          !globalState.get("profileId") ||
          !globalState.get("profileAcquirerId")
        ) {
          this.skip();
        }
      });

      it("Create Acquirer Config (Mastercard)", () => {
        globalState.set(
          "visaAcquirerId",
          globalState.get("profileAcquirerId")
        );
        const body = {
          ...fixtures.businessProfile.acquirerConfigCreateMastercard,
        };
        cy.createAcquirerConfigTest(
          body,
          globalState,
          200,
          "profile",
          "mastercardAcquirerId"
        );
      });

      it("Retrieve Business Profile — Verify both acquirer configs", () => {
        cy.retrieveBusinessProfileTest(globalState).then((response) => {
          expect(response.body.acquirer_configs).to.be.an("array");
          expect(response.body.acquirer_configs.length).to.equal(2);
          expect(response.body.acquirer_config_bucket).to.not.be.null;
          expect(response.body.acquirer_config_bucket.configs).to.have.property(
            globalState.get("visaAcquirerId")
          );
          expect(response.body.acquirer_config_bucket.configs).to.have.property(
            globalState.get("mastercardAcquirerId")
          );
          const networks = response.body.acquirer_configs.map(
            (config) => config.network
          );
          expect(networks).to.include("Visa");
          expect(networks).to.include("Mastercard");
        });
      });
    }
  );

  context("Error Cases — Create (independent)", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("Create with non-existent profile_id → 404 HE_02", () => {
      const body = {
        ...fixtures.businessProfile.acquirerConfigErrorNonExistentProfile,
      };
      cy.createAcquirerConfigTest(body, globalState, 404);
    });
  });
});
