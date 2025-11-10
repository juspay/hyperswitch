import State from "../../../utils/State";

let globalState;

describe("Customer List With Count - Pagination flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  it("customer-list-pagination-call-test", () => {
    cy.customerListPaginationCallTest(globalState);
  });
});
