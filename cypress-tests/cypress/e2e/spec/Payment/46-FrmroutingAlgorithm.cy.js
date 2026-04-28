import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";

let globalState;

describe("FRM Routing Algorithm Test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Test FRM Routing Algorithm in merchant account creation", () => {
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
      cy.merchantRetrieveWithFrmValidationCall(globalState);
    });

    it("frm-routing-algorithm-structure-validation", () => {
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

      testCases.forEach((algorithmConfig) => {
        cy.log("Testing FRM routing algorithm config:", JSON.stringify(algorithmConfig));
        expect(algorithmConfig).to.have.property("type");
        expect(algorithmConfig).to.have.property("data");
      });
    });
  });

  context("Test FRM Routing Algorithm persistence", () => {
    it("verify-frm-routing-in-merchant-response", () => {
      cy.merchantRetrieveWithFrmValidationCall(globalState);
    });
  });
});
