import State from "../../../../utils/State";

// #14970 (evidence deletion) + #15098 (connector fetch), merged into one
// file per the endpoint-wise grouping decision rather than one file per PR.
//
// GAP (documented in 00-Setup.cy.js): this suite has no command that
// fabricates a dispute - disputes only exist from a real connector
// chargeback. D_PA / D_PB are looked up at runtime from whatever disputes
// already exist for this merchant; if none are found for a profile, that
// profile's tests skip, matching the existing 47-DisputeTests.cy.js
// convention in this repo rather than failing on missing fixtures.
//
// Routes (verified): DELETE /disputes/evidence, GET /disputes/{connector_id}/fetch
// (param is connector_id, i.e. the MCA id), GET /disputes/{dispute_id}
// (retrieve, for before/after snapshots) - all ApiKeyAuth(allow_connected_scope_operation)
// or JWTAuth(ProfileDispute{Read,Write}).
//
// CAUTION: TC-14970-02's positive control actually deletes real evidence on
// a real dispute. It's gated behind DELETE_REAL_DISPUTE_EVIDENCE=true (not
// set by default) so this file never destroys real sandbox data just by
// running - flip it on deliberately if you want that case exercised.
let globalState;
let baseUrl;
let apiKeyM1;
let profileIdPA;
let profileIdPB;
let mcaPaId;
let mcaPbId;
let disputePA;
let disputePB;

function findDisputeForProfile(disputes, profileId) {
  return (disputes || []).find((d) => d.profile_id === profileId);
}

describe("OMP Scoping - Disputes (#14970, #15098)", () => {
  before("seed global state and look up real disputes per profile", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      baseUrl = globalState.get("baseUrl");
      apiKeyM1 = globalState.get("apiKeyM1");
      profileIdPA = globalState.get("profileIdPA");
      profileIdPB = globalState.get("profileIdPB");
      mcaPaId = globalState.get("mcaPaId");
      mcaPbId = globalState.get("mcaPbId");
    });

    cy.then(() => {
      return cy.ompRequest({
        method: "GET",
        url: `${baseUrl}/disputes/list`,
        auth: { type: "apiKey", apiKey: apiKeyM1 },
      });
    }).then((res) => {
      const disputes = Array.isArray(res.body) ? res.body : res.body?.data;
      disputePA = findDisputeForProfile(disputes, profileIdPA);
      disputePB = findDisputeForProfile(disputes, profileIdPB);

      if (!disputePA || !disputePB) {
        cy.task(
          "cli_log",
          "No real dispute found for one or both profiles - dispute scoping " +
            "tests in this file will skip. See the GAP comment at the top of " +
            "this file."
        );
      }
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  function fetchDispute(disputeId) {
    return () =>
      cy.ompRequest({
        method: "GET",
        url: `${baseUrl}/disputes/${disputeId}`,
        auth: { type: "apiKey", apiKey: apiKeyM1 },
      });
  }

  context("Evidence deletion (#14970)", () => {
    beforeEach(function () {
      if (!disputePB) this.skip();
    });

    it("TC-14970-01: delete evidence on cross-profile D_PB denied, evidence unchanged", () => {
      cy.ompAssertDeniedAndUnchanged({
        fetch: fetchDispute(disputePB.dispute_id),
        negativeRequest: () =>
          cy.ompRequest({
            method: "DELETE",
            url: `${baseUrl}/disputes/evidence`,
            auth: { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
            body: { dispute_id: disputePB.dispute_id },
          }),
      });
    });

    it("TC-14970-02 (positive, destructive - opt-in only): delete evidence on own D_PA succeeds", function () {
      if (Cypress.env("DELETE_REAL_DISPUTE_EVIDENCE") !== true) {
        cy.task(
          "cli_log",
          "Skipping: set CYPRESS_DELETE_REAL_DISPUTE_EVIDENCE=true to run " +
            "this destructive positive control."
        );
        this.skip();
      }
      if (!disputePA) this.skip();

      cy.ompRequest({
        method: "DELETE",
        url: `${baseUrl}/disputes/evidence`,
        auth: { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        body: { dispute_id: disputePA.dispute_id },
      }).then((res) => {
        expect(res.status).to.eq(200);
      });
    });

    it("TC-14970-E01: deleting evidence on a non-existent dispute_id returns 404", () => {
      cy.ompRequest({
        method: "DELETE",
        url: `${baseUrl}/disputes/evidence`,
        auth: { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        body: { dispute_id: "dp_nonexistent_omp_test" },
      }).then((res) => {
        expect(res.status).to.eq(404);
      });
    });

    it("TC-14970-E03: malformed dispute_id returns 4xx, not 500", () => {
      cy.ompRequest({
        method: "DELETE",
        url: `${baseUrl}/disputes/evidence`,
        auth: { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        body: { dispute_id: "!!bad!!" },
      }).then((res) => {
        expect(res.status).to.be.within(400, 499);
      });
    });
  });

  context("Connector fetch (#15098)", () => {
    it("TC-15098-01: fetch disputes on cross-profile MCA_PB denied", () => {
      cy.ompRequest({
        method: "GET",
        url: `${baseUrl}/disputes/${mcaPbId}/fetch`,
        auth: { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
      }).then((res) => {
        expect(res.status).to.be.within(400, 499);
      });
    });

    it("TC-15098-02 (positive): fetch disputes on own MCA_PA isn't profile-rejected", () => {
      cy.ompRequest({
        method: "GET",
        url: `${baseUrl}/disputes/${mcaPaId}/fetch`,
        auth: { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
      }).then((res) => {
        expect(res.status).to.not.eq(403);
      });
    });

    it("TC-15098-E01: fetch on a non-existent MCA id is rejected, not 500", () => {
      // Server returns 400 (ID format/lookup validation) rather than 404
      // here, same pattern seen elsewhere in this folder.
      cy.ompRequest({
        method: "GET",
        url: `${baseUrl}/disputes/mca_nonexistent_omp_test/fetch`,
        auth: { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
      }).then((res) => {
        expect(res.status).to.be.within(400, 499);
      });
    });
  });
});
