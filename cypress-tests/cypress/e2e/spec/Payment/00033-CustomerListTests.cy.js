import State from "../../../utils/State";

let globalState;

describe("Customer List API Tests", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Customer List With Count - Various Limits", () => {
    it("should work with default limit (20)", () => {
      cy.customerListWithCountCallTest(globalState); // default 20, 0
    });

    it("should work with small limit (5)", () => {
      cy.customerListWithCountCallTest(globalState, 5, 0);
    });

    it("should work with limit 1 (edge case)", () => {
      cy.customerListWithCountCallTest(globalState, 1, 0);
    });

    it("should work with large limit (100)", () => {
      cy.customerListWithCountCallTest(globalState, 100, 0);
    });
    it("should handle offset beyond total_count", () => {
      cy.customerListWithCountCallTest(globalState, 10, 9999);
    });
  });
});
