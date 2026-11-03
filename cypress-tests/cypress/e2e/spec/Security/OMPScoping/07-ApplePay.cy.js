import State from "../../../../utils/State";

// #15049 (Apple Pay half) - "enforce merchant and profile isolation".
// #15117 (V2 auth alignment for applepay-verified-domains) is V2-only and
// out of scope per the V2 decision - not covered here.
//
// Routes (verified): POST /verify/apple_pay/{merchant_id} (registration AND
// verification are the same endpoint - there is no separate "register"
// route in V1), GET /verify/applepay_verified_domains?merchant_id=&merchant_connector_account_id=.
// Both ApiKeyAuth or JWTAuth, cfg #[cfg(all(feature = "olap", feature = "v1"))].
let globalState;
let baseUrl;
let apiKeyM1;
let merchantIdM1;
let merchantIdM2;
let profileIdPA;
let jwtPA;
let mcaPaId;
let mcaPbId;

describe("OMP Scoping - Apple Pay (#15049)", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      baseUrl = globalState.get("baseUrl");
      apiKeyM1 = globalState.get("apiKeyM1");
      merchantIdM1 = globalState.get("merchantIdM1");
      merchantIdM2 = globalState.get("merchantIdM2");
      profileIdPA = globalState.get("profileIdPA");
      jwtPA = globalState.get("userInfoTokenPA");
      mcaPaId = globalState.get("mcaPaId");
      mcaPbId = globalState.get("mcaPbId");
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  it("TC-15049-10: verify/register rejected for a merchant_id the credential doesn't own", () => {
    cy.ompRequest({
      method: "POST",
      url: `${baseUrl}/verify/apple_pay/${merchantIdM2}`,
      auth: { type: "apiKey", apiKey: apiKeyM1 },
      body: {
        domain_names: ["example.com"],
        merchant_connector_account_id: mcaPaId,
      },
    }).then((res) => {
      expect(res.status).to.be.within(400, 499);
    });
  });

  it("TC-15049-12: domain registration against cross-profile MCA_PB denied", () => {
    cy.ompRequest({
      method: "POST",
      url: `${baseUrl}/verify/apple_pay/${merchantIdM1}`,
      auth: { type: "jwt", token: jwtPA, profileId: profileIdPA },
      body: {
        domain_names: ["example.com"],
        merchant_connector_account_id: mcaPbId,
      },
    }).then((res) => {
      expect(res.status).to.be.within(400, 499);
    });
  });

  it("TC-15049-11: verified-domains read for cross-profile MCA_PB denied", () => {
    cy.ompRequest({
      method: "GET",
      url: `${baseUrl}/verify/applepay_verified_domains?merchant_id=${merchantIdM1}&merchant_connector_account_id=${mcaPbId}`,
      auth: { type: "jwt", token: jwtPA, profileId: profileIdPA },
    }).then((res) => {
      expect(res.status).to.be.within(400, 499);
    });
  });

  it("TC-15049-14 (positive): verify/register and verified-domains read work for PA's own MCA_PA", () => {
    cy.ompRequest({
      method: "POST",
      url: `${baseUrl}/verify/apple_pay/${merchantIdM1}`,
      auth: { type: "jwt", token: jwtPA, profileId: profileIdPA },
      body: {
        domain_names: ["example.com"],
        merchant_connector_account_id: mcaPaId,
      },
    }).then((res) => {
      expect(res.status).to.not.eq(403);
    });

    cy.ompRequest({
      method: "GET",
      url: `${baseUrl}/verify/applepay_verified_domains?merchant_id=${merchantIdM1}&merchant_connector_account_id=${mcaPaId}`,
      auth: { type: "jwt", token: jwtPA, profileId: profileIdPA },
    }).then((res) => {
      expect(res.status).to.not.eq(403);
    });
  });

  it("TC-15049-E01: verified-domains read for a non-existent MCA returns 404", () => {
    cy.ompRequest({
      method: "GET",
      url: `${baseUrl}/verify/applepay_verified_domains?merchant_id=${merchantIdM1}&merchant_connector_account_id=mca_nonexistent_omp_test`,
      auth: { type: "jwt", token: jwtPA, profileId: profileIdPA },
    }).then((res) => {
      expect(res.status).to.eq(404);
    });
  });

  it("TC-15049-E04: invalid credential on verify/register is rejected", () => {
    cy.ompRequest({
      method: "POST",
      url: `${baseUrl}/verify/apple_pay/${merchantIdM1}`,
      auth: { type: "invalid" },
      body: {
        domain_names: ["example.com"],
        merchant_connector_account_id: mcaPaId,
      },
    }).then((res) => {
      expect(res.status).to.eq(401);
    });
  });
});
