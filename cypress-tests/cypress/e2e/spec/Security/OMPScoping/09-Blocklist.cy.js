import State from "../../../../utils/State";

// GET /blocklist no longer accepts publishable-key/client-secret SDK auth;
// a JWT listing uses the JWT's own profile rather than a conflicting
// X-Profile-Id header; an API key with no resolved profile stays
// merchant-wide (legacy behavior).
//
// Route (verified): GET/POST/DELETE /blocklist, cfg
// #[cfg(all(feature = "olap", feature = "v1"))], auth is
// check_sdk_auth_and_get_auth (dynamic: api-key, or publishable
// key/client-secret SDK auth) by default - GET is narrowed specifically.
let globalState;
let apiKeyM1;
let publishableKeyM1;
let profileIdPA;
let profileIdPB;
let jwtPA;

describe("OMP Scoping - Blocklist", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      apiKeyM1 = globalState.get("apiKeyM1");
      profileIdPA = globalState.get("profileIdPA");
      profileIdPB = globalState.get("profileIdPB");
      jwtPA = globalState.get("userInfoTokenPA");
    });

    // publishableKeyM1 is only set by merchantCreateCallTest, which M1
    // (created via the dashboard signup flow in 00-Setup.cy.js) never went
    // through - fetch it from the merchant account directly.
    cy.then(() => {
      return cy.ompGetMerchantAccount(
        globalState.get("merchantIdM1"),
        globalState
      );
    }).then((res) => {
      publishableKeyM1 = res.body?.publishable_key;
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  it("publishable-key/client-secret SDK auth is rejected on GET", () => {
    cy.ompGetBlocklist(
      { type: "apiKey", apiKey: publishableKeyM1 },
      "?data_kind=card_bin",
      globalState
    ).then((res) => {
      expect(res.status).to.be.within(400, 499);
    });
  });

  it("JWT + conflicting X-Profile-Id never returns the other profile's entries", () => {
    // Must be unique per run - the server rejects re-blocking a bin that's
    // already blocked (IR_16 "provided bin is already blocked"), so a fixed
    // value collides across repeated test runs.
    const pbOnlyBin = String(Date.now()).slice(-6);

    cy.ompCreateBlocklistEntry(
      { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPB },
      { type: "card_bin", data: pbOnlyBin },
      globalState
    ).then((createRes) => {
      expect(createRes.status).to.eq(200);
    });

    // JWT scoped to PA, but the header asks for PB - the fix must use the
    // JWT's own profile (PA), never PB's. The list response
    // (ListBlocklistResponse) is { count, total_count, data: [...] }, and
    // each entry (BlocklistResponse) carries profile_id but not the raw
    // bin value - that's the field this check actually has to assert on.
    cy.ompGetBlocklist(
      { type: "jwt", token: jwtPA, profileId: profileIdPB },
      "?data_kind=card_bin",
      globalState
    ).then((res) => {
      expect(res.status).to.eq(200);
      const returnedProfileIds = (res.body?.data || []).map(
        (entry) => entry.profile_id
      );
      expect(returnedProfileIds).to.not.include(profileIdPB);
    });
  });

  it("merchant API key with no profile header stays merchant-wide (positive/compat)", () => {
    cy.ompGetBlocklist(
      { type: "apiKey", apiKey: apiKeyM1 },
      "?data_kind=card_bin",
      globalState
    ).then((res) => {
      expect(res.status).to.eq(200);
    });
  });

  it("malformed query parameters return 4xx, not 500", () => {
    cy.ompGetBlocklist(
      { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
      "?limit=abc&offset=-1",
      globalState
    ).then((res) => {
      expect(res.status).to.be.within(400, 499);
    });
  });

  it("invalid credential on GET is rejected", () => {
    cy.ompGetBlocklist(
      { type: "invalid" },
      "?data_kind=card_bin",
      globalState
    ).then((res) => {
      expect(res.status).to.eq(401);
    });
  });
});
