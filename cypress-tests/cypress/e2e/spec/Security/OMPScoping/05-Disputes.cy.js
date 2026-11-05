import State from "../../../../utils/State";

// Covers both evidence deletion and connector dispute fetch, merged into
// one file per the endpoint-wise grouping decision rather than one file per
// fix.
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
// CAUTION: the "delete evidence on own D_PA succeeds" positive control
// actually deletes real evidence on a real dispute. It's gated behind
// DELETE_REAL_DISPUTE_EVIDENCE=true (not set by default) so this file never
// destroys real sandbox data just by running - flip it on deliberately if
// you want that case exercised.
let globalState;
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

describe("OMP Scoping - Disputes", () => {
  before("seed global state and look up real disputes per profile", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      apiKeyM1 = globalState.get("apiKeyM1");
      profileIdPA = globalState.get("profileIdPA");
      profileIdPB = globalState.get("profileIdPB");
      mcaPaId = globalState.get("mcaPaId");
      mcaPbId = globalState.get("mcaPbId");
    });

    cy.then(() => {
      return cy.ompListDisputes(
        { type: "apiKey", apiKey: apiKeyM1 },
        globalState
      );
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
      cy.ompRetrieveDispute(
        disputeId,
        { type: "apiKey", apiKey: apiKeyM1 },
        globalState
      );
  }

  context("Evidence deletion", () => {
    beforeEach(function () {
      if (!disputePB) this.skip();
    });

    it("delete evidence on cross-profile D_PB denied, evidence unchanged", () => {
      cy.ompAssertDeniedAndUnchanged({
        fetch: fetchDispute(disputePB.dispute_id),
        negativeRequest: () =>
          cy.ompDeleteDisputeEvidence(
            disputePB.dispute_id,
            { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
            globalState
          ),
      });
    });

    it("delete evidence on own D_PA succeeds (positive, destructive - opt-in only)", function () {
      if (Cypress.env("DELETE_REAL_DISPUTE_EVIDENCE") !== true) {
        cy.task(
          "cli_log",
          "Skipping: set CYPRESS_DELETE_REAL_DISPUTE_EVIDENCE=true to run " +
            "this destructive positive control."
        );
        this.skip();
      }
      if (!disputePA) this.skip();

      cy.ompDeleteDisputeEvidence(
        disputePA.dispute_id,
        { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(200);
      });
    });

    it("deleting evidence on a non-existent dispute_id returns 404", () => {
      cy.ompDeleteDisputeEvidence(
        "dp_nonexistent_omp_test",
        { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(404);
      });
    });

    it("malformed dispute_id returns 4xx, not 500", () => {
      cy.ompDeleteDisputeEvidence(
        "!!bad!!",
        { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        globalState
      ).then((res) => {
        expect(res.status).to.be.within(400, 499);
      });
    });

    it("invalid credential on evidence deletion is rejected", () => {
      cy.ompDeleteDisputeEvidence(
        "dp_nonexistent_omp_test",
        { type: "invalid" },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(401);
      });
    });
  });

  context("Connector fetch", () => {
    // ompFetchDisputesFromConnector supplies a default fetch_from/fetch_till
    // window itself - omitting those required query params returns 400
    // "missing field `fetch_from`" before auth even runs, which would make
    // every test below pass for the wrong reason regardless of profile or
    // credentials (confirmed live, same false-pass risk as the Connector
    // Onboarding PayPal-only check).
    it("fetch disputes on cross-profile MCA_PB denied", () => {
      cy.ompFetchDisputesFromConnector(
        mcaPbId,
        { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        globalState
      ).then((res) => {
        expect(res.status).to.be.within(400, 499);
      });
    });

    it("fetch disputes on own MCA_PA isn't profile-rejected (positive)", () => {
      // Can't assert a single exact success status here - the real
      // business outcome depends on whether the connector sandbox actually
      // has dispute data to fetch, which this suite doesn't control. What
      // this test actually has to prove is narrower: the profile check
      // passed, i.e. none of the denial statuses a cross-profile request
      // would get (401/403/404/412) showed up here.
      cy.ompFetchDisputesFromConnector(
        mcaPaId,
        { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        globalState
      ).then((res) => {
        expect([401, 403, 404, 412]).to.not.include(res.status);
      });
    });

    it("fetch on a non-existent MCA id is rejected, not 500", () => {
      // Server returns 400 (ID format/lookup validation) rather than 404
      // here, same pattern seen elsewhere in this folder.
      cy.ompFetchDisputesFromConnector(
        "mca_nonexistent_omp_test",
        { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        globalState
      ).then((res) => {
        expect(res.status).to.be.within(400, 499);
      });
    });

    it("invalid credential on connector fetch is rejected", () => {
      cy.ompFetchDisputesFromConnector(
        mcaPaId,
        { type: "invalid" },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(401);
      });
    });
  });
});
