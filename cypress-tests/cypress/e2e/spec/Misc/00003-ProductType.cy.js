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
      const baseBody = { ...fixtures.merchantCreateBody };
      delete baseBody.merchant_id;
      const merchantCreateBody = {
        ...baseBody,
        product_type: "vault",
      };

      // Create merchant with vault product_type
      cy.merchantCreateCallTest(merchantCreateBody, globalState, {
        expectedProductType: "vault",
      });

      // Verify product_type persists on retrieve
      cy.merchantRetrieveCallTest(globalState, {
        expectedProductType: "vault",
      });

      // Cleanup merchant
      cy.merchantDeleteCall(globalState);
    });
  });

  context("Create merchant with product_type=recon", () => {
    it("should create merchant with recon product type and verify persistence", () => {
      const baseBody = { ...fixtures.merchantCreateBody };
      delete baseBody.merchant_id;
      const merchantCreateBody = {
        ...baseBody,
        product_type: "recon",
      };

      cy.merchantCreateCallTest(merchantCreateBody, globalState, {
        expectedProductType: "recon",
      });

      cy.merchantRetrieveCallTest(globalState, {
        expectedProductType: "recon",
      });

      cy.merchantDeleteCall(globalState);
    });
  });

  context("Create merchant with product_type=recovery", () => {
    it("should create merchant with recovery product type and verify persistence", () => {
      const baseBody = { ...fixtures.merchantCreateBody };
      delete baseBody.merchant_id;
      const merchantCreateBody = {
        ...baseBody,
        product_type: "recovery",
      };

      cy.merchantCreateCallTest(merchantCreateBody, globalState, {
        expectedProductType: "recovery",
      });

      cy.merchantRetrieveCallTest(globalState, {
        expectedProductType: "recovery",
      });

      cy.merchantDeleteCall(globalState);
    });
  });

  context("Create merchant with product_type=cost_observability", () => {
    it("should create merchant with cost_observability product type and verify persistence", () => {
      const baseBody = { ...fixtures.merchantCreateBody };
      delete baseBody.merchant_id;
      const merchantCreateBody = {
        ...baseBody,
        product_type: "cost_observability",
      };

      cy.merchantCreateCallTest(merchantCreateBody, globalState, {
        expectedProductType: "cost_observability",
      });

      cy.merchantRetrieveCallTest(globalState, {
        expectedProductType: "cost_observability",
      });

      cy.merchantDeleteCall(globalState);
    });
  });

  context("Create merchant with product_type=dynamic_routing", () => {
    it("should create merchant with dynamic_routing product type and verify persistence", () => {
      const baseBody = { ...fixtures.merchantCreateBody };
      delete baseBody.merchant_id;
      const merchantCreateBody = {
        ...baseBody,
        product_type: "dynamic_routing",
      };

      cy.merchantCreateCallTest(merchantCreateBody, globalState, {
        expectedProductType: "dynamic_routing",
      });

      cy.merchantRetrieveCallTest(globalState, {
        expectedProductType: "dynamic_routing",
      });

      cy.merchantDeleteCall(globalState);
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

        cy.merchantRetrieveCallTest(globalState, {
          expectedProductType: "orchestration",
        });

        cy.merchantDeleteCall(globalState);
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

      cy.merchantCreateFailCall(merchantCreateBody, globalState, {
        expectedStatus: 400,
        expectedErrorCode: "IR_06",
      });
    });
  });
});
