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
      const merchantId = RequestBodyUtils.generateRandomString();
      const createBody = JSON.parse(
        JSON.stringify(fixtures.merchantCreateBody)
      );
      createBody.merchant_id = merchantId;
      createBody.frm_routing_algorithm =
        fixtures.frmRoutingTestData.valid_single_routing;

      globalState.set("frmMerchantId", merchantId);

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
        expect(response.body).to.have.property("frm_routing_algorithm");
        expect(response.body.frm_routing_algorithm).to.deep.equal(
          fixtures.frmRoutingTestData.valid_single_routing
        );
        globalState.set("frmProfileId", response.body.default_profile);
        globalState.set("frmPublishableKey", response.body.publishable_key);
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

        cy.request({
          method: "GET",
          url: `${globalState.get("baseUrl")}/accounts/${merchantId}`,
          headers: {
            Accept: "application/json",
            "Content-Type": "application/json",
            "api-key": globalState.get("adminApiKey"),
          },
        }).then((response) => {
          expect(response.status).to.equal(200);
          expect(response.body.merchant_id).to.equal(merchantId);
          expect(response.body).to.have.property("frm_routing_algorithm");
          expect(response.body.frm_routing_algorithm).to.deep.equal(
            fixtures.frmRoutingTestData.valid_single_routing
          );
        });
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
        globalState.set("frmUpdateProfileId", response.body.default_profile);
        globalState.set(
          "frmUpdatePublishableKey",
          response.body.publishable_key
        );
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

      cy.request({
        method: "POST",
        url: `${globalState.get("baseUrl")}/accounts/${merchantId}`,
        headers: {
          Accept: "application/json",
          "Content-Type": "application/json",
          "api-key": globalState.get("adminApiKey"),
        },
        body: updateBody,
        failOnStatusCode: false,
      }).then((response) => {
        expect(response.status).to.equal(200);
        expect(response.body.merchant_id).to.equal(merchantId);
        expect(response.body).to.have.property("frm_routing_algorithm");
        expect(response.body.frm_routing_algorithm).to.deep.equal(
          fixtures.frmRoutingTestData.valid_single_routing_alt
        );
      });
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
      const merchantId = RequestBodyUtils.generateRandomString();
      const createBody = JSON.parse(
        JSON.stringify(fixtures.merchantCreateBody)
      );
      createBody.merchant_id = merchantId;
      createBody.frm_routing_algorithm =
        fixtures.frmRoutingTestData.invalid_routing_missing_type;

      cy.request({
        method: "POST",
        url: `${globalState.get("baseUrl")}/accounts`,
        headers: {
          Accept: "application/json",
          "Content-Type": "application/json",
          "api-key": globalState.get("adminApiKey"),
        },
        body: createBody,
        failOnStatusCode: false,
      }).then((response) => {
        expect(response.status).to.be.oneOf([200, 400, 422]);
        if (response.status === 200) {
          expect(response.body.merchant_id).to.equal(merchantId);
          expect(response.body).to.have.property("frm_routing_algorithm");
        } else {
          expect(response.body).to.have.property("error");
        }
      });
    });

    it("should handle invalid frm_routing_algorithm with missing data field", () => {
      const merchantId = RequestBodyUtils.generateRandomString();
      const createBody = JSON.parse(
        JSON.stringify(fixtures.merchantCreateBody)
      );
      createBody.merchant_id = merchantId;
      createBody.frm_routing_algorithm =
        fixtures.frmRoutingTestData.invalid_routing_missing_data;

      cy.request({
        method: "POST",
        url: `${globalState.get("baseUrl")}/accounts`,
        headers: {
          Accept: "application/json",
          "Content-Type": "application/json",
          "api-key": globalState.get("adminApiKey"),
        },
        body: createBody,
        failOnStatusCode: false,
      }).then((response) => {
        expect(response.status).to.be.oneOf([200, 400, 422]);
        if (response.status === 200) {
          expect(response.body.merchant_id).to.equal(merchantId);
          expect(response.body).to.have.property("frm_routing_algorithm");
        } else {
          expect(response.body).to.have.property("error");
        }
      });
    });

    it("should handle invalid frm_routing_algorithm with empty object", () => {
      const merchantId = RequestBodyUtils.generateRandomString();
      const createBody = JSON.parse(
        JSON.stringify(fixtures.merchantCreateBody)
      );
      createBody.merchant_id = merchantId;
      createBody.frm_routing_algorithm =
        fixtures.frmRoutingTestData.invalid_routing_empty_object;

      cy.request({
        method: "POST",
        url: `${globalState.get("baseUrl")}/accounts`,
        headers: {
          Accept: "application/json",
          "Content-Type": "application/json",
          "api-key": globalState.get("adminApiKey"),
        },
        body: createBody,
        failOnStatusCode: false,
      }).then((response) => {
        expect(response.status).to.be.oneOf([200, 400, 422]);
        if (response.status === 200) {
          expect(response.body.merchant_id).to.equal(merchantId);
          expect(response.body).to.have.property("frm_routing_algorithm");
        } else {
          expect(response.body).to.have.property("error");
        }
      });
    });

    it("should handle invalid frm_routing_algorithm with unknown type", () => {
      const merchantId = RequestBodyUtils.generateRandomString();
      const createBody = JSON.parse(
        JSON.stringify(fixtures.merchantCreateBody)
      );
      createBody.merchant_id = merchantId;
      createBody.frm_routing_algorithm =
        fixtures.frmRoutingTestData.invalid_routing_unknown_type;

      cy.request({
        method: "POST",
        url: `${globalState.get("baseUrl")}/accounts`,
        headers: {
          Accept: "application/json",
          "Content-Type": "application/json",
          "api-key": globalState.get("adminApiKey"),
        },
        body: createBody,
        failOnStatusCode: false,
      }).then((response) => {
        expect(response.status).to.be.oneOf([200, 400, 422]);
        if (response.status === 200) {
          expect(response.body.merchant_id).to.equal(merchantId);
          expect(response.body).to.have.property("frm_routing_algorithm");
        } else {
          expect(response.body).to.have.property("error");
        }
      });
    });
  });
});
