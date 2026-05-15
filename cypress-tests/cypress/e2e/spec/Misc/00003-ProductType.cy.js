import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
let globalState;
describe("Merchant Account Product Type Tests", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });
  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });
  context("Create merchant with product_type=vault", () => {
    it("should create merchant with vault product type and verify persistence", () => {
      const { merchant_id: _, ...baseBody } = fixtures.merchantCreateBody;
      const merchantCreateBody = {
        ...baseBody,
        product_type: "vault",
      };
      // Create merchant with vault product_type
      cy.request({
        method: "POST",
        url: `${globalState.get("baseUrl")}/accounts`,
        headers: {
          Accept: "application/json",
          "Content-Type": "application/json",
          "api-key": globalState.get("adminApiKey"),
        },
        body: merchantCreateBody,
      }).then((response) => {
        expect(response.status).to.equal(200);
        expect(response.body).to.have.property("product_type", "vault");
        globalState.set("merchantId", response.body.merchant_id);
        // Verify product_type persists on retrieve
        const merchant_id_val = globalState.get("merchantId");
        cy.request({
          method: "GET",
          url: `${globalState.get("baseUrl")}/accounts/${merchant_id_val}`,
          headers: {
            Accept: "application/json",
            "Content-Type": "application/json",
            "api-key": globalState.get("adminApiKey"),
          },
          failOnStatusCode: false,
        }).then((getResponse) => {
          expect(getResponse.body).to.have.property("product_type", "vault");
        });
        // Cleanup merchant
        cy.merchantDeleteCall(globalState);
      });
    });
  });
  context("Create merchant with product_type=recon", () => {
    it("should create merchant with recon product type and verify persistence", () => {
      const { merchant_id: _, ...baseBody } = fixtures.merchantCreateBody;
      const merchantCreateBody = {
        ...baseBody,
        product_type: "recon",
      };
      cy.request({
        method: "POST",
        url: `${globalState.get("baseUrl")}/accounts`,
        headers: {
          Accept: "application/json",
          "Content-Type": "application/json",
          "api-key": globalState.get("adminApiKey"),
        },
        body: merchantCreateBody,
      }).then((response) => {
        expect(response.status).to.equal(200);
        expect(response.body).to.have.property("product_type", "recon");
        globalState.set("merchantId", response.body.merchant_id);
        const merchant_id_val = globalState.get("merchantId");
        cy.request({
          method: "GET",
          url: `${globalState.get("baseUrl")}/accounts/${merchant_id_val}`,
          headers: {
            Accept: "application/json",
            "Content-Type": "application/json",
            "api-key": globalState.get("adminApiKey"),
          },
          failOnStatusCode: false,
        }).then((getResponse) => {
          expect(getResponse.body).to.have.property("product_type", "recon");
        });
        cy.merchantDeleteCall(globalState);
      });
    });
  });
  context("Create merchant with product_type=recovery", () => {
    it("should create merchant with recovery product type and verify persistence", () => {
      const { merchant_id: _, ...baseBody } = fixtures.merchantCreateBody;
      const merchantCreateBody = {
        ...baseBody,
        product_type: "recovery",
      };
      cy.request({
        method: "POST",
        url: `${globalState.get("baseUrl")}/accounts`,
        headers: {
          Accept: "application/json",
          "Content-Type": "application/json",
          "api-key": globalState.get("adminApiKey"),
        },
        body: merchantCreateBody,
      }).then((response) => {
        expect(response.status).to.equal(200);
        expect(response.body).to.have.property("product_type", "recovery");
        globalState.set("merchantId", response.body.merchant_id);
        const merchant_id_val = globalState.get("merchantId");
        cy.request({
          method: "GET",
          url: `${globalState.get("baseUrl")}/accounts/${merchant_id_val}`,
          headers: {
            Accept: "application/json",
            "Content-Type": "application/json",
            "api-key": globalState.get("adminApiKey"),
          },
          failOnStatusCode: false,
        }).then((getResponse) => {
          expect(getResponse.body).to.have.property("product_type", "recovery");
        });
        cy.merchantDeleteCall(globalState);
      });
    });
  });
  context("Create merchant with product_type=cost_observability", () => {
    it("should create merchant with cost_observability product type and verify persistence", () => {
      const { merchant_id: _, ...baseBody } = fixtures.merchantCreateBody;
      const merchantCreateBody = {
        ...baseBody,
        product_type: "cost_observability",
      };
      cy.request({
        method: "POST",
        url: `${globalState.get("baseUrl")}/accounts`,
        headers: {
          Accept: "application/json",
          "Content-Type": "application/json",
          "api-key": globalState.get("adminApiKey"),
        },
        body: merchantCreateBody,
      }).then((response) => {
        expect(response.status).to.equal(200);
        expect(response.body).to.have.property("product_type", "cost_observability");
        globalState.set("merchantId", response.body.merchant_id);
        const merchant_id_val = globalState.get("merchantId");
        cy.request({
          method: "GET",
          url: `${globalState.get("baseUrl")}/accounts/${merchant_id_val}`,
          headers: {
            Accept: "application/json",
            "Content-Type": "application/json",
            "api-key": globalState.get("adminApiKey"),
          },
          failOnStatusCode: false,
        }).then((getResponse) => {
          expect(getResponse.body).to.have.property("product_type", "cost_observability");
        });
        cy.merchantDeleteCall(globalState);
      });
    });
  });
  context("Create merchant with product_type=dynamic_routing", () => {
    it("should create merchant with dynamic_routing product type and verify persistence", () => {
      const { merchant_id: _, ...baseBody } = fixtures.merchantCreateBody;
      const merchantCreateBody = {
        ...baseBody,
        product_type: "dynamic_routing",
      };
      cy.request({
        method: "POST",
        url: `${globalState.get("baseUrl")}/accounts`,
        headers: {
          Accept: "application/json",
          "Content-Type": "application/json",
          "api-key": globalState.get("adminApiKey"),
        },
        body: merchantCreateBody,
      }).then((response) => {
        expect(response.status).to.equal(200);
        expect(response.body).to.have.property("product_type", "dynamic_routing");
        globalState.set("merchantId", response.body.merchant_id);
        const merchant_id_val = globalState.get("merchantId");
        cy.request({
          method: "GET",
          url: `${globalState.get("baseUrl")}/accounts/${merchant_id_val}`,
          headers: {
            Accept: "application/json",
            "Content-Type": "application/json",
            "api-key": globalState.get("adminApiKey"),
          },
          failOnStatusCode: false,
        }).then((getResponse) => {
          expect(getResponse.body).to.have.property("product_type", "dynamic_routing");
        });
        cy.merchantDeleteCall(globalState);
      });
    });
  });
  context("Create merchant without product_type (default to orchestration)", () => {
    it("should create merchant without product_type and default to orchestration", () => {
      const { merchant_id: _merchant_id, ...merchantCreateBody } = fixtures.merchantCreateBody;
      cy.request({
        method: "POST",
        url: `${globalState.get("baseUrl")}/accounts`,
        headers: {
          Accept: "application/json",
          "Content-Type": "application/json",
          "api-key": globalState.get("adminApiKey"),
        },
        body: merchantCreateBody,
      }).then((response) => {
        expect(response.status).to.equal(200);
        expect(response.body).to.have.property("product_type", "orchestration");
        globalState.set("merchantId", response.body.merchant_id);
        const merchant_id_val = globalState.get("merchantId");
        cy.request({
          method: "GET",
          url: `${globalState.get("baseUrl")}/accounts/${merchant_id_val}`,
          headers: {
            Accept: "application/json",
            "Content-Type": "application/json",
            "api-key": globalState.get("adminApiKey"),
          },
          failOnStatusCode: false,
        }).then((getResponse) => {
          expect(getResponse.body).to.have.property("product_type", "orchestration");
        });
        cy.merchantDeleteCall(globalState);
      });
    });
  });
  context("Create merchant with invalid product_type (negative test)", () => {
    it("should return 400 error for invalid product_type value", () => {
      const { merchant_id: _, ...baseBody } = fixtures.merchantCreateBody;
      const merchantCreateBody = {
        ...baseBody,
        product_type: "invalid_product_type",
      };
      cy.request({
        method: "POST",
        url: `${globalState.get("baseUrl")}/accounts`,
        headers: {
          Accept: "application/json",
          "Content-Type": "application/json",
          "api-key": globalState.get("adminApiKey"),
        },
        body: merchantCreateBody,
        failOnStatusCode: false,
      }).then((response) => {
        expect(response.status).to.equal(400);
        expect(response.body).to.have.property("error");
        expect(response.body.error.code).to.equal("IR_06");
      });
    });
  });
});
