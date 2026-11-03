import State from "../../../../utils/State";

// #15049 (cached profile read + MIT toggle halves) - "enforce merchant and
// profile isolation". Profile reads are cached; cached profile reads must
// still enforce merchant ownership, and the connector-agnostic MIT toggle
// must use the authenticated processor merchant for its profile lookup.
//
// Routes (verified): GET/POST /account/{account_id}/business_profile/{profile_id},
// POST .../toggle_connector_agnostic_mit - both ApiKeyAuth or JWTAuth,
// cfg #[cfg(all(feature = "olap", feature = "v1"))].
let globalState;
let baseUrl;
let apiKeyM1;
let apiKeyM2;
let merchantIdM1;
let merchantIdM2;
let profileIdPB;

describe("OMP Scoping - Business Profile (#15049)", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      baseUrl = globalState.get("baseUrl");
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
      cy.ompRequest({
        method: "GET",
        url: `${baseUrl}/account/${merchantIdM1}/business_profile/${profileIdPB}`,
        auth: { type: "apiKey", apiKey: apiKeyM1 },
      });
  }

  it("TC-15049-01: cached profile read never leaks PB through M2's route/creds", () => {
    // Warm the cache via a legitimate M1 read first.
    cy.ompRequest({
      method: "GET",
      url: `${baseUrl}/account/${merchantIdM1}/business_profile/${profileIdPB}`,
      auth: { type: "apiKey", apiKey: apiKeyM1 },
    }).then((warmRes) => {
      expect(warmRes.status).to.eq(200);
    });

    cy.ompRequest({
      method: "GET",
      url: `${baseUrl}/account/${merchantIdM2}/business_profile/${profileIdPB}`,
      auth: { type: "apiKey", apiKey: apiKeyM2 },
    }).then((res) => {
      expect(res.status).to.be.within(400, 499);
      expect(res.body?.profile_id).to.not.eq(profileIdPB);
    });
  });

  it("TC-15049-13: MIT toggle blocked cross-merchant, PB toggle unchanged", () => {
    cy.ompAssertDeniedAndUnchanged({
      fetch: fetchProfilePB(),
      negativeRequest: () =>
        cy.ompRequest({
          method: "POST",
          url: `${baseUrl}/account/${merchantIdM2}/business_profile/${profileIdPB}/toggle_connector_agnostic_mit`,
          auth: { type: "apiKey", apiKey: apiKeyM1 },
          body: { enabled: true },
        }),
    });
  });

  it("TC-15049-14 (positive): M1 can read and toggle PB's own profile", () => {
    cy.ompRequest({
      method: "GET",
      url: `${baseUrl}/account/${merchantIdM1}/business_profile/${profileIdPB}`,
      auth: { type: "apiKey", apiKey: apiKeyM1 },
    }).then((res) => {
      expect(res.status).to.eq(200);
    });

    cy.ompRequest({
      method: "POST",
      url: `${baseUrl}/account/${merchantIdM1}/business_profile/${profileIdPB}/toggle_connector_agnostic_mit`,
      auth: { type: "apiKey", apiKey: apiKeyM1 },
      body: { enabled: true },
    }).then((res) => {
      expect(res.status).to.eq(200);
    });
  });

  it("TC-15049-E01: retrieving a non-existent profile returns 404", () => {
    cy.ompRequest({
      method: "GET",
      url: `${baseUrl}/account/${merchantIdM1}/business_profile/pro_nonexistent_omp_test`,
      auth: { type: "apiKey", apiKey: apiKeyM1 },
    }).then((res) => {
      expect(res.status).to.eq(404);
    });
  });

  it("TC-15049-E04: invalid credential on profile read is rejected", () => {
    cy.ompRequest({
      method: "GET",
      url: `${baseUrl}/account/${merchantIdM1}/business_profile/${profileIdPB}`,
      auth: { type: "invalid" },
    }).then((res) => {
      expect(res.status).to.eq(401);
    });
  });
});
