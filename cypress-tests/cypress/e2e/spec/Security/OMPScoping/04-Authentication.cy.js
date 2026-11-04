import State from "../../../../utils/State";

// #15097 - "enforce authentication profile scope". These are standalone 3DS
// AuthenticationCreateRequest records (crates/api_models/src/authentication.rs),
// distinct from a payment's own authentication flow.
//
// Routes (verified): POST /authentication (ApiKeyAuth), POST
// /authentication/{id}/eligibility and POST
// /authentication/{merchant_id}/{id}/sync (SDK-aware auth, api-key capable).
// Eligibility/sync compare the authenticated profile against the stored
// record's profile BEFORE any connector (e.g. Netcetera) call - this repo
// has no Netcetera sandbox creds configured, so positive controls here only
// assert the call isn't rejected for profile-mismatch reasons, not that a
// full 3DS flow succeeds.
let globalState;
let baseUrl;
let apiKeyM1;
let merchantIdM1;
let profileIdPA;
let profileIdPB;

describe("OMP Scoping - Authentication (#15097)", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      baseUrl = globalState.get("baseUrl");
      apiKeyM1 = globalState.get("apiKeyM1");
      merchantIdM1 = globalState.get("merchantIdM1");
      profileIdPA = globalState.get("profileIdPA");
      profileIdPB = globalState.get("profileIdPB");
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Create", () => {
    // POST /authentication is ApiKeyAuth-only (HeaderAuth(ApiKeyAuth)) - no
    // JWT alternative, unlike eligibility/sync below.
    it("TC-15097-01: create rejects explicit cross-profile request", () => {
      cy.ompRequest({
        method: "POST",
        url: `${baseUrl}/authentication`,
        auth: { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        body: { amount: 1000, currency: "USD", profile_id: profileIdPB },
      }).then((res) => {
        expect(res.status).to.eq(400);
        expect(res.body?.error?.code).to.eq("IR_16");
      });
    });

    it("TC-15097-02 (positive): create with no profile_id defaults to auth profile", () => {
      cy.ompRequest({
        method: "POST",
        url: `${baseUrl}/authentication`,
        auth: { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        body: { amount: 1000, currency: "USD" },
      }).then((res) => {
        expect(res.status).to.eq(200);
        expect(res.body.profile_id).to.eq(profileIdPA);
      });
    });
  });

  context("Eligibility / sync", () => {
    let authPBId;
    let authPAId;

    before("create one authentication record under each profile", () => {
      cy.ompRequest({
        method: "POST",
        url: `${baseUrl}/authentication`,
        auth: { type: "apiKey", apiKey: apiKeyM1 },
        body: { amount: 1000, currency: "USD", profile_id: profileIdPB },
      }).then((res) => {
        expect(res.status).to.eq(200);
        authPBId = res.body.authentication_id;
      });

      cy.ompRequest({
        method: "POST",
        url: `${baseUrl}/authentication`,
        auth: { type: "apiKey", apiKey: apiKeyM1 },
        body: { amount: 1000, currency: "USD", profile_id: profileIdPA },
      }).then((res) => {
        expect(res.status).to.eq(200);
        authPAId = res.body.authentication_id;
      });
    });

    it("TC-15097-03: eligibility on a PB authentication denied for PA auth", () => {
      cy.ompRequest({
        method: "POST",
        url: `${baseUrl}/authentication/${authPBId}/eligibility`,
        auth: { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        body: {},
      }).then((res) => {
        expect(res.status).to.be.within(400, 499);
      });
    });

    it("TC-15097-04: sync on a PB authentication denied for PA auth", () => {
      cy.ompRequest({
        method: "POST",
        url: `${baseUrl}/authentication/${merchantIdM1}/${authPBId}/sync`,
        auth: { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        body: {},
      }).then((res) => {
        expect(res.status).to.be.within(400, 499);
      });
    });

    it("TC-15097-05: eligibility rejects conflicting request profile", () => {
      cy.ompRequest({
        method: "POST",
        url: `${baseUrl}/authentication/${authPAId}/eligibility`,
        auth: { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        body: { profile_id: profileIdPB },
      }).then((res) => {
        expect(res.status).to.be.within(400, 499);
      });
    });

    it("TC-15097-06 (positive): eligibility on own PA authentication isn't profile-rejected", () => {
      cy.ompRequest({
        method: "POST",
        url: `${baseUrl}/authentication/${authPAId}/eligibility`,
        auth: { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        body: {},
      }).then((res) => {
        expect(res.status).to.not.eq(403);
      });
    });

    it("TC-15097-E01: eligibility on a non-existent authentication ID is rejected, not 500", () => {
      // Server returns 400 here rather than 404 - ID format/lookup
      // validation runs before an existence check.
      cy.ompRequest({
        method: "POST",
        url: `${baseUrl}/authentication/auth_nonexistent_omp_test/eligibility`,
        auth: { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        body: {},
      }).then((res) => {
        expect(res.status).to.be.within(400, 499);
      });
    });

    it("TC-15097-E03: malformed authentication ID returns 4xx, not 500", () => {
      cy.ompRequest({
        method: "POST",
        url: `${baseUrl}/authentication/!!bad-id!!/eligibility`,
        auth: { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        body: {},
      }).then((res) => {
        expect(res.status).to.be.within(400, 499);
      });
    });

    it("TC-15097-E04: invalid credential on create is rejected", () => {
      cy.ompRequest({
        method: "POST",
        url: `${baseUrl}/authentication`,
        auth: { type: "invalid" },
        body: { amount: 1000, currency: "USD" },
      }).then((res) => {
        expect(res.status).to.eq(401);
      });
    });
  });
});
