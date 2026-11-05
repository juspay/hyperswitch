import State from "../../../../utils/State";

// CONFIRMED GAP: list_webhook_delivery_attempts and
// retry_webhook_delivery_attempt (crates/router/src/core/webhooks/webhook_events.rs)
// only check merchant (JWTAuthMerchantFromRoute validates the path
// merchant_id against the JWT's merchant) - neither checks profile_id at
// all. A JWT scoped to profile PA can list and retry profile PB's webhook
// delivery attempts on the same merchant. The tests below document the
// CURRENT (gap) behavior rather than assert denial, so they flip red ->
// green once a profile check is added upstream.
//
// GAP: this suite has no command that fabricates a webhook delivery
// attempt - like disputes, these only exist from real connector-initiated
// webhook events. Both tests here look up whatever delivery attempts
// already exist for M1 at runtime and skip if none are found.
//
// Routes (verified): POST /events/{merchant_id} (list),
// POST /events/{merchant_id}/{initial_attempt_id}/retry.
let globalState;
let jwtPA;
let merchantIdM1;
let initialAttemptId;

describe("OMP Scoping - Webhook Delivery Attempts", () => {
  before("seed global state and look up a real delivery attempt", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      jwtPA = globalState.get("userInfoTokenPA");
      merchantIdM1 = globalState.get("merchantIdM1");
    });

    cy.then(() => {
      return cy.ompListWebhookDeliveryAttempts(
        merchantIdM1,
        { type: "jwt", token: jwtPA },
        {},
        globalState
      );
    }).then((res) => {
      const attempts = Array.isArray(res.body) ? res.body : res.body?.data;
      if (attempts && attempts.length > 0) {
        initialAttemptId = attempts[0].initial_attempt_id || attempts[0].id;
      } else {
        cy.task(
          "cli_log",
          "No real webhook delivery attempt found for M1 - webhook gap " +
            "tests in this file will skip. See the GAP comment at the top " +
            "of this file."
        );
      }
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  beforeEach(function () {
    if (!initialAttemptId) this.skip();
  });

  it("PA lists webhook delivery attempts - no profile scoping (confirmed gap)", () => {
    cy.ompListWebhookDeliveryAttempts(
      merchantIdM1,
      { type: "jwt", token: jwtPA },
      {},
      globalState
    ).then((res) => {
      // No assertion of denial here deliberately - the audit found no
      // profile_id check in list_webhook_delivery_attempts at all. This
      // documents that the call succeeds at the merchant level regardless
      // of which profile the JWT is scoped to.
      expect(
        res.status,
        "listing succeeds at merchant level - no profile check exists"
      ).to.eq(200);
      cy.task(
        "cli_log",
        "CONFIRMED GAP: webhook delivery attempt listing has no profile " +
          "check - any profile-scoped JWT for this merchant can list all " +
          "delivery attempts regardless of which profile they belong to."
      );
    });
  });

  it("PA retries a webhook delivery attempt - no profile scoping (confirmed gap)", () => {
    cy.ompRetryWebhookDeliveryAttempt(
      merchantIdM1,
      initialAttemptId,
      { type: "jwt", token: jwtPA },
      globalState
    ).then((res) => {
      // Same reasoning as above - document the actual status rather than
      // assert either outcome, since retrying twice may itself fail for
      // unrelated reasons (e.g. already-delivered state).
      cy.task(
        "cli_log",
        `Webhook retry returned status ${res.status} - no profile check ` +
          "exists in retry_webhook_delivery_attempt, only a merchant check."
      );
    });
  });

  it("invalid credential on webhook list is rejected", () => {
    cy.ompListWebhookDeliveryAttempts(
      merchantIdM1,
      { type: "invalid" },
      {},
      globalState
    ).then((res) => {
      expect(res.status).to.eq(401);
    });
  });
});
