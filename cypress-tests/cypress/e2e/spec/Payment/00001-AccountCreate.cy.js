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

  it("create-ucs-configs-based-on-mode", () => {
    // Automatically create UCS configs based on UCS_MODE environment variable
    // If UCS_MODE not set, creates both rollout and shadow configs (backward compatible)
    cy.createUcsConfigsByMode(globalState);
  });
});
