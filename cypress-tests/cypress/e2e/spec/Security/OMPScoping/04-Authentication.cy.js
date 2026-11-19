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

// AuthenticationEligibilityRequest requires payment_method_data and
// payment_method (crates/api_models/src/authentication.rs) - an empty body
// fails deserialization (400) before the profile check ever runs, which
// would make every eligibility test below pass "by accident" regardless of
// whether the actual profile check works. A valid minimal card body makes
// sure any 4xx seen is attributable to the profile check, not this.
const eligibilityBody = {
  payment_method_data: {
    card: {
      card_number: "4242424242424242",
      card_exp_month: "12",
      card_exp_year: "28",
      card_cvc: "123",
    },
  },
  payment_method: "card",
};

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
        eligibilityBody,
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
        { ...eligibilityBody, profile_id: profileIdPB },
        globalState
      ).then((res) => {
        expect(res.status).to.be.within(400, 499);
      });
    });

    it("eligibility on own PA authentication succeeds (positive)", () => {
      // Expected 200 (the profile check passes correctly, which is this
      // test's actual purpose). PA's business profile has a netcetera
      // authentication_processor connector configured in 00-Setup.cy.js
      // specifically so eligibility can proceed past the profile check into
      // real 3DS processing, instead of failing early with 422 IR_29
      // "authentication_connector_details is not available in business
      // profile" (crates/router/src/core/authentication/utils.rs:603-610).
      cy.ompAuthenticationEligibility(
        authPAId,
        { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        eligibilityBody,
        globalState
      ).then((res) => {
        expect(res.status).to.eq(200);
      });
    });

    it("eligibility on a non-existent authentication ID is rejected, not 500", () => {
      // Server returns 400 here rather than 404 - ID format/lookup
      // validation runs before an existence check.
      cy.ompAuthenticationEligibility(
        "auth_nonexistent_omp_test",
        { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        eligibilityBody,
        globalState
      ).then((res) => {
        expect(res.status).to.be.within(400, 499);
      });
    });

    it("malformed authentication ID returns 4xx, not 500", () => {
      cy.ompAuthenticationEligibility(
        "!!bad-id!!",
        { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        eligibilityBody,
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
