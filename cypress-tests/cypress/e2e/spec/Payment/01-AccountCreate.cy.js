import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";

let globalState;
describe("Account Create flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  it("merchant-create-call-test", () => {
    cy.merchantCreateCallTest(fixtures.merchantCreateBody, globalState);
  });

  it("api-key-create-call-test", () => {
    cy.apiKeyCreateTest(fixtures.apiKeyCreateBody, globalState);
  });

  it("create-shadow-config-if-shadow-mode-enabled", () => {
    // Shadow and rollout configs are now merged - create unified config with execution_mode: "shadow"
    cy.createRolloutConfig(globalState);
  });
});
