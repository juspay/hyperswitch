import State from "../../../../utils/State";

// Covers cached profile reads and the connector-agnostic MIT toggle.
// Profile reads are cached; cached profile reads must still enforce
// merchant ownership, and the connector-agnostic MIT toggle must use the
// authenticated processor merchant for its profile lookup.
//
// Routes (verified): GET/POST /account/{account_id}/business_profile/{profile_id},
// POST .../toggle_connector_agnostic_mit - both ApiKeyAuth or JWTAuth,
// cfg #[cfg(all(feature = "olap", feature = "v1"))].
let globalState;
let apiKeyM1;
let apiKeyM2;
let merchantIdM1;
let merchantIdM2;
let profileIdPB;

describe("OMP Scoping - Business Profile", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      apiKeyM1 = globalState.get("apiKeyM1");
      apiKeyM2 = globalState.get("apiKeyM2");
      merchantIdM1 = globalState.get("merchantIdM1");
      merchantIdM2 = globalState.get("merchantIdM2");
      profileIdPB = globalState.get("profileIdPB");
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  function fetchProfilePB() {
    return () =>
      cy.ompRetrieveBusinessProfile(
        merchantIdM1,
        profileIdPB,
        { type: "apiKey", apiKey: apiKeyM1 },
        globalState
      );
  }

  it("cached profile read never leaks PB through M2's route/creds", () => {
    // Warm the cache via a legitimate M1 read first.
    cy.ompRetrieveBusinessProfile(
      merchantIdM1,
      profileIdPB,
      { type: "apiKey", apiKey: apiKeyM1 },
      globalState
    ).then((warmRes) => {
      expect(warmRes.status).to.eq(200);
    });

    cy.ompRetrieveBusinessProfile(
      merchantIdM2,
      profileIdPB,
      { type: "apiKey", apiKey: apiKeyM2 },
      globalState
    ).then((res) => {
      expect(res.status).to.be.within(400, 499);
      expect(res.body?.profile_id).to.not.eq(profileIdPB);
    });
  });

  it("MIT toggle blocked cross-merchant, PB toggle unchanged", () => {
    cy.ompAssertDeniedAndUnchanged({
      fetch: fetchProfilePB(),
      negativeRequest: () =>
        cy.ompToggleConnectorAgnosticMit(
          merchantIdM2,
          profileIdPB,
          { type: "apiKey", apiKey: apiKeyM1 },
          { enabled: true },
          globalState
        ),
    });
  });

  it("M1 can read and toggle PB's own profile (positive)", () => {
    cy.ompRetrieveBusinessProfile(
      merchantIdM1,
      profileIdPB,
      { type: "apiKey", apiKey: apiKeyM1 },
      globalState
    ).then((res) => {
      expect(res.status).to.eq(200);
    });

    cy.ompToggleConnectorAgnosticMit(
      merchantIdM1,
      profileIdPB,
      { type: "apiKey", apiKey: apiKeyM1 },
      { enabled: true },
      globalState
    ).then((res) => {
      expect(res.status).to.eq(200);
    });
  });

  it("retrieving a non-existent profile returns 404", () => {
    cy.ompRetrieveBusinessProfile(
      merchantIdM1,
      "pro_nonexistent_omp_test",
      { type: "apiKey", apiKey: apiKeyM1 },
      globalState
    ).then((res) => {
      expect(res.status).to.eq(404);
    });
  });

  it("invalid credential on profile read is rejected", () => {
    cy.ompRetrieveBusinessProfile(
      merchantIdM1,
      profileIdPB,
      { type: "invalid" },
      globalState
    ).then((res) => {
      expect(res.status).to.eq(401);
    });
  });
});
