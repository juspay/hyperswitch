import State from "../../../../utils/State";

// CONFIRMED GAP: relay_retrieve (crates/router/src/core/relay.rs) validates
// that the CALLER's own profile exists, but never compares the fetched
// relay record's merchant_id/profile_id against the authenticated context
// - find_relay_by_id queries by id alone. Un-encrypted metadata fields
// (status, connector_reference_id, error_code/message, timestamps) leak
// across profiles on the same merchant freely. Cross-merchant leakage of
// the encrypted request_data/response_data payload is only incidentally
// blocked by decryption failing with the wrong merchant's key - that's
// accidental protection, not an actual check. relay_create itself IS
// correctly profile-scoped (validates via
// find_business_profile_by_merchant_id_profile_id before creating) - only
// retrieve has the gap.
//
// GAP: this suite has no command that creates a relay record (it's
// created by connector-initiated webhook relay / out-of-band events, not
// a direct "create" API meant for this). The tests below look up whatever
// relay record already exists for M1 at runtime and skip if none found.
//
// Route (verified): GET /relay/{relay_id} - ApiKeyAuth.
let globalState;
let apiKeyM1;
let apiKeyM2;
let profileIdPA;
let profileIdPB;
let relayId;

describe("OMP Scoping - Relay", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      apiKeyM1 = globalState.get("apiKeyM1");
      apiKeyM2 = globalState.get("apiKeyM2");
      profileIdPA = globalState.get("profileIdPA");
      profileIdPB = globalState.get("profileIdPB");
      relayId = globalState.get("relayIdPA");

      if (!relayId) {
        cy.task(
          "cli_log",
          "Skipping this suite: no relayIdPA in globalState - this suite " +
            "has no command that creates a relay record, see the GAP " +
            "comment at the top of this file. Set it manually if testing " +
            "against an environment with real relay data."
        );
      }
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  beforeEach(function () {
    if (!relayId) this.skip();
  });

  it("PB retrieves PA's relay record (same merchant) - metadata leaks (confirmed gap)", () => {
    cy.ompRetrieveRelay(
      relayId,
      { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPB },
      globalState
    ).then((res) => {
      // No assertion of denial here deliberately - the audit found no
      // merchant/profile check on relay_retrieve at all. This documents
      // that un-encrypted metadata is returned regardless of profile.
      expect(
        res.status,
        "retrieve succeeds across profiles - no check exists"
      ).to.eq(200);
      cy.task(
        "cli_log",
        `CONFIRMED GAP: relay ${relayId} metadata (status=${res.body?.status}) ` +
          "was returned to profile PB even though it belongs to profile PA."
      );
    });
  });

  it("M2 retrieves M1's relay record (cross-merchant) - payload decrypt fails, metadata still returned", () => {
    cy.ompRetrieveRelay(
      relayId,
      { type: "apiKey", apiKey: apiKeyM2 },
      globalState
    ).then((res) => {
      cy.task(
        "cli_log",
        `Cross-merchant relay retrieve returned status ${res.status} - ` +
          "documenting actual response shape rather than asserting an " +
          "outcome, since encrypted-field decryption failure may surface " +
          "as a 200 with null payload fields, a 500, or a 4xx depending " +
          "on how the decrypt error propagates."
      );
    });
  });

  it("PA retrieves its own relay record (positive)", () => {
    cy.ompRetrieveRelay(
      relayId,
      { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
      globalState
    ).then((res) => {
      expect(res.status).to.eq(200);
    });
  });

  it("retrieving a non-existent relay_id returns 404", () => {
    cy.ompRetrieveRelay(
      "relay_nonexistent_omp_test",
      { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
      globalState
    ).then((res) => {
      expect(res.status).to.eq(404);
    });
  });

  it("invalid credential on relay retrieve is rejected", () => {
    cy.ompRetrieveRelay(relayId, { type: "invalid" }, globalState).then(
      (res) => {
        expect(res.status).to.eq(401);
      }
    );
  });
});
