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
    it("should create merchant with vault product type and verify in response", () => {
      const { merchant_id, ...baseBody } = fixtures.merchantCreateBody;
      const merchantCreateBody = {
        ...baseBody,
        product_type: "vault",
      };

      cy.merchantCreateCallTest(merchantCreateBody, globalState, {
        expectedProductType: "vault",
      });
    });

    it("should verify product_type persists on retrieve", () => {
      cy.merchantRetrieveCall(globalState);
      cy.then(() => {
        expect(globalState.get("merchantDetails")).to.have.property(
          "product_type",
          "vault"
        );
      });
    });
  });

  context("Create merchant with product_type=recon", () => {
    it("should create merchant with recon product type and verify in response", () => {
      const { merchant_id, ...baseBody } = fixtures.merchantCreateBody;
      const merchantCreateBody = {
        ...baseBody,
        product_type: "recon",
      };

      cy.merchantCreateCallTest(merchantCreateBody, globalState, {
        expectedProductType: "recon",
      });
    });

    it("should verify product_type persists on retrieve", () => {
      cy.merchantRetrieveCall(globalState);
      cy.then(() => {
        expect(globalState.get("merchantDetails")).to.have.property(
          "product_type",
          "recon"
        );
      });
    });
  });

  context("Create merchant with product_type=recovery", () => {
    it("should create merchant with recovery product type and verify in response", () => {
      const { merchant_id, ...baseBody } = fixtures.merchantCreateBody;
      const merchantCreateBody = {
        ...baseBody,
        product_type: "recovery",
      };

      cy.merchantCreateCallTest(merchantCreateBody, globalState, {
        expectedProductType: "recovery",
      });
    });

    it("should verify product_type persists on retrieve", () => {
      cy.merchantRetrieveCall(globalState);
      cy.then(() => {
        expect(globalState.get("merchantDetails")).to.have.property(
          "product_type",
          "recovery"
        );
      });
    });
  });

  context("Create merchant with product_type=cost_observability", () => {
    it("should create merchant with cost_observability product type and verify in response", () => {
      const { merchant_id, ...baseBody } = fixtures.merchantCreateBody;
      const merchantCreateBody = {
        ...baseBody,
        product_type: "cost_observability",
      };

      cy.merchantCreateCallTest(merchantCreateBody, globalState, {
        expectedProductType: "cost_observability",
      });
    });

    it("should verify product_type persists on retrieve", () => {
      cy.merchantRetrieveCall(globalState);
      cy.then(() => {
        expect(globalState.get("merchantDetails")).to.have.property(
          "product_type",
          "cost_observability"
        );
      });
    });
  });

  context("Create merchant with product_type=dynamic_routing", () => {
    it("should create merchant with dynamic_routing product type and verify in response", () => {
      const { merchant_id, ...baseBody } = fixtures.merchantCreateBody;
      const merchantCreateBody = {
        ...baseBody,
        product_type: "dynamic_routing",
      };

      cy.merchantCreateCallTest(merchantCreateBody, globalState, {
        expectedProductType: "dynamic_routing",
      });
    });

    it("should verify product_type persists on retrieve", () => {
      cy.merchantRetrieveCall(globalState);
      cy.then(() => {
        expect(globalState.get("merchantDetails")).to.have.property(
          "product_type",
          "dynamic_routing"
        );
      });
    });
  });

  context("Create merchant without product_type (default to orchestration)", () => {
    it("should create merchant without product_type and default to orchestration", () => {
      const { merchant_id, ...merchantCreateBody } = fixtures.merchantCreateBody;

      cy.merchantCreateCallTest(merchantCreateBody, globalState, {
        expectedProductType: "orchestration",
      });
    });

    it("should verify default product_type persists on retrieve", () => {
      cy.merchantRetrieveCall(globalState);
      cy.then(() => {
        expect(globalState.get("merchantDetails")).to.have.property(
          "product_type",
          "orchestration"
        );
      });
    });
  });

  context("Create merchant with invalid product_type (negative test)", () => {
    it("should return 400 error for invalid product_type value", () => {
      const { merchant_id, ...baseBody } = fixtures.merchantCreateBody;
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
