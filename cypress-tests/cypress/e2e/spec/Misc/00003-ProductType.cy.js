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

  afterEach("cleanup merchant after each test", () => {
    const merchantId = globalState.get("merchantId");
    if (merchantId) {
      cy.merchantDeleteCall(globalState);
    }
  });

  context("Create merchant with product_type=vault", () => {
    it("should create merchant with vault product type", () => {
      const baseBody = { ...fixtures.merchantCreateBody };
      delete baseBody.merchant_id;
      const merchantCreateBody = {
        ...baseBody,
        product_type: "vault",
      };

      cy.merchantCreateCallTest(merchantCreateBody, globalState, {
        expectedProductType: "vault",
      });
    });

    it("should verify vault product_type persists on retrieve", function () {
      const merchantId = globalState.get("merchantId");
      if (!merchantId) {
        this.skip();
      }
      cy.merchantRetrieveCall(globalState, {
        expectedProductType: "vault",
      });
    });
  });

  context("Create merchant with product_type=recon", () => {
    it("should create merchant with recon product type", () => {
      const baseBody = { ...fixtures.merchantCreateBody };
      delete baseBody.merchant_id;
      const merchantCreateBody = {
        ...baseBody,
        product_type: "recon",
      };

      cy.merchantCreateCallTest(merchantCreateBody, globalState, {
        expectedProductType: "recon",
      });
    });

    it("should verify recon product_type persists on retrieve", function () {
      const merchantId = globalState.get("merchantId");
      if (!merchantId) {
        this.skip();
      }
      cy.merchantRetrieveCall(globalState, {
        expectedProductType: "recon",
      });
    });
  });

  context("Create merchant with product_type=recovery", () => {
    it("should create merchant with recovery product type", () => {
      const baseBody = { ...fixtures.merchantCreateBody };
      delete baseBody.merchant_id;
      const merchantCreateBody = {
        ...baseBody,
        product_type: "recovery",
      };

      cy.merchantCreateCallTest(merchantCreateBody, globalState, {
        expectedProductType: "recovery",
      });
    });

    it("should verify recovery product_type persists on retrieve", function () {
      const merchantId = globalState.get("merchantId");
      if (!merchantId) {
        this.skip();
      }
      cy.merchantRetrieveCall(globalState, {
        expectedProductType: "recovery",
      });
    });
  });

  context("Create merchant with product_type=cost_observability", () => {
    it("should create merchant with cost_observability product type", () => {
      const baseBody = { ...fixtures.merchantCreateBody };
      delete baseBody.merchant_id;
      const merchantCreateBody = {
        ...baseBody,
        product_type: "cost_observability",
      };

      cy.merchantCreateCallTest(merchantCreateBody, globalState, {
        expectedProductType: "cost_observability",
      });
    });

    it("should verify cost_observability product_type persists on retrieve", function () {
      const merchantId = globalState.get("merchantId");
      if (!merchantId) {
        this.skip();
      }
      cy.merchantRetrieveCall(globalState, {
        expectedProductType: "cost_observability",
      });
    });
  });

  context("Create merchant with product_type=dynamic_routing", () => {
    it("should create merchant with dynamic_routing product type", () => {
      const baseBody = { ...fixtures.merchantCreateBody };
      delete baseBody.merchant_id;
      const merchantCreateBody = {
        ...baseBody,
        product_type: "dynamic_routing",
      };

      cy.merchantCreateCallTest(merchantCreateBody, globalState, {
        expectedProductType: "dynamic_routing",
      });
    });

    it("should verify dynamic_routing product_type persists on retrieve", function () {
      const merchantId = globalState.get("merchantId");
      if (!merchantId) {
        this.skip();
      }
      cy.merchantRetrieveCall(globalState, {
        expectedProductType: "dynamic_routing",
      });
    });
  });

  context(
    "Create merchant without product_type (default to orchestration)",
    () => {
      it("should create merchant without product_type and default to orchestration", () => {
        const merchantCreateBody = { ...fixtures.merchantCreateBody };
        delete merchantCreateBody.merchant_id;

        cy.merchantCreateCallTest(merchantCreateBody, globalState, {
          expectedProductType: "orchestration",
        });
      });

      it("should verify orchestration product_type persists on retrieve", function () {
        const merchantId = globalState.get("merchantId");
        if (!merchantId) {
          this.skip();
        }
        cy.merchantRetrieveCall(globalState, {
          expectedProductType: "orchestration",
        });
      });
    }
  );

  context("Create merchant with invalid product_type (negative test)", () => {
    it("should return 400 error for invalid product_type value", () => {
      const baseBody = { ...fixtures.merchantCreateBody };
      delete baseBody.merchant_id;
      const merchantCreateBody = {
        ...baseBody,
        product_type: "invalid_product_type",
      };

      cy.merchantCreateCallTest(merchantCreateBody, globalState, {
        expectedStatus: 400,
        expectedErrorCode: "IR_06",
      });
    });
  });
});
