import State from "../../../../utils/State";
import { acquirerBody } from "../../../configs/Security/Commons";

// Create binds a profile JWT to the body's profile_id; update binds it to
// the path's profile_id. The merchant API key's legacy (unscoped) behavior
// is deliberately retained - see the "API key create is unscoped" case.
//
// Routes (verified against crates/router/src/routes/profile_acquirer.rs,
// app.rs): POST /profile_acquirer, POST /profile_acquirer/{profile_id}/{id},
// gated #[cfg(all(feature = "olap", feature = "v1"))] (default build), auth
// is JWTAuth{permission: ProfileAccountWrite} or ApiKeyAuth.
let globalState;
let apiKeyM1;
let profileIdPA;
let profileIdPB;
let jwtPA;

describe("OMP Scoping - Profile Acquirer", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      apiKeyM1 = globalState.get("apiKeyM1");
      profileIdPA = globalState.get("profileIdPA");
      profileIdPB = globalState.get("profileIdPB");
      jwtPA = globalState.get("userInfoTokenPA");
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  it("create with matching PA body profile succeeds (positive)", () => {
    cy.ompCreateProfileAcquirer(
      { type: "jwt", token: jwtPA, profileId: profileIdPA },
      acquirerBody(profileIdPA),
      globalState
    ).then((res) => {
      expect(res.status).to.eq(200);
      cy.wrap(res.body).as("acquirerPA");
    });
  });

  it("create rejects body profile mismatch", () => {
    cy.ompCreateProfileAcquirer(
      { type: "jwt", token: jwtPA, profileId: profileIdPA },
      acquirerBody(profileIdPB),
      globalState
    ).then((res) => {
      expect(res.status).to.eq(401);
      expect(res.body?.error?.code).to.eq("IR_17");
    });
  });

  it("API key create is unscoped (legacy behavior, positive/compat)", () => {
    cy.ompCreateProfileAcquirer(
      { type: "apiKey", apiKey: apiKeyM1 },
      acquirerBody(profileIdPB),
      globalState
    ).then((res) => {
      expect(res.status).to.eq(200);
    });
  });

  context("Update", () => {
    let acquirerIdPA;

    before("create a profile_acquirer under PA to update", () => {
      cy.ompCreateProfileAcquirer(
        { type: "apiKey", apiKey: apiKeyM1 },
        acquirerBody(profileIdPA),
        globalState
      ).then((res) => {
        expect(res.status).to.eq(200);
        acquirerIdPA = res.body.profile_acquirer_id;
      });
    });

    it("update with matching PA path profile succeeds (positive)", () => {
      // network is required on any update that isn't purely toggling
      // is_default (IR_06 otherwise).
      cy.ompUpdateProfileAcquirer(
        profileIdPA,
        acquirerIdPA,
        { type: "jwt", token: jwtPA, profileId: profileIdPA },
        { merchant_name: "Updated by PA", network: "Visa" },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(200);
      });
    });

    it("update rejects path profile mismatch", () => {
      // No confirmed GET/retrieve route exists for profile_acquirer, so this
      // only asserts the denial itself, not a before/after unchanged
      // comparison like other specs in this folder.
      cy.ompUpdateProfileAcquirer(
        profileIdPB,
        acquirerIdPA,
        { type: "jwt", token: jwtPA, profileId: profileIdPA },
        { merchant_name: "Should be rejected" },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(401);
        expect(res.body?.error?.code).to.eq("IR_17");
      });
    });

    it("update on non-existent profile_acquirer_id returns 404", () => {
      cy.ompUpdateProfileAcquirer(
        profileIdPA,
        "pacq_nonexistent_omp_test",
        { type: "jwt", token: jwtPA, profileId: profileIdPA },
        { merchant_name: "x" },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(404);
      });
    });

    it("invalid credential on update is rejected", () => {
      cy.ompUpdateProfileAcquirer(
        profileIdPA,
        acquirerIdPA,
        { type: "invalid" },
        { merchant_name: "x" },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(401);
      });
    });
  });

  context("Edge cases on create", () => {
    it("malformed profile_id in create body is rejected, not 500", () => {
      cy.ompCreateProfileAcquirer(
        { type: "jwt", token: jwtPA, profileId: profileIdPA },
        acquirerBody("!!not-a-valid-id!!"),
        globalState
      ).then((res) => {
        expect(res.status).to.be.within(400, 499);
      });
    });
  });
});
