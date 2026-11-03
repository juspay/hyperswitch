import State from "../../../../utils/State";

// #15049 (payout half) - "enforce merchant and profile isolation". PO_PA /
// PO_PB are payouts under M1, one per profile, created in 00-Setup.cy.js
// (skipped there if the configured payout connector has no credentials -
// gate on the same payoutsExecution flag here, matching the existing
// Payout/*.cy.js convention in this repo).
//
// update/confirm/cancel/fulfill are ApiKeyAuth-only routes (verified against
// crates/router/src/routes/app.rs) - there is no JWT alternative, so these
// negative tests use the merchant API key plus X-Profile-Id, the same
// combination the PR text itself uses for payouts.
let globalState;
let baseUrl;
let apiKeyM1;
let merchantIdM2;
let profileIdPA;
let profileIdPB;
let payoutIdPA;
let payoutIdPB;
let payoutsExecution;

describe("OMP Scoping - Payouts (#15049)", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      baseUrl = globalState.get("baseUrl");
      apiKeyM1 = globalState.get("apiKeyM1");
      merchantIdM2 = globalState.get("merchantIdM2");
      profileIdPA = globalState.get("profileIdPA");
      profileIdPB = globalState.get("profileIdPB");
      payoutIdPA = globalState.get("payoutIdPA");
      payoutIdPB = globalState.get("payoutIdPB");
      payoutsExecution = globalState.get("payoutsExecution");
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  beforeEach(function () {
    if (!payoutsExecution) {
      cy.task(
        "cli_log",
        "Skipping: no payout_processor credentials in the creds file (see 00-Setup.cy.js)."
      );
      this.skip();
    }
  });

  // Ground-truth snapshot, deliberately merchant-wide (no X-Profile-Id) so
  // comparing PB's "before"/"after" state isn't trivially masked by PA's
  // own denied access to it.
  function fetchPayout(payoutId) {
    return () =>
      cy.ompRequest({
        method: "GET",
        url: `${baseUrl}/payouts/${payoutId}`,
        auth: { type: "apiKey", apiKey: apiKeyM1 },
      });
  }

  context("Update / confirm / cancel / fulfill - cross-profile denial", () => {
    it("TC-15049-02: payout update blocked cross-profile", () => {
      cy.ompAssertDeniedAndUnchanged({
        fetch: fetchPayout(payoutIdPB),
        negativeRequest: () =>
          cy.ompRequest({
            method: "PUT",
            url: `${baseUrl}/payouts/${payoutIdPB}`,
            auth: { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
            body: { metadata: { attempted_by: "PA" } },
          }),
      });
    });

    it("TC-15049-03: payout confirm blocked cross-profile", () => {
      cy.ompAssertDeniedAndUnchanged({
        fetch: fetchPayout(payoutIdPB),
        negativeRequest: () =>
          cy.ompRequest({
            method: "POST",
            url: `${baseUrl}/payouts/${payoutIdPB}/confirm`,
            auth: { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
            body: { confirm: true },
          }),
      });
    });

    it("TC-15049-04: payout cancel blocked cross-profile", () => {
      cy.ompAssertDeniedAndUnchanged({
        fetch: fetchPayout(payoutIdPB),
        negativeRequest: () =>
          cy.ompRequest({
            method: "POST",
            url: `${baseUrl}/payouts/${payoutIdPB}/cancel`,
            auth: { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
            body: {},
          }),
      });
    });

    it("TC-15049-05: payout fulfill blocked cross-profile", () => {
      cy.ompAssertDeniedAndUnchanged({
        fetch: fetchPayout(payoutIdPB),
        negativeRequest: () =>
          cy.ompRequest({
            method: "POST",
            url: `${baseUrl}/payouts/${payoutIdPB}/fulfill`,
            auth: { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
            body: {},
          }),
      });
    });

    it("TC-15049-14a (positive): PA updates its own PO_PA", () => {
      cy.ompRequest({
        method: "PUT",
        url: `${baseUrl}/payouts/${payoutIdPA}`,
        auth: { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        body: { metadata: { attempted_by: "PA" } },
      }).then((res) => {
        expect(res.status).to.not.eq(403);
        expect(res.status).to.not.eq(404);
      });
    });
  });

  context("Inconsistent merchant_id / profile_id in request body", () => {
    it("TC-15049-06: payout update rejects inconsistent merchant_id", () => {
      cy.ompRequest({
        method: "PUT",
        url: `${baseUrl}/payouts/${payoutIdPA}`,
        auth: { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        body: { merchant_id: merchantIdM2 },
      }).then((res) => {
        expect(res.status).to.be.within(400, 499);
      });
    });

    it("TC-15049-07: payout confirm rejects inconsistent merchant_id", () => {
      cy.ompRequest({
        method: "POST",
        url: `${baseUrl}/payouts/${payoutIdPA}/confirm`,
        auth: { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        body: { confirm: true, merchant_id: merchantIdM2 },
      }).then((res) => {
        expect(res.status).to.be.within(400, 499);
      });
    });

    it("TC-15049-08: payout update rejects inconsistent profile_id", () => {
      cy.ompRequest({
        method: "PUT",
        url: `${baseUrl}/payouts/${payoutIdPA}`,
        auth: { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        body: { profile_id: profileIdPB },
      }).then((res) => {
        expect(res.status).to.be.within(400, 499);
      });
    });

    it("TC-15049-09: payout confirm rejects inconsistent profile_id", () => {
      cy.ompRequest({
        method: "POST",
        url: `${baseUrl}/payouts/${payoutIdPA}/confirm`,
        auth: { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        body: { confirm: true, profile_id: profileIdPB },
      }).then((res) => {
        expect(res.status).to.be.within(400, 499);
      });
    });
  });

  context("Edge cases", () => {
    it("TC-15049-E01: non-existent payout ID returns 404", () => {
      cy.ompRequest({
        method: "PUT",
        url: `${baseUrl}/payouts/payout_nonexistent_omp_test`,
        auth: { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        body: { metadata: {} },
      }).then((res) => {
        expect(res.status).to.eq(404);
      });
    });

    it("TC-15049-E03: malformed profile_id in payout body is rejected, not 500", () => {
      cy.ompRequest({
        method: "PUT",
        url: `${baseUrl}/payouts/${payoutIdPA}`,
        auth: { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        body: { profile_id: "!!not-a-valid-id!!" },
      }).then((res) => {
        expect(res.status).to.be.within(400, 499);
      });
    });

    it("TC-15049-E04: invalid credential on payout update is rejected", () => {
      cy.ompRequest({
        method: "PUT",
        url: `${baseUrl}/payouts/${payoutIdPA}`,
        auth: { type: "apiKey", apiKey: "not_a_real_api_key" },
        body: { metadata: {} },
      }).then((res) => {
        expect(res.status).to.eq(401);
      });
    });

    it("TC-15049-E05: revoked API key is rejected on payout update", () => {
      // Creating/revoking an API key needs the admin API key, not a
      // merchant's own api-key (see getManagementAuthHeaders in
      // commands.js) - the merchant key is only valid for routes that
      // operate as that merchant, and key management isn't one of them.
      const adminApiKey = globalState.get("adminApiKey");

      cy.request({
        method: "POST",
        url: `${baseUrl}/api_keys/${globalState.get("merchantIdM1")}`,
        headers: {
          "Content-Type": "application/json",
          Accept: "application/json",
          Authorization: `admin-api-key=${adminApiKey}`,
        },
        body: { name: "omp-scoping-throwaway-key", expiration: "2069-09-23T01:02:03.000Z" },
        failOnStatusCode: false,
      }).then((createRes) => {
        expect(createRes.status).to.eq(200);
        const throwawayKey = createRes.body.api_key;
        const keyId = createRes.body.key_id;

        cy.request({
          method: "DELETE",
          url: `${baseUrl}/api_keys/${globalState.get("merchantIdM1")}/${keyId}`,
          headers: { Authorization: `admin-api-key=${adminApiKey}` },
          failOnStatusCode: false,
        }).then((revokeRes) => {
          expect(revokeRes.status).to.eq(200);

          cy.ompRequest({
            method: "PUT",
            url: `${baseUrl}/payouts/${payoutIdPA}`,
            auth: { type: "apiKey", apiKey: throwawayKey },
            body: { metadata: {} },
          }).then((res) => {
            expect(res.status).to.eq(401);
          });
        });
      });
    });
  });
});
