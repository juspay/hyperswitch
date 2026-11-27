import State from "../../../../utils/State";

// The platform/connected-merchant mechanism (X-Connected-Merchant-Id header,
// lib.rs:171) is structurally present on most transactional tables
// (processor_merchant_id/platform_merchant_id columns, Sheet 2) but had
// never been exercised by any test in this suite. Source-verified: the
// authorization check DOES exist -
// validate_connected_merchant_account() (services/authentication.rs,
// ~6660-6672) - and requires BOTH (a) the target merchant's organization_id
// matches the authenticated platform merchant's own organization_id, AND
// (b) the target's merchant_account_type == Connected.
//
// The platform/connected/standard fixture pair is built once in
// 00-Setup.cy.js ("Platform + connected merchant pair") and reused here,
// and by every other platform-axis test across resources (Payments,
// Payouts, Dispute, Authentication, Blocklist, Webhook, Relay, Payment
// Link), so it's only created once per run.
let globalState;
let merchantIdM2;
let platformApiKey;
let connectedMerchantId;
let standardMerchantId;

describe("OMP Scoping - Platform / Connected merchant header", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      merchantIdM2 = globalState.get("merchantIdM2");
      platformApiKey = globalState.get("platformApiKey");
      connectedMerchantId = globalState.get("connectedMerchantId");
      standardMerchantId = globalState.get("standardMerchantId");
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  beforeEach(function () {
    if (!platformApiKey || !connectedMerchantId) {
      cy.task(
        "cli_log",
        "Skipping: platform/connected merchant fixture setup failed - " +
          "check whether state.conf.platform.enabled is on for this server."
      );
      this.skip();
    }
  });

  it("platform merchant acting as its own connected merchant succeeds (positive)", () => {
    cy.ompListPayments(
      {
        type: "apiKey",
        apiKey: platformApiKey,
        connectedMerchantId: connectedMerchantId,
      },
      globalState
    ).then((res) => {
      expect(res.status, "platform acting as its own connected merchant").to.eq(
        200
      );
    });
  });

  it("platform merchant CANNOT act as a merchant in a different organization", () => {
    cy.ompListPayments(
      {
        type: "apiKey",
        apiKey: platformApiKey,
        connectedMerchantId: merchantIdM2,
      },
      globalState
    ).then((res) => {
      expect(
        res.status,
        "cross-org connected-merchant header must be rejected"
      ).to.be.within(400, 499);
    });
  });

  it("platform merchant CANNOT act as a same-org Standard-type merchant via the header", () => {
    cy.ompListPayments(
      {
        type: "apiKey",
        apiKey: platformApiKey,
        connectedMerchantId: standardMerchantId,
      },
      globalState
    ).then((res) => {
      expect(
        res.status,
        "header must only resolve to a Connected-type merchant"
      ).to.be.within(400, 499);
    });
  });

  it("a Standard-type merchant sending the header is rejected, not silently ignored", () => {
    // M1 is an ordinary Standard merchant - sending
    // X-Connected-Merchant-Id should error (per
    // services/authentication.rs's Standard-merchant rejection branch), not
    // silently fall back to M1's own identity, since a silent fallback
    // would mask a client-side bug rather than surface it.
    const apiKeyM1 = globalState.get("apiKeyM1");
    cy.ompListPayments(
      {
        type: "apiKey",
        apiKey: apiKeyM1,
        connectedMerchantId: connectedMerchantId,
      },
      globalState
    ).then((res) => {
      expect(
        res.status,
        "a Standard merchant sending the connected-merchant header must be rejected"
      ).to.be.within(400, 499);
    });
  });
});
