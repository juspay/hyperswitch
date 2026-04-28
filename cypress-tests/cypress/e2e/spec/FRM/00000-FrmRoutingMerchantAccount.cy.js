import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import * as RequestBodyUtils from "../../../utils/RequestBodyUtils";

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
      const createBody = JSON.parse(
        JSON.stringify(fixtures.merchantCreateBody)
      );

      cy.merchantCreateWithFrmCallTest(
        createBody,
        fixtures.frmRoutingTestData.valid_single_routing,
        globalState,
        { merchantIdStateKey: "frmMerchantId" }
      );
    });

    it("should create merchant account with priority frm_routing_algorithm and verify response", () => {
      const createBody = JSON.parse(
        JSON.stringify(fixtures.merchantCreateBody)
      );

      cy.merchantCreateWithFrmCallTest(
        createBody,
        fixtures.frmRoutingTestData.valid_priority_routing,
        globalState,
        { merchantIdStateKey: "frmPriorityMerchantId" }
      );
    });
  });

  context("Retrieve merchant account and verify frm_routing_algorithm persistence", () => {
    it("should retrieve merchant account and verify frm_routing_algorithm persists", () => {
      const merchantId = globalState.get("frmMerchantId");

      cy.merchantRetrieveFrmCallTest(
        merchantId,
        fixtures.frmRoutingTestData.valid_single_routing,
        globalState
      );
    });
  });

  context("Update merchant account with frm_routing_algorithm via POST", () => {
    before("create a merchant for update tests", () => {
      const createBody = JSON.parse(
        JSON.stringify(fixtures.merchantCreateBody)
      );

      cy.merchantCreateWithFrmCallTest(
        createBody,
        null,
        globalState,
        { merchantIdStateKey: "frmUpdateMerchantId", verifyFrmInResponse: false }
      );
    });

    it("should update merchant account with frm_routing_algorithm", () => {
      const merchantId = globalState.get("frmUpdateMerchantId");
      const updateBody = JSON.parse(
        JSON.stringify(fixtures.merchantUpdateBody)
      );

      cy.merchantUpdateWithFrmCallTest(
        merchantId,
        updateBody,
        fixtures.frmRoutingTestData.valid_single_routing_alt,
        globalState
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

      cy.request({
        method: "POST",
        url: `${globalState.get("baseUrl")}/accounts`,
        headers: {
          Accept: "application/json",
          "Content-Type": "application/json",
          "api-key": globalState.get("adminApiKey"),
        },
        body: createBody,
      }).then((response) => {
        expect(response.status).to.equal(200);
        expect(response.body.merchant_id).to.equal(merchantId);
        expect(
          response.body.frm_routing_algorithm,
          "frm_routing_algorithm should be null when not provided"
        ).to.be.oneOf([null, undefined]);
      });
    });
  });

  context("Edge cases - invalid frm_routing_algorithm structures", () => {
    it("should handle invalid frm_routing_algorithm with missing type field", () => {
      const createBody = JSON.parse(
        JSON.stringify(fixtures.merchantCreateBody)
      );

      cy.merchantCreateWithFrmCallTest(
        createBody,
        fixtures.frmRoutingTestData.invalid_routing_missing_type,
        globalState,
        { expectedStatus: [200, 400, 422], verifyFrmInResponse: false }
      );
    });

    it("should handle invalid frm_routing_algorithm with missing data field", () => {
      const createBody = JSON.parse(
        JSON.stringify(fixtures.merchantCreateBody)
      );

      cy.merchantCreateWithFrmCallTest(
        createBody,
        fixtures.frmRoutingTestData.invalid_routing_missing_data,
        globalState,
        { expectedStatus: [200, 400, 422], verifyFrmInResponse: false }
      );
    });

    it("should handle invalid frm_routing_algorithm with empty object", () => {
      const createBody = JSON.parse(
        JSON.stringify(fixtures.merchantCreateBody)
      );

      cy.merchantCreateWithFrmCallTest(
        createBody,
        fixtures.frmRoutingTestData.invalid_routing_empty_object,
        globalState,
        { expectedStatus: [200, 400, 422], verifyFrmInResponse: false }
      );
    });

    it("should handle invalid frm_routing_algorithm with unknown type", () => {
      const createBody = JSON.parse(
        JSON.stringify(fixtures.merchantCreateBody)
      );

      cy.merchantCreateWithFrmCallTest(
        createBody,
        fixtures.frmRoutingTestData.invalid_routing_unknown_type,
        globalState,
        { expectedStatus: [200, 400, 422], verifyFrmInResponse: false }
      );
    });
  });
});
