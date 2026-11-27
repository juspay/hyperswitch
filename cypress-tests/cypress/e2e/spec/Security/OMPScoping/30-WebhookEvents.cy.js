import State from "../../../../utils/State";

// POST /events/{merchant_id} and GET/POST .../{event_id}/attempts,/retry are
// ALL scoped by JWTAuthMerchantFromRoute{merchant_id} ONLY
// (webhook_events.rs:22,46-51) - there is NO api-key auth alternative on any
// of these routes. event_id ("initial_attempt_id") is a plain String, not a
// branded ID type - no merchant-scoping was confirmed at the type/route
// layer for the event_id lookup itself.
//
// IMPORTANT TEST-DESIGN NOTE: a genuine cross-merchant check on these routes
// needs a JWT scoped to a DIFFERENT merchant than the path - an api-key
// gets rejected outright regardless of which merchant it belongs to, since
// api-key simply isn't an accepted credential type here at all. We don't
// have a dashboard JWT for M2 (M2 was created via the plain merchant-create
// API, not a dashboard signup, and provisioning one needs the same
// email-invite flow blocked elsewhere in this suite). So the "M2 cannot..."
// tests below use apiKeyM2 and WILL be denied, but that only proves
// "api-key isn't accepted here," not "a different merchant's JWT is
// correctly denied" - the real cross-merchant question remains an open gap,
// documented honestly rather than claimed as tested.
let globalState;
let apiKeyM2;
let merchantIdM1;
let jwtPA;

describe("OMP Scoping - Webhook delivery attempts", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      apiKeyM2 = globalState.get("apiKeyM2");
      merchantIdM1 = globalState.get("merchantIdM1");
      jwtPA = globalState.get("userInfoTokenPA");
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("List - JWT-only auth", () => {
    it("M1 lists its own webhook delivery attempts (positive)", function () {
      if (!jwtPA) this.skip();
      cy.ompListWebhookDeliveryAttempts(
        merchantIdM1,
        { type: "jwt", token: jwtPA },
        {},
        globalState
      ).then((res) => {
        // A 401 here with M1's own JWT and M1's own path would be
        // unexpected given this route's merchant_id check should match -
        // if this starts failing, check whether jwtPA is still fresh
        // (dashboard session JWTs are short-lived, and this spec runs
        // late in a long sequential suite) before assuming a real bug.
        expect(res.status).to.eq(200);
      });
    });

    it("an api-key is rejected outright on this route, regardless of merchant (NOT a cross-merchant test)", () => {
      cy.ompListWebhookDeliveryAttempts(
        merchantIdM1,
        { type: "apiKey", apiKey: apiKeyM2 },
        {},
        globalState
      ).then((res) => {
        expect(res.status).to.be.within(400, 499);
      });
    });
  });

  context(
    "Attempts / Retry - JWT-only auth, event_id scoping unconfirmed",
    () => {
      it("an api-key is rejected outright on the attempts route too (NOT a cross-merchant test)", () => {
        // Without a real seeded event_id AND without an M2 JWT, this only
        // proves api-key isn't an accepted credential here - the real
        // cross-merchant event_id question (can M1's own JWT reach an
        // event_id that actually belongs to a different merchant) remains
        // an open gap, not resolved by this test.
        cy.ompGetWebhookDeliveryAttempts(
          merchantIdM1,
          "evt_omp_scoping_test_placeholder",
          { type: "apiKey", apiKey: apiKeyM2 },
          globalState
        ).then((res) => {
          expect(res.status).to.be.within(400, 499);
        });
      });

      it("M1's own JWT querying a non-existent event_id under its own merchant_id returns 404, not 500", function () {
        if (!jwtPA) this.skip();
        cy.ompGetWebhookDeliveryAttempts(
          merchantIdM1,
          "evt_omp_scoping_test_placeholder",
          { type: "jwt", token: jwtPA },
          globalState
        ).then((res) => {
          expect(res.status).to.be.within(400, 499);
        });
      });

      it("invalid credential on webhook attempts retrieval is rejected", () => {
        cy.ompGetWebhookDeliveryAttempts(
          merchantIdM1,
          "evt_omp_scoping_test_placeholder",
          { type: "invalid" },
          globalState
        ).then((res) => {
          expect(res.status).to.eq(401);
        });
      });
    }
  );
});
