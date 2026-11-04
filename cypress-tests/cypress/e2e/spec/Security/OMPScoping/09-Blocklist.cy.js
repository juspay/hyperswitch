import State from "../../../../utils/State";

// #14985 - "blocklist management API handling". GET /blocklist no longer
// accepts publishable-key/client-secret SDK auth; a JWT listing uses the
// JWT's own profile rather than a conflicting X-Profile-Id header; an API
// key with no resolved profile stays merchant-wide (legacy behavior).
//
// Route (verified): GET/POST/DELETE /blocklist, cfg
// #[cfg(all(feature = "olap", feature = "v1"))], auth is
// check_sdk_auth_and_get_auth (dynamic: api-key, or publishable
// key/client-secret SDK auth) by default - this PR narrows GET specifically.
let globalState;
let baseUrl;
let apiKeyM1;
let publishableKeyM1;
let profileIdPA;
let profileIdPB;
let jwtPA;

describe("OMP Scoping - Blocklist (#14985)", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      baseUrl = globalState.get("baseUrl");
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

  it("TC-14985-01: publishable-key/client-secret SDK auth is rejected on GET", () => {
    cy.ompRequest({
      method: "GET",
      url: `${baseUrl}/blocklist?data_kind=card_bin`,
      auth: { type: "apiKey", apiKey: publishableKeyM1 },
    }).then((res) => {
      expect(res.status).to.be.within(400, 499);
    });
  });

  it("TC-14985-02: JWT + conflicting X-Profile-Id never returns the other profile's entries", () => {
    // Must be unique per run - the server rejects re-blocking a bin that's
    // already blocked (IR_16 "provided bin is already blocked"), so a fixed
    // value collides across repeated test runs.
    const pbOnlyBin = String(Date.now()).slice(-6);

    cy.ompRequest({
      method: "POST",
      url: `${baseUrl}/blocklist`,
      auth: { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPB },
      body: { type: "card_bin", data: pbOnlyBin },
    }).then((createRes) => {
      expect(createRes.status).to.eq(200);
    });

    // JWT scoped to PA, but the header asks for PB - the fix must use the
    // JWT's own profile (PA), never PB's. The list response
    // (ListBlocklistResponse) is { count, total_count, data: [...] }, and
    // each entry (BlocklistResponse) carries profile_id but not the raw
    // bin value - that's the field this check actually has to assert on.
    cy.ompRequest({
      method: "GET",
      url: `${baseUrl}/blocklist?data_kind=card_bin`,
      auth: { type: "jwt", token: jwtPA, profileId: profileIdPB },
    }).then((res) => {
      expect(res.status).to.eq(200);
      const returnedProfileIds = (res.body?.data || []).map(
        (entry) => entry.profile_id
      );
      expect(returnedProfileIds).to.not.include(profileIdPB);
    });
  });

  it("TC-14985-03 (positive/compat): merchant API key with no profile header stays merchant-wide", () => {
    cy.ompRequest({
      method: "GET",
      url: `${baseUrl}/blocklist?data_kind=card_bin`,
      auth: { type: "apiKey", apiKey: apiKeyM1 },
    }).then((res) => {
      expect(res.status).to.eq(200);
    });
  });

  it("TC-14985-E01: malformed query parameters return 4xx, not 500", () => {
    cy.ompRequest({
      method: "GET",
      url: `${baseUrl}/blocklist?limit=abc&offset=-1`,
      auth: { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
    }).then((res) => {
      expect(res.status).to.be.within(400, 499);
    });
  });

  it("TC-14985-E02: invalid credential on GET is rejected", () => {
    cy.ompRequest({
      method: "GET",
      url: `${baseUrl}/blocklist?data_kind=card_bin`,
      auth: { type: "invalid" },
    }).then((res) => {
      expect(res.status).to.eq(401);
    });
  });
});
