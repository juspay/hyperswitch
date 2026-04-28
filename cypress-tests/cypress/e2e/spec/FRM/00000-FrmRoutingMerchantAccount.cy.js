import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";

let globalState;

describe("FRM Routing Algorithm - Merchant Account Tests", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Create merchant account with frm_routing_algorithm", () => {
    it("should create merchant account with single frm_routing_algorithm and verify response", () => {
      const merchantId = RequestBodyUtils.generateRandomString();
      const createBody = JSON.parse(
        JSON.stringify(fixtures.merchantCreateBody)
      );
      createBody.merchant_id = merchantId;
      createBody.frm_routing_algorithm =
        fixtures.frmRoutingTestData.valid_single_routing;

      globalState.set("frmMerchantId", merchantId);

      cy.merchantCreateCallTest(createBody, globalState, {
        merchantIdStateKey: "frmMerchantId",
        profileIdStateKey: "frmProfileId",
        publishableKeyStateKey: "frmPublishableKey",
      }).then((response) => {
        expect(response.body).to.have.property("frm_routing_algorithm");
        expect(response.body.frm_routing_algorithm).to.deep.equal(
          fixtures.frmRoutingTestData.valid_single_routing
        );
      });
    });

    it("should create merchant account with priority frm_routing_algorithm and verify response", () => {
      const merchantId = RequestBodyUtils.generateRandomString();
      const createBody = JSON.parse(
        JSON.stringify(fixtures.merchantCreateBody)
      );
      createBody.merchant_id = merchantId;
      createBody.frm_routing_algorithm =
        fixtures.frmRoutingTestData.valid_priority_routing;

      globalState.set("frmPriorityMerchantId", merchantId);

      cy.merchantCreateCallTest(createBody, globalState, {
        merchantIdStateKey: "frmPriorityMerchantId",
      }).then((response) => {
        expect(response.body).to.have.property("frm_routing_algorithm");
        expect(response.body.frm_routing_algorithm).to.deep.equal(
          fixtures.frmRoutingTestData.valid_priority_routing
        );
      });
    });
  });

  context(
    "Retrieve merchant account and verify frm_routing_algorithm persistence",
    () => {
      it("should retrieve merchant account and verify frm_routing_algorithm persists", () => {
        const merchantId = globalState.get("frmMerchantId");

        cy.merchantRetrieveCallWithId(merchantId, globalState).then(
          (response) => {
            expect(response.body.merchant_id).to.equal(merchantId);
            expect(response.body).to.have.property("frm_routing_algorithm");
            expect(response.body.frm_routing_algorithm).to.deep.equal(
              fixtures.frmRoutingTestData.valid_single_routing
            );
          }
        );
      });
    }
  );

  context("Update merchant account with frm_routing_algorithm via POST", () => {
    before("create a merchant for update tests", () => {
      const merchantId = RequestBodyUtils.generateRandomString();
      const createBody = JSON.parse(
        JSON.stringify(fixtures.merchantCreateBody)
      );
      createBody.merchant_id = merchantId;

      globalState.set("frmUpdateMerchantId", merchantId);

      cy.merchantCreateCallTest(createBody, globalState, {
        merchantIdStateKey: "frmUpdateMerchantId",
        profileIdStateKey: "frmUpdateProfileId",
        publishableKeyStateKey: "frmUpdatePublishableKey",
      }).then((response) => {
        globalState.set(
          "frmUpdateOrganizationId",
          response.body.organization_id
        );
      });
    });

    it("should update merchant account with frm_routing_algorithm", () => {
      const merchantId = globalState.get("frmUpdateMerchantId");
      const updateBody = JSON.parse(
        JSON.stringify(fixtures.merchantUpdateBody)
      );
      updateBody.merchant_id = merchantId;
      updateBody.frm_routing_algorithm =
        fixtures.frmRoutingTestData.valid_single_routing_alt;

      cy.merchantUpdateCallTest(updateBody, globalState, merchantId).then(
        (response) => {
          expect(response.body.merchant_id).to.equal(merchantId);
          expect(response.body).to.have.property("frm_routing_algorithm");
          expect(response.body.frm_routing_algorithm).to.deep.equal(
            fixtures.frmRoutingTestData.valid_single_routing_alt
          );
        }
      );
    });
  });

  context("Create merchant account without frm_routing_algorithm", () => {
    it("should create merchant account without frm_routing_algorithm and field should be absent or null", () => {
      const merchantId = RequestBodyUtils.generateRandomString();
      const createBody = JSON.parse(
        JSON.stringify(fixtures.merchantCreateBody)
      );
      createBody.merchant_id = merchantId;
      delete createBody.frm_routing_algorithm;

      cy.merchantCreateCallTest(createBody, globalState).then((response) => {
        expect(
          response.body.frm_routing_algorithm,
          "frm_routing_algorithm should be null when not provided"
        ).to.be.oneOf([null, undefined]);
      });
    });
  });

  context("Edge cases - invalid frm_routing_algorithm structures", () => {
    it("should handle invalid frm_routing_algorithm with missing type field", () => {
      const merchantId = RequestBodyUtils.generateRandomString();
      const createBody = JSON.parse(
        JSON.stringify(fixtures.merchantCreateBody)
      );
      createBody.merchant_id = merchantId;
      createBody.frm_routing_algorithm =
        fixtures.frmRoutingTestData.invalid_routing_missing_type;

      cy.merchantCreateCallTestExpectingPossibleFailure(
        createBody,
        globalState,
        [200, 400, 422]
      );
    });

    it("should handle invalid frm_routing_algorithm with missing data field", () => {
      const merchantId = RequestBodyUtils.generateRandomString();
      const createBody = JSON.parse(
        JSON.stringify(fixtures.merchantCreateBody)
      );
      createBody.merchant_id = merchantId;
      createBody.frm_routing_algorithm =
        fixtures.frmRoutingTestData.invalid_routing_missing_data;

      cy.merchantCreateCallTestExpectingPossibleFailure(
        createBody,
        globalState,
        [200, 400, 422]
      );
    });

    it("should handle invalid frm_routing_algorithm with empty object", () => {
      const merchantId = RequestBodyUtils.generateRandomString();
      const createBody = JSON.parse(
        JSON.stringify(fixtures.merchantCreateBody)
      );
      createBody.merchant_id = merchantId;
      createBody.frm_routing_algorithm =
        fixtures.frmRoutingTestData.invalid_routing_empty_object;

      cy.merchantCreateCallTestExpectingPossibleFailure(
        createBody,
        globalState,
        [200, 400, 422]
      );
    });

    it("should handle invalid frm_routing_algorithm with unknown type", () => {
      const merchantId = RequestBodyUtils.generateRandomString();
      const createBody = JSON.parse(
        JSON.stringify(fixtures.merchantCreateBody)
      );
      createBody.merchant_id = merchantId;
      createBody.frm_routing_algorithm =
        fixtures.frmRoutingTestData.invalid_routing_unknown_type;

      cy.merchantCreateCallTestExpectingPossibleFailure(
        createBody,
        globalState,
        [200, 400, 422]
      );
    });
  });
});
