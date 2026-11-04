import State from "../../../../utils/State";

// These are standalone 3DS AuthenticationCreateRequest records
// (crates/api_models/src/authentication.rs), distinct from a payment's own
// authentication flow.
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
let apiKeyM1;
let merchantIdM1;
let profileIdPA;
let profileIdPB;

describe("OMP Scoping - Authentication", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
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
    it("create rejects explicit cross-profile request", () => {
      cy.ompCreateAuthentication(
        { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        { amount: 1000, currency: "USD", profile_id: profileIdPB },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(400);
        expect(res.body?.error?.code).to.eq("IR_16");
      });
    });

    it("create with no profile_id defaults to auth profile (positive)", () => {
      cy.ompCreateAuthentication(
        { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        { amount: 1000, currency: "USD" },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(200);
        expect(res.body.profile_id).to.eq(profileIdPA);
      });
    });
  });

  context("Eligibility / sync", () => {
    let authPBId;
    let authPAId;

    before("create one authentication record under each profile", () => {
      cy.ompCreateAuthentication(
        { type: "apiKey", apiKey: apiKeyM1 },
        { amount: 1000, currency: "USD", profile_id: profileIdPB },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(200);
        authPBId = res.body.authentication_id;
      });

      cy.ompCreateAuthentication(
        { type: "apiKey", apiKey: apiKeyM1 },
        { amount: 1000, currency: "USD", profile_id: profileIdPA },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(200);
        authPAId = res.body.authentication_id;
      });
    });

    it("eligibility on a PB authentication denied for PA auth", () => {
      cy.ompAuthenticationEligibility(
        authPBId,
        { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        {},
        globalState
      ).then((res) => {
        expect(res.status).to.be.within(400, 499);
      });
    });

    it("sync on a PB authentication denied for PA auth", () => {
      cy.ompAuthenticationSync(
        merchantIdM1,
        authPBId,
        { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        {},
        globalState
      ).then((res) => {
        expect(res.status).to.be.within(400, 499);
      });
    });

    it("eligibility rejects conflicting request profile", () => {
      cy.ompAuthenticationEligibility(
        authPAId,
        { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        { profile_id: profileIdPB },
        globalState
      ).then((res) => {
        expect(res.status).to.be.within(400, 499);
      });
    });

    it("eligibility on own PA authentication isn't profile-rejected (positive)", () => {
      cy.ompAuthenticationEligibility(
        authPAId,
        { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        {},
        globalState
      ).then((res) => {
        expect(res.status).to.not.eq(403);
      });
    });

    it("eligibility on a non-existent authentication ID is rejected, not 500", () => {
      // Server returns 400 here rather than 404 - ID format/lookup
      // validation runs before an existence check.
      cy.ompAuthenticationEligibility(
        "auth_nonexistent_omp_test",
        { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        {},
        globalState
      ).then((res) => {
        expect(res.status).to.be.within(400, 499);
      });
    });

    it("malformed authentication ID returns 4xx, not 500", () => {
      cy.ompAuthenticationEligibility(
        "!!bad-id!!",
        { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        {},
        globalState
      ).then((res) => {
        expect(res.status).to.be.within(400, 499);
      });
    });

    it("invalid credential on create is rejected", () => {
      cy.ompCreateAuthentication(
        { type: "invalid" },
        { amount: 1000, currency: "USD" },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(401);
      });
    });
  });
});
