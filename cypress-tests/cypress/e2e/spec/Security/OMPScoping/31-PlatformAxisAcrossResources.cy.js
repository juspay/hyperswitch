import State from "../../../../utils/State";

// The platform/connected-merchant axis (X-Connected-Merchant-Id,
// processor_merchant_id/platform_merchant_id columns, Sheet 2) was only
// ever tested against Payments (26-PlatformConnectedMerchant.cy.js). This
// file extends the SAME platform+connected fixture pair (built once in
// 00-Setup.cy.js) to the other resources that share the same column -
// Payouts, Dispute, Blocklist, Authentication.
//
// Resources this axis is NOT tested against here, and why:
// - Relay: GET /relay/{id} needs a real relay record belonging to the
//   connected merchant - setting one up needs a confirmed/captured
//   payment on that merchant first, which is more setup than this batch
//   covers. Follow-up, not silently assumed safe.
// - Webhook delivery attempts: JWTAuthMerchantFromRoute ONLY (confirmed in
//   30-WebhookEvents.cy.js's investigation) - no api-key/HeaderAuth path
//   exists on this route at all, and resolve_platform's connected-merchant
//   resolution is wired through the api-key/HeaderAuth auth layer, not
//   JWT. The platform axis may not even apply to this resource's routes.
// - Payment Link: customer-facing, unauthenticated render routes - no
//   credential of any kind is used, so there's no "platform key" to test.
let globalState;
let platformApiKey;
let connectedMerchantId;

describe("OMP Scoping - Platform axis across resources", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      platformApiKey = globalState.get("platformApiKey");
      connectedMerchantId = globalState.get("connectedMerchantId");
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

  it("platform merchant CANNOT use the connected-merchant header on payouts list - not supported by this route", () => {
    // CONFIRMED BY SOURCE (2026-11-19): GET /payouts/list is registered
    // with HeaderAuth(ApiKeyAuth { allow_connected_scope_operation: false,
    // ... }) (payouts.rs:413-416) - unlike Payments/Dispute/Blocklist, this
    // route doesn't support the platform/connected axis AT ALL, by design.
    // The 400 IR_49 "API does not support connected account operation" is
    // the CORRECT behavior here, not a gap - this test was originally
    // written expecting 200, which was simply wrong about this route's
    // auth config.
    cy.ompListPayouts(
      {
        type: "apiKey",
        apiKey: platformApiKey,
        connectedMerchantId: connectedMerchantId,
      },
      globalState
    ).then((res) => {
      expect(res.status).to.be.within(400, 499);
    });
  });

  it("platform merchant can list disputes acting as its own connected merchant (positive)", () => {
    cy.ompListDisputes(
      {
        type: "apiKey",
        apiKey: platformApiKey,
        connectedMerchantId: connectedMerchantId,
      },
      globalState
    ).then((res) => {
      expect(
        res.status,
        "platform acting as its own connected merchant, disputes"
      ).to.eq(200);
    });
  });

  it("platform merchant can list blocklist entries acting as its own connected merchant (positive)", () => {
    // data_kind is a REQUIRED query param (api_models/blocklist.rs:82-84,
    // no #[serde(default)]) - omitting it entirely (as an earlier version
    // of this test did) fails deserialization with 400 before the
    // platform/connected mechanism is even reached, which looked like a
    // mechanism failure but was actually just a missing param.
    // allow_connected_scope_operation: true IS confirmed on this route
    // (blocklist.rs:172-173), so 200 is the correct expectation once the
    // query is valid.
    cy.ompGetBlocklist(
      {
        type: "apiKey",
        apiKey: platformApiKey,
        connectedMerchantId: connectedMerchantId,
      },
      "?data_kind=payment_method",
      globalState
    ).then((res) => {
      expect(
        res.status,
        "platform acting as its own connected merchant, blocklist"
      ).to.eq(200);
    });
  });

  it("platform merchant can add a blocklist entry acting as its own connected merchant (positive)", () => {
    // allow_connected_scope_operation: true confirmed (blocklist.rs:53-55).
    // GenericCardBin tagged enum: {type, data} + transaction_type.
    cy.ompCreateBlocklistEntry(
      {
        type: "apiKey",
        apiKey: platformApiKey,
        connectedMerchantId: connectedMerchantId,
      },
      {
        type: "generic_card_bin",
        data: "411111",
        transaction_type: "payment",
      },
      globalState
    ).then((res) => {
      expect(
        res.status,
        "platform acting as its own connected merchant, blocklist create"
      ).to.eq(200);
    });
  });

  it("platform merchant can remove a blocklist entry acting as its own connected merchant (positive)", () => {
    // allow_connected_scope_operation: true confirmed (blocklist.rs:112-114).
    cy.ompDeleteBlocklistEntry(
      {
        type: "apiKey",
        apiKey: platformApiKey,
        connectedMerchantId: connectedMerchantId,
      },
      {
        type: "generic_card_bin",
        data: "411111",
        transaction_type: "payment",
      },
      globalState
    ).then((res) => {
      // 200 if the entry from the create test above still exists, 404 if
      // it's already gone - either is a legitimate "reached real deletion
      // logic" outcome; a 401/403 would indicate the connected-merchant
      // header itself was rejected, which is the only thing this
      // assertion needs to rule out.
      expect(
        res.status,
        "platform acting as its own connected merchant, blocklist delete"
      ).to.not.be.oneOf([401, 403]);
    });
  });

  it("platform merchant CANNOT use the connected-merchant header on authentication create - not supported by this route", () => {
    // CONFIRMED BY SOURCE (2026-11-19): POST /authentication is registered
    // with HeaderAuth(ApiKeyAuth { allow_connected_scope_operation: false,
    // ... }) (authentication.rs:40-43) - same as Payouts list, this route
    // doesn't support the platform/connected axis at all. The 400 is
    // correct behavior, not a gap.
    cy.ompCreateAuthentication(
      {
        type: "apiKey",
        apiKey: platformApiKey,
        connectedMerchantId: connectedMerchantId,
      },
      { amount: 1000, currency: "USD" },
      globalState
    ).then((res) => {
      expect(res.status).to.be.within(400, 499);
    });
  });
});
