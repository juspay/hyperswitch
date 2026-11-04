import State from "../../../../utils/State";

// "Remove deprecated ephemeral-key routes" - NOT MERGED at the time this
// suite was written, and not an OMP scoping fix (credential surface, not
// ownership logic) - kept skipped until that PR merges and deploys. Flip
// the outer describe.skip to describe once it has.
//
// The V2 /v2/client-secret positive control is out of scope entirely per
// the V2 decision for this effort, not just gated on this PR - leave it out
// even after unskipping the V1 checks below.
let globalState;
let apiKeyM1;

describe.skip("OMP Scoping - Ephemeral Keys - not merged yet", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      apiKeyM1 = globalState.get("apiKeyM1");
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  it("POST /ephemeral_keys is unavailable", () => {
    cy.ompCreateEphemeralKey(
      { type: "apiKey", apiKey: apiKeyM1 },
      { customer_id: "cus_omp_test" },
      globalState
    ).then((res) => {
      expect([404, 410]).to.include(res.status);
    });
  });

  it("DELETE /ephemeral_keys/{id} is unavailable", () => {
    cy.ompDeleteEphemeralKey(
      "ephkey_nonexistent_omp_test",
      { type: "apiKey", apiKey: apiKeyM1 },
      globalState
    ).then((res) => {
      expect([404, 410]).to.include(res.status);
    });
  });
});
