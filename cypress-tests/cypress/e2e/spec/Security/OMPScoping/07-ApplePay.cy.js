import State from "../../../../utils/State";

// V2 auth alignment for applepay-verified-domains is V2-only and out of
// scope per the V2 decision - not covered here.
//
// Routes (verified): POST /verify/apple_pay/{merchant_id} (registration AND
// verification are the same endpoint - there is no separate "register"
// route in V1), GET /verify/applepay_verified_domains?merchant_id=&merchant_connector_account_id=.
// Both ApiKeyAuth or JWTAuth, cfg #[cfg(all(feature = "olap", feature = "v1"))].
let globalState;
let apiKeyM1;
let merchantIdM1;
let merchantIdM2;
let profileIdPA;
let jwtPA;
let mcaPaId;
let mcaPbId;

describe("OMP Scoping - Apple Pay", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
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

  it("verify/register rejected for a merchant_id the credential doesn't own", () => {
    cy.ompVerifyApplePay(
      merchantIdM2,
      { type: "apiKey", apiKey: apiKeyM1 },
      { domain_names: ["example.com"], merchant_connector_account_id: mcaPaId },
      globalState
    ).then((res) => {
      expect(res.status).to.be.within(400, 499);
    });
  });

  it("domain registration against cross-profile MCA_PB denied", () => {
    cy.ompVerifyApplePay(
      merchantIdM1,
      { type: "jwt", token: jwtPA, profileId: profileIdPA },
      { domain_names: ["example.com"], merchant_connector_account_id: mcaPbId },
      globalState
    ).then((res) => {
      expect(res.status).to.be.within(400, 499);
    });
  });

  it("verified-domains read for cross-profile MCA_PB denied", () => {
    cy.ompGetApplePayVerifiedDomains(
      merchantIdM1,
      mcaPbId,
      { type: "jwt", token: jwtPA, profileId: profileIdPA },
      globalState
    ).then((res) => {
      expect(res.status).to.be.within(400, 499);
    });
  });

  it("verify/register and verified-domains read work for PA's own MCA_PA (positive)", () => {
    cy.ompVerifyApplePay(
      merchantIdM1,
      { type: "jwt", token: jwtPA, profileId: profileIdPA },
      { domain_names: ["example.com"], merchant_connector_account_id: mcaPaId },
      globalState
    ).then((res) => {
      expect(res.status).to.not.eq(403);
    });

    cy.ompGetApplePayVerifiedDomains(
      merchantIdM1,
      mcaPaId,
      { type: "jwt", token: jwtPA, profileId: profileIdPA },
      globalState
    ).then((res) => {
      expect(res.status).to.not.eq(403);
    });
  });

  it("verified-domains read for a non-existent MCA returns 404", () => {
    cy.ompGetApplePayVerifiedDomains(
      merchantIdM1,
      "mca_nonexistent_omp_test",
      { type: "jwt", token: jwtPA, profileId: profileIdPA },
      globalState
    ).then((res) => {
      expect(res.status).to.eq(404);
    });
  });

  it("invalid credential on verify/register is rejected", () => {
    cy.ompVerifyApplePay(
      merchantIdM1,
      { type: "invalid" },
      { domain_names: ["example.com"], merchant_connector_account_id: mcaPaId },
      globalState
    ).then((res) => {
      expect(res.status).to.eq(401);
    });
  });
});
