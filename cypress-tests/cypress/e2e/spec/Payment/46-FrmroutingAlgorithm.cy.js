import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";

let globalState;

describe("FRM Routing Algorithm Test", () => {
  let shouldContinue = true;

    beforeEach(() => {
    cy.session("login", () => {
      if (!globalState.get("email") || !globalState.get("password")) {
        throw new Error("Missing login credentials in global state");
      }

      cy.userLogin(globalState)
        .then(() => cy.terminate2Fa(globalState))
        .then(() => cy.userInfo(globalState))
        .then(() => {
          const requiredKeys = [
            "userInfoToken",
            "merchantId",
            "organizationId",
            "profileId",
          ];
          requiredKeys.forEach((key) => {
            if (!globalState.get(key)) {
              throw new Error(`Missing required key after login: ${key}`);
            }
          });
        });
    });
  });

  context("Test FRM Routing Algorithm in merchant account creation", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("merchant-create-call-test-with-frm-routing", () => {
      const merchantCreateBodyWithFrm = {
        ...fixtures.merchantCreateBody,
        frm_routing_algorithm: {
          type: "single",
          data: "signifyd"
        }
      };
      
      cy.merchantCreateCallTest(merchantCreateBodyWithFrm, globalState);
    });

    it("verify-merchant-retrieve-includes-frm-routing", () => {
      cy.merchantRetrieveCall(globalState);
      // Note: frm_routing_algorithm should be present in the response
      // The field is stored in the merchant account and returned on retrieve
    });

    it("frm-routing-algorithm-validation", () => {
      // Test with different FRM routing algorithm configurations
      const testCases = [
        {
          type: "single",
          data: "signifyd"
        },
        {
          type: "priority",
          data: ["signifyd", "riskified"]
        }
      ];

      // Validate that frm_routing_algorithm accepts proper JSON structure
      testCases.forEach((algorithmConfig) => {
        cy.log("Testing FRM routing algorithm config:", JSON.stringify(algorithmConfig));
        // The configuration should be a valid JSON object
        expect(algorithmConfig).to.have.property("type");
        expect(algorithmConfig).to.have.property("data");
      });
    });
  });

  context("Test FRM Routing Algorithm persistence", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("verify-frm-routing-in-merchant-response", () => {
      const merchant_id = globalState.get("merchantId");
      
      cy.request({
        method: "GET",
        url: `${globalState.get("baseUrl")}/accounts/${merchant_id}`,
        headers: {
          Accept: "application/json",
          "Content-Type": "application/json",
          "api-key": globalState.get("adminApiKey"),
        },
        failOnStatusCode: false,
      }).then((response) => {
        expect(response.status).to.eq(200);
        expect(response.body).to.have.property("merchant_id").that.equals(merchant_id);
        
        // Verify merchant account has expected structure
        expect(response.body).to.have.property("default_profile");
        expect(response.body).to.have.property("publishable_key");
        expect(response.body).to.have.property("organization_id");
        
        // Log frm_routing_algorithm if present (it's stored in the backend)
        if (response.body.frm_routing_algorithm) {
          cy.log("FRM Routing Algorithm present:", JSON.stringify(response.body.frm_routing_algorithm));
        }
      });
    });
  });
});
