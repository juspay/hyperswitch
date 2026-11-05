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

  it("create with JWT-PA + conflicting X-Profile-Id uses the JWT's own profile, not the header", () => {
    // Mirrors the GET test above for the write path: the fix must use the
    // JWT's own profile (PA) to scope where the entry is created, never the
    // conflicting header value (PB).
    const paOnlyBin = String(Date.now()).slice(-6);

    cy.ompCreateBlocklistEntry(
      { type: "jwt", token: jwtPA, profileId: profileIdPB },
      { type: "card_bin", data: paOnlyBin },
      globalState
    ).then((createRes) => {
      expect(createRes.status).to.eq(200);
      expect(createRes.body?.profile_id).to.eq(profileIdPA);
    });
  });

  it("delete rejects a bin that belongs to a different profile, entry unchanged", () => {
    // delete_entry_from_blocklist resolves profile_id from the caller
    // (header/JWT, defaulting if omitted) and queries by
    // (merchant_id, bin, profile_id) - a PA-scoped delete for a PB-owned
    // bin simply won't match that row (core/blocklist/utils.rs:513-524,
    // GenericNotFoundError), not because of an explicit profile check but
    // because the row lookup itself is profile-scoped. Confirm the entry
    // survives via /blocklist/lookup, which reports BlocklistResponse's
    // omitted raw bin value as a blocked:true/false flag instead.
    // Must be a plain 6-digit number - "card_bin" (unlike "generic_card_bin")
    // validates exactly 6 digits server-side (IR_05 otherwise, confirmed
    // live). Date.now() alone is enough to avoid colliding with the bin the
    // "JWT + conflicting X-Profile-Id" test above creates, since tests run
    // at different timestamps.
    const pbOnlyBin = String(Date.now()).slice(-6);

    cy.ompCreateBlocklistEntry(
      { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPB },
      { type: "card_bin", data: pbOnlyBin },
      globalState
    ).then((createRes) => {
      expect(createRes.status).to.eq(200);
    });

    cy.ompDeleteBlocklistEntry(
      { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
      { type: "card_bin", data: pbOnlyBin },
      globalState
    ).then((deleteRes) => {
      expect(deleteRes.status).to.eq(404);
    });

    cy.ompLookupBlocklist(
      { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPB },
      pbOnlyBin,
      globalState
    ).then((res) => {
      expect(res.status).to.eq(200);
      expect(res.body?.blocked).to.eq(true);
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
