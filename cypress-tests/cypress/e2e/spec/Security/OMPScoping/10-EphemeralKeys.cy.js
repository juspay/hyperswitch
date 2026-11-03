import State from "../../../../utils/State";

// #15099 - "remove deprecated ephemeral-key routes". NOT MERGED at the time
// this suite was written, and not an OMP scoping fix (credential surface,
// not ownership logic) - kept skipped until the PR merges and deploys.
// Flip the outer describe.skip to describe once it has.
//
// The V2 /v2/client-secret positive control (TC-15099-04) is out of scope
// entirely per the V2 decision for this effort, not just gated on this PR -
// leave it out even after unskipping the V1 checks below.
let globalState;
let baseUrl;
let apiKeyM1;

describe.skip("OMP Scoping - Ephemeral Keys (#15099) - not merged yet", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      baseUrl = globalState.get("baseUrl");
      apiKeyM1 = globalState.get("apiKeyM1");
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  it("TC-15099-01: POST /ephemeral_keys is unavailable", () => {
    cy.ompRequest({
      method: "POST",
      url: `${baseUrl}/ephemeral_keys`,
      auth: { type: "apiKey", apiKey: apiKeyM1 },
      body: { customer_id: "cus_omp_test" },
    }).then((res) => {
      expect([404, 410]).to.include(res.status);
    });
  });

  it("TC-15099-02: DELETE /ephemeral_keys/{id} is unavailable", () => {
    cy.ompRequest({
      method: "DELETE",
      url: `${baseUrl}/ephemeral_keys/ephkey_nonexistent_omp_test`,
      auth: { type: "apiKey", apiKey: apiKeyM1 },
    }).then((res) => {
      expect([404, 410]).to.include(res.status);
    });
  });
});
