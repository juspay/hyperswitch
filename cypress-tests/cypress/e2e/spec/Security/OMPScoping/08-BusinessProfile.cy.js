import State from "../../../../utils/State";
import { OMP_ERRORS } from "../../../configs/Security/Commons";

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
let profileIdPA;
let profileIdPB;
let jwtPA;

describe("OMP Scoping - Business Profile", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      apiKeyM1 = globalState.get("apiKeyM1");
      apiKeyM2 = globalState.get("apiKeyM2");
      merchantIdM1 = globalState.get("merchantIdM1");
      merchantIdM2 = globalState.get("merchantIdM2");
      profileIdPA = globalState.get("profileIdPA");
      profileIdPB = globalState.get("profileIdPB");
      jwtPA = globalState.get("userInfoTokenPA");
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

  it("update rejects cross-profile path, PB unchanged (JWT, same merchant)", () => {
    // Everything above is M1-vs-M2 (cross-merchant, API key). This is the
    // PA-vs-PB case within the SAME merchant - JWTAuthMerchantAndProfileFromRoute
    // requires the JWT's own profile to equal the path profile_id, so PA's
    // JWT must be denied here even though the account_id (M1) matches.
    cy.ompAssertDeniedAndUnchanged({
      fetch: fetchProfilePB(),
      negativeRequest: () =>
        cy.ompUpdateBusinessProfile(
          merchantIdM1,
          profileIdPB,
          { type: "jwt", token: jwtPA, profileId: profileIdPA },
          { profile_name: "Should be rejected" },
          globalState
        ),
      // Confirmed live - same generic JWT profile-mismatch path as the
      // Payments retrieve test, not specific to Business Profile.
      expectedError: OMP_ERRORS.JWT_PROFILE_MISMATCH,
    });
  });

  it("PA can update its own business profile (JWT, positive)", () => {
    cy.ompUpdateBusinessProfile(
      merchantIdM1,
      profileIdPA,
      { type: "jwt", token: jwtPA, profileId: profileIdPA },
      { profile_name: "OMP Scoping Test PA" },
      globalState
    ).then((res) => {
      expect(res.status).to.eq(200);
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
