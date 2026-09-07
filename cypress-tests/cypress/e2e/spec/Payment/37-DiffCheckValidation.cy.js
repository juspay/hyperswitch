import State from "../../../utils/State";

let globalState;

describe("Diff Check Result Validation Tests", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  it("diff-check-result-test", () => {
    cy.diffCheckResult(globalState);
  });
});
