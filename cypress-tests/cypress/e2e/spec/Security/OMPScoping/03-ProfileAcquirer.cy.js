import State from "../../../../utils/State";

// #15093 - "enforce JWT profile scope for acquirer writes". Create binds a
// profile JWT to the body's profile_id; update binds it to the path's
// profile_id. The merchant API key's legacy (unscoped) behavior is
// deliberately retained - see TC-15093-05.
//
// Routes (verified against crates/router/src/routes/profile_acquirer.rs,
// app.rs): POST /profile_acquirer, POST /profile_acquirer/{profile_id}/{id},
// gated #[cfg(all(feature = "olap", feature = "v1"))] (default build), auth
// is JWTAuth{permission: ProfileAccountWrite} or ApiKeyAuth.
let globalState;
let baseUrl;
let apiKeyM1;
let profileIdPA;
let profileIdPB;
let jwtPA;

function acquirerBody(profileId, overrides = {}) {
  return {
    acquirer_assigned_merchant_id: "omp_test_merchant_id",
    merchant_name: "OMP Scoping Test Merchant",
    network: "Visa",
    acquirer_bin: "123456",
    profile_id: profileId,
    ...overrides,
  };
}

describe("OMP Scoping - Profile Acquirer (#15093)", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      baseUrl = globalState.get("baseUrl");
      apiKeyM1 = globalState.get("apiKeyM1");
      profileIdPA = globalState.get("profileIdPA");
      profileIdPB = globalState.get("profileIdPB");
      jwtPA = globalState.get("userInfoTokenPA");
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  it("TC-15093-03 (positive): create with matching PA body profile succeeds", () => {
    cy.ompRequest({
      method: "POST",
      url: `${baseUrl}/profile_acquirer`,
      auth: { type: "jwt", token: jwtPA, profileId: profileIdPA },
      body: acquirerBody(profileIdPA),
    }).then((res) => {
      expect(res.status).to.eq(200);
      cy.wrap(res.body).as("acquirerPA");
    });
  });

  it("TC-15093-01: create rejects body profile mismatch", () => {
    cy.ompRequest({
      method: "POST",
      url: `${baseUrl}/profile_acquirer`,
      auth: { type: "jwt", token: jwtPA, profileId: profileIdPA },
      body: acquirerBody(profileIdPB),
    }).then((res) => {
      expect(res.status).to.eq(401);
      expect(res.body?.error?.code).to.eq("IR_17");
    });
  });

  it("TC-15093-05 (positive/compat): API key create is unscoped (legacy behavior)", () => {
    cy.ompRequest({
      method: "POST",
      url: `${baseUrl}/profile_acquirer`,
      auth: { type: "apiKey", apiKey: apiKeyM1 },
      body: acquirerBody(profileIdPB),
    }).then((res) => {
      expect(res.status).to.eq(200);
    });
  });

  context("Update", () => {
    let acquirerIdPA;

    before("create a profile_acquirer under PA to update", () => {
      cy.ompRequest({
        method: "POST",
        url: `${baseUrl}/profile_acquirer`,
        auth: { type: "apiKey", apiKey: apiKeyM1 },
        body: acquirerBody(profileIdPA),
      }).then((res) => {
        expect(res.status).to.eq(200);
        acquirerIdPA = res.body.profile_acquirer_id;
      });
    });

    it("TC-15093-04 (positive): update with matching PA path profile succeeds", () => {
      // network is required on any update that isn't purely toggling
      // is_default (IR_06 otherwise).
      cy.ompRequest({
        method: "POST",
        url: `${baseUrl}/profile_acquirer/${profileIdPA}/${acquirerIdPA}`,
        auth: { type: "jwt", token: jwtPA, profileId: profileIdPA },
        body: { merchant_name: "Updated by PA", network: "Visa" },
      }).then((res) => {
        expect(res.status).to.eq(200);
      });
    });

    it("TC-15093-02: update rejects path profile mismatch", () => {
      // No confirmed GET/retrieve route exists for profile_acquirer, so this
      // only asserts the denial itself, not a before/after unchanged
      // comparison like other specs in this folder.
      cy.ompRequest({
        method: "POST",
        url: `${baseUrl}/profile_acquirer/${profileIdPB}/${acquirerIdPA}`,
        auth: { type: "jwt", token: jwtPA, profileId: profileIdPA },
        body: { merchant_name: "Should be rejected" },
      }).then((res) => {
        expect(res.status).to.eq(401);
        expect(res.body?.error?.code).to.eq("IR_17");
      });
    });

    it("TC-15093-E01: update on non-existent profile_acquirer_id returns 404", () => {
      cy.ompRequest({
        method: "POST",
        url: `${baseUrl}/profile_acquirer/${profileIdPA}/pacq_nonexistent_omp_test`,
        auth: { type: "jwt", token: jwtPA, profileId: profileIdPA },
        body: { merchant_name: "x" },
      }).then((res) => {
        expect(res.status).to.eq(404);
      });
    });

    it("TC-15093-E04: invalid credential on update is rejected", () => {
      cy.ompRequest({
        method: "POST",
        url: `${baseUrl}/profile_acquirer/${profileIdPA}/${acquirerIdPA}`,
        auth: { type: "invalid" },
        body: { merchant_name: "x" },
      }).then((res) => {
        expect(res.status).to.eq(401);
      });
    });
  });

  context("Edge cases on create", () => {
    it("TC-15093-E03: malformed profile_id in create body is rejected, not 500", () => {
      cy.ompRequest({
        method: "POST",
        url: `${baseUrl}/profile_acquirer`,
        auth: { type: "jwt", token: jwtPA, profileId: profileIdPA },
        body: acquirerBody("!!not-a-valid-id!!"),
      }).then((res) => {
        expect(res.status).to.be.within(400, 499);
      });
    });
  });
});
