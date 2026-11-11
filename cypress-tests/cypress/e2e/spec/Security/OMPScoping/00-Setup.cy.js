import * as fixtures from "../../../../fixtures/imports";
import State from "../../../../utils/State";
import { payment_methods_enabled } from "../../../configs/Payment/Commons";
import getConnectorDetails from "../../../configs/Payment/Utils";
import { PaymentIntent, CardPayout } from "../../../configs/Security/Commons";

// Shared fixtures for the OMP (merchant/profile) scoping test matrix.
// Downstream specs in this folder read everything this file stores in
// globalState; none of them re-create these resources.
//
// Naming follows the test sheet convention:
//   M1 / M2        merchants
//   PA / PB        business profiles, both under M1
//   KEY_M1         merchant API key for M1            -> globalState "apiKeyM1"
//   JWT_PA         dashboard-user session scoped to M1/PA -> "userInfoTokenPA"
//   MCA_PA / MCA_PB, PI_PA / PI_PB, PO_PA / PO_PB      paired resources per profile
//
// ASSUMPTION (unverified against a running server): the dashboard signup
// flow (signupUserWithMerchant) provisions its own fresh merchant for the
// new user rather than attaching to a merchant created via the admin API.
// To avoid fighting that, M1 itself IS the merchant the signup flow
// creates, and its default business profile becomes PA. PB is a second
// profile created afterward under the same M1 using the admin API key.
// If signupUserWithMerchant turns out to support joining an existing
// merchant, this can be simplified to create M1 up front instead.
let globalState;

// Completes the invited-user login chain for a temp-password invite
// response: signin -> terminate 2FA (lands on force_set_password, since a
// temp password always has last_password_modified_at=None, user.rs:
// 1227-1247) -> rotate_password (no token returned) -> signin again with
// the new password -> terminate 2FA again (lands on user_info this time).
// Stores the final usable session JWT as globalState "jwtMerchantAdminM1".
// Only ever called when invite_multiple actually returned a password - see
// the GAP comment at its call site in 00-Setup.cy.js.
function completeInvitedUserLogin(email, tempPassword, globalState) {
  // Generated per-run, not a static literal - a hardcoded password-shaped
  // string here trips generic secret scanners (confirmed: GitGuardian
  // flagged a prior literal version of this on push) even though it's a
  // throwaway value this test sets itself via rotate_password for a
  // randomly-emailed test user, never a real credential.
  const newPassword = `OmpScoping_${Date.now()}_Aa1!`;
  const tokens = {};

  cy.ompUserSignin(email, tempPassword, globalState).then((res) => {
    expect(res.status, "signin with temp password").to.eq(200);
    expect(res.body?.token_type).to.eq("totp");
    tokens.totp1 = res.body.token;
  });

  cy.then(() => cy.ompTerminate2FaRaw(tokens.totp1, globalState)).then(
    (res) => {
      expect(res.status, "terminate 2FA (first pass)").to.eq(200);
      expect(res.body?.token_type).to.eq("force_set_password");
      tokens.forceSetPw = res.body.token;
    }
  );

  cy.then(() =>
    cy.ompRotatePassword(tokens.forceSetPw, newPassword, globalState)
  ).then((res) => {
    expect(res.status, "rotate password").to.eq(200);
  });

  cy.then(() => cy.ompUserSignin(email, newPassword, globalState)).then(
    (res) => {
      expect(res.status, "signin with rotated password").to.eq(200);
      expect(res.body?.token_type).to.eq("totp");
      tokens.totp2 = res.body.token;
    }
  );

  return cy
    .then(() => cy.ompTerminate2FaRaw(tokens.totp2, globalState))
    .then((res) => {
      expect(res.status, "terminate 2FA (second pass)").to.eq(200);
      expect(res.body?.token_type).to.eq("user_info");
      globalState.set("jwtMerchantAdminM1", res.body.token);
    });
}

describe("OMP Scoping - shared fixtures", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("M1 / PA via dashboard signup", () => {
    it("signs up a dashboard user, creating M1 with default profile PA", () => {
      cy.signupUserWithMerchant("OmpScopingM1", globalState);
      cy.userLogin(globalState);
      cy.terminate2Fa(globalState);
      cy.userInfo(globalState);

      cy.then(() => {
        globalState.set("merchantIdM1", globalState.get("merchantId"));
        globalState.set("profileIdPA", globalState.get("profileId"));
        globalState.set("userInfoTokenPA", globalState.get("userInfoToken"));
      });
    });

    it("creates KEY_M1 (merchant API key for M1)", () => {
      cy.then(() => {
        globalState.set("merchantId", globalState.get("merchantIdM1"));
      });

      cy.apiKeyCreateTest(fixtures.apiKeyCreateBody, globalState);

      cy.then(() => {
        globalState.set("apiKeyM1", globalState.get("apiKey"));
      });
    });

    it("creates business profile PB under M1", () => {
      cy.then(() => {
        globalState.set("merchantId", globalState.get("merchantIdM1"));
        globalState.set("apiKey", globalState.get("apiKeyM1"));
      });

      cy.createBusinessProfileTest(
        fixtures.businessProfile.bpCreate,
        globalState,
        "pb"
      );

      cy.then(() => {
        globalState.set("profileIdPB", globalState.get("pbId"));
      });
    });
  });

  context("M2 - a wholly separate merchant", () => {
    it("creates M2 with its own default profile", () => {
      cy.merchantCreateCallTest(fixtures.merchantCreateBody, globalState, {
        merchantIdStateKey: "merchantIdM2",
        profileIdStateKey: "profileIdM2Default",
        publishableKeyStateKey: "publishableKeyM2",
      });
    });

    it("creates KEY_M2 (merchant API key for M2)", () => {
      cy.then(() => {
        globalState.set("merchantId", globalState.get("merchantIdM2"));
      });

      cy.apiKeyCreateTest(fixtures.apiKeyCreateBody, globalState);

      cy.then(() => {
        globalState.set("apiKeyM2", globalState.get("apiKey"));
      });
    });
  });

  context("Paired resources under M1 - one per profile (PA, PB)", () => {
    // MCA_PA and MCA_PB are intentionally created with the SAME connector,
    // hardcoded to "stripe" rather than whatever CYPRESS_CONNECTOR happens
    // to be set to - stripe is the most reliably stable connector in this
    // repo's creds file, and OMP scoping is about merchant/profile
    // ownership, not connector behavior, so there's no reason to let an
    // unrelated env var make this suite flaky. Restored to the original
    // CYPRESS_CONNECTOR afterward in case any later step in this file
    // depends on it. Payouts use "adyenplatform" and Connector Onboarding
    // uses "paypal" for the same reliability reason, each is its own
    // dedicated MCA, not a change to this one.
    it("creates MCA_PA and MCA_PB", () => {
      const originalConnectorId = globalState.get("connectorId");

      cy.then(() => {
        globalState.set("merchantId", globalState.get("merchantIdM1"));
        globalState.set("apiKey", globalState.get("apiKeyM1"));
        globalState.set("connectorId", "stripe");
        // createConnectorCallTest resolves the profile via
        // globalState.get(`${profilePrefix}Id`) - "pa"/"pb" prefixes read
        // "paId"/"pbId", which only PB's creation (via
        // createBusinessProfileTest) populated. PA's profile id lives under
        // "profileIdPA" (set by the signup flow), so mirror it into "paId".
        globalState.set("paId", globalState.get("profileIdPA"));
      });

      cy.createConnectorCallTest(
        "payment_processor",
        fixtures.createConnectorBody,
        payment_methods_enabled,
        globalState,
        "pa",
        "mcaPa"
      );

      cy.createConnectorCallTest(
        "payment_processor",
        fixtures.createConnectorBody,
        payment_methods_enabled,
        globalState,
        "pb",
        "mcaPb"
      );

      cy.then(() => {
        globalState.set("connectorId", originalConnectorId);
      });
    });

    it("creates MCA_PAYPAL_PA and MCA_PAYPAL_PB, so connector onboarding sync has real PayPal MCAs to test against", () => {
      // POST /connector_onboarding/sync returns 400 IR_20 "Connector
      // onboarding flow not supported" for any non-PayPal connector,
      // confirmed live - the feature is PayPal-only. MCA_PA (whatever
      // CYPRESS_CONNECTOR is) can't be used for 06-ConnectorOnboarding.cy.js's
      // checks, so this creates PayPal-specific MCAs under BOTH profiles -
      // same connectorId-override pattern as the Adyenplatform payout
      // connector step above.
      //
      // Both profiles, not just PA: the actual ownership check compares
      // the target MCA's real profile against the authenticated profile,
      // not an arbitrary profile_id field in the request body - confirmed
      // live when a single PA-owned MCA plus a spoofed body.profile_id
      // returned 200 instead of denial (PA's own MCA + PA's own auth is
      // never a real mismatch, no matter what the body claims). A genuine
      // cross-profile test needs an MCA that actually belongs to PB.
      const paymentsConnectorId = globalState.get("connectorId");

      cy.then(() => {
        globalState.set("connectorId", "paypal");
      });

      cy.createConnectorCallTest(
        "payment_processor",
        structuredClone(fixtures.createConnectorBody),
        payment_methods_enabled,
        globalState,
        "pa",
        "mcaPaypalPa"
      );

      cy.createConnectorCallTest(
        "payment_processor",
        structuredClone(fixtures.createConnectorBody),
        payment_methods_enabled,
        globalState,
        "pb",
        "mcaPaypalPb"
      );

      cy.then(() => {
        globalState.set("connectorId", paymentsConnectorId);
      });
    });

    it("creates a netcetera authentication_processor connector for PA and configures it on PA's business profile", () => {
      // Standalone /authentication eligibility and /authentication/sync
      // (04-Authentication.cy.js) need PA's business profile to actually
      // have authentication_connector_details set
      // (crates/router/src/core/authentication/utils.rs:603-610) - without
      // it, eligibility can't proceed past the profile check into real
      // 3DS processing. Same setup 47-StepUpAuth.cy.js already does for its
      // own profile: create a netcetera MCA with connector_type
      // "authentication_processor" under PA, then update PA's business
      // profile with authentication_connector_details pointing at it.
      cy.then(() => {
        globalState.set("merchantId", globalState.get("merchantIdM1"));
        globalState.set("apiKey", globalState.get("apiKeyM1"));
      });

      const createConnectorBody = {
        ...structuredClone(fixtures.createConnectorBody),
        connector_name: "netcetera",
      };

      cy.createAuthenticationProcessorConnectorTest(
        createConnectorBody,
        globalState,
        "pa",
        "authProcessorConnectorPa"
      );

      cy.then(() => {
        if (!globalState.get("authProcessorConnectorPaId")) {
          cy.task(
            "cli_log",
            "Skipping: netcetera authentication_processor connector creation did not succeed."
          );
          return;
        }

        cy.UpdateBusinessProfileTest(
          {
            authentication_connector_details: {
              authentication_connectors: ["netcetera"],
              three_ds_requestor_url: "https://example.com",
            },
          },
          false,
          false,
          false,
          false,
          false,
          globalState,
          "pa"
        );
      });
    });

    it("creates PI_PA and PI_PB (payment intents)", () => {
      // Request/Response come from configs/Security/Commons.js, the same
      // bundling convention configs/Payment/Commons.js uses for its own
      // PaymentIntent getter.
      //
      // These intents run against whichever single connector
      // CYPRESS_CONNECTOR is set to for this run - the suite doesn't loop
      // over multiple connectors, because OMP (profile/merchant) scoping is
      // enforced in the router layer before any connector is ever reached,
      // so the connector choice is orthogonal to what this suite verifies.
      cy.then(() => {
        globalState.set("merchantId", globalState.get("merchantIdM1"));
        globalState.set("apiKey", globalState.get("apiKeyM1"));
        globalState.set("profileId", globalState.get("profileIdPA"));
      });

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        { Configs: {}, ...PaymentIntent() },
        "no_three_ds",
        "automatic",
        globalState
      );

      cy.then(() => {
        globalState.set("paymentIdPA", globalState.get("paymentID"));
        globalState.set("profileId", globalState.get("profileIdPB"));
      });

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        { Configs: {}, ...PaymentIntent() },
        "no_three_ds",
        "automatic",
        globalState
      );

      cy.then(() => {
        globalState.set("paymentIdPB", globalState.get("paymentID"));
      });
    });

    it("creates a payout_processor connector for PA and PB", () => {
      // createConfirmPayoutTest's generic payment_processor MCA can't route
      // payouts - payouts specifically need a "payout_processor" MCA, whose
      // credentials come from the `${connectorId}_payout` entry in the
      // connector creds file (see createPayoutConnectorCallTest). Adyenplatform
      // has no plain (non-payout) creds entry in this repo's creds.json, so
      // it can only ever be a payout connector, not a payment one - hence
      // the connectorId is temporarily overridden to "adyenplatform" for
      // just this step and restored afterward, independent of whatever
      // connector the rest of the suite runs payments against.
      const paymentsConnectorId = globalState.get("connectorId");

      cy.then(() => {
        globalState.set("merchantId", globalState.get("merchantIdM1"));
        globalState.set("apiKey", globalState.get("apiKeyM1"));
        globalState.set("profileId", globalState.get("profileIdPA"));
        globalState.set("connectorId", "adyenplatform");
      });

      cy.createPayoutConnectorCallTest(
        "payout_processor",
        structuredClone(fixtures.createConnectorBody),
        globalState
      );

      cy.then(() => {
        globalState.set("profileId", globalState.get("profileIdPB"));
      });

      cy.createPayoutConnectorCallTest(
        "payout_processor",
        structuredClone(fixtures.createConnectorBody),
        globalState
      );

      cy.then(() => {
        globalState.set("connectorId", paymentsConnectorId);
      });
    });

    it("creates PO_PA and PO_PB (payouts), if the connector supports payouts", () => {
      if (!globalState.get("payoutsExecution")) {
        cy.task(
          "cli_log",
          "Skipping PO_PA/PO_PB: configured connector has no payout " +
            "credentials in the creds file. 02-Payouts.cy.js must gate on " +
            "the same payoutsExecution flag."
        );
        return;
      }

      // createConfirmPayoutTest reads customer_id from
      // globalState.get("customerId") - must exist before a payout can be
      // created, and payouts need a customer regardless of profile.
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);

      cy.createConfirmPayoutTest(
        structuredClone(fixtures.createPayoutBody),
        {
          ...CardPayout(),
          Request: {
            ...CardPayout().Request,
            profile_id: globalState.get("profileIdPA"),
          },
        },
        true,
        false,
        globalState
      );

      cy.then(() => {
        globalState.set("payoutIdPA", globalState.get("payoutID"));
      });

      cy.createConfirmPayoutTest(
        structuredClone(fixtures.createPayoutBody),
        {
          ...CardPayout(),
          Request: {
            ...CardPayout().Request,
            profile_id: globalState.get("profileIdPB"),
          },
        },
        true,
        false,
        globalState
      );

      cy.then(() => {
        globalState.set("payoutIdPB", globalState.get("payoutID"));
      });
    });

    // GAP (see feasibility check): this suite has no command that fabricates
    // a dispute. The existing 47-DisputeTests.cy.js only reads whatever
    // disputes already exist for the merchant/connector and skips its own
    // assertions when none are found. D_PA / D_PB can only be populated by
    // listing real disputes against a connector sandbox that actually has
    // dispute data - there is nothing to create here. Downstream specs
    // (05-Disputes.cy.js) must follow the same "skip if none found" gate
    // this repo already uses rather than assume D_PA/D_PB exist.
    it("looks up any pre-existing disputes for D_PA / D_PB (best effort)", () => {
      cy.task(
        "cli_log",
        "No dispute-fabrication command exists in this suite; 05-Disputes.cy.js " +
          "will list disputes per profile at runtime and skip cases if none exist."
      );
    });

    it("creates CUSTOMER_M1 under M1", () => {
      cy.then(() => {
        globalState.set("merchantId", globalState.get("merchantIdM1"));
        globalState.set("apiKey", globalState.get("apiKeyM1"));
      });

      // createCustomerCallTest sets globalState "customerId" - independent
      // of the payout block's own customer (that one is gated on
      // payoutsExecution and may not run), so this gets its own customer
      // regardless of payout connector availability.
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);

      cy.then(() => {
        globalState.set("customerIdM1", globalState.get("customerId"));
      });
    });

    it("creates PAYMENT_METHOD_M1 attached to CUSTOMER_M1", () => {
      cy.then(() => {
        globalState.set("customerId", globalState.get("customerIdM1"));
      });

      // Reuses the shared card_pm "PaymentMethod" fixture
      // (configs/Payment/Commons.js, same one 24-PaymentMethods.cy.js
      // uses) rather than hand-rolling card details, per review feedback.
      cy.createPaymentMethodTest(
        globalState,
        getConnectorDetails("commons")["card_pm"]["PaymentMethod"]
      );

      cy.then(() => {
        globalState.set(
          "paymentMethodIdM1",
          globalState.get("paymentMethodId")
        );
      });
    });

    it("creates a captured CIT payment under PA (also yields MANDATE_PA)", () => {
      // Reuses the existing citForMandatesCallTest infrastructure
      // (cypress/support/commands.js, same command the Mandate spec uses)
      // rather than hand-building confirm/capture logic - this is the only
      // way to get an actually-captured payment (for Refund) and a real
      // mandate_id (for Mandate) without re-deriving per-connector card
      // behavior from scratch. Needs globalState "merchantConnectorId"
      // (the default key citForMandatesCallTest's profile/connector
      // resolution reads), distinct from our own "mcaPaId" naming.
      // Depends on the configured connector having a
      // MandateSingleUseNo3DSAutoCapture fixture in configs/Payment/ - if
      // it doesn't, this step (and anything downstream that needs
      // capturedPaymentIdPA/mandateIdPA) needs live verification.
      cy.then(() => {
        globalState.set("merchantId", globalState.get("merchantIdM1"));
        globalState.set("apiKey", globalState.get("apiKeyM1"));
        globalState.set("profileId", globalState.get("profileIdPA"));
        globalState.set("merchantConnectorId", globalState.get("mcaPaId"));
      });

      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["MandateSingleUseNo3DSAutoCapture"];

      cy.citForMandatesCallTest(
        fixtures.citConfirmBody,
        data,
        true,
        "automatic",
        "new_mandate",
        globalState
      );

      cy.then(() => {
        globalState.set("capturedPaymentIdPA", globalState.get("paymentID"));
        globalState.set("mandateIdPA", globalState.get("mandateId"));
      });
    });

    it("creates REFUND_PA against the captured CIT payment", () => {
      // refundCallTest reads payment_id from globalState "paymentID", still
      // set to capturedPaymentIdPA from the step above.
      const refundData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["Refund"];

      cy.refundCallTest(fixtures.refundBody, refundData, globalState);

      cy.then(() => {
        globalState.set("refundIdPA", globalState.get("refundId"));
      });
    });

    it("creates ROUTING_CONFIG_PA under PA and activates it", () => {
      cy.then(() => {
        globalState.set("profileId", globalState.get("profileIdPA"));
      });

      const routing_data = [
        {
          connector: globalState.get("connectorId"),
          merchant_connector_id: globalState.get("mcaPaId"),
        },
      ];

      cy.addRoutingConfig(
        fixtures.routingConfigBody,
        { Request: {}, Response: { status: 200, body: {} } },
        "priority",
        routing_data,
        globalState
      );

      cy.activateRoutingConfig(
        { Request: {}, Response: { status: 200, body: {} } },
        globalState
      );

      cy.then(() => {
        globalState.set(
          "routingConfigIdPA",
          globalState.get("routingConfigId")
        );
      });
    });

    it("upserts SURCHARGE_CONFIG_M1 for M1", () => {
      // Reuses the existing createSurchargeDSLConfig command (Routing/00005-
      // SurchargeDSLConfiguration.cy.js) rather than a bespoke request -
      // it already knows this route needs JWT auth (reads
      // globalState.get("userInfoToken") directly, falling back to api-key
      // only if that's unset) and needs a full algorithm, not just
      // merchant_surcharge_configs (confirmed live: omitting it returns
      // IR_04 "Missing required param: algorithm").
      const surchargeBody = {
        name: "omp_scoping_surcharge_config",
        merchant_surcharge_configs: {},
        algorithm: {
          defaultSelection: {
            surcharge_details: {
              surcharge: { type: "rate", value: { percentage: 2.5 } },
            },
          },
          rules: [],
          metadata: {},
        },
      };

      cy.createSurchargeDSLConfig(
        surchargeBody,
        { Response: { body: {} } },
        globalState
      );
    });

    // DROPPED: a NULL profile_id can't actually be produced through the
    // public API - connector creation always resolves a profile at
    // creation time, either explicitly via profile_id or via a legacy
    // business_country/business_label fallback (confirmed: omitting
    // profile_id returns IR_04 "Missing required param: profile_id or
    // business_country, business_label"). The nullable-profile_id
    // fail-closed behavior (validate_profile_id_from_auth_layer) was
    // already confirmed by reading its source; there's no reachable
    // precondition to test it against here.

    it("creates PAYMENT_LINK_PA (a payment with payment_link enabled)", () => {
      cy.then(() => {
        globalState.set("merchantId", globalState.get("merchantIdM1"));
        globalState.set("apiKey", globalState.get("apiKeyM1"));
        globalState.set("profileId", globalState.get("profileIdPA"));
        globalState.set("customerId", globalState.get("customerIdM1"));
      });

      cy.createPaymentIntentWithPaymentLinkTest(
        fixtures.createPaymentBody,
        { Request: { currency: "USD" }, Response: { status: 200 } },
        "no_three_ds",
        "automatic",
        globalState
      );

      cy.then(() => {
        globalState.set("paymentLinkPaymentId", globalState.get("paymentID"));
      });
    });

    it("creates PAYOUT_LINK_PA", () => {
      // This repo's Cypress config restricts Payout Link to "wise"
      // (CONNECTOR_LISTS.INCLUDE.PAYOUT_LINK, configs/Payout/Utils.js), but
      // that's not an actual backend restriction - core/payout_link.rs
      // builds its connector list dynamically from whichever
      // payout_processor MCAs are enabled and support the requested payout
      // method, not a fixed allowlist. config/development.toml's
      // [payout_method_filters.adyenplatform] explicitly supports
      // credit/debit card payouts for EUR/USD/GBP/CAD/JPY across many
      // countries - exactly the card/EUR combination this suite's own
      // CardPayout() fixture already uses for PO_PA/PO_PB. adyenplatform is
      // hardcoded as this suite's payout connector regardless of
      // CYPRESS_CONNECTOR (see the payout_processor MCA step above), so
      // just attempt it directly rather than gating on a wise-only check
      // that - by the time this step runs - was comparing against the
      // restored Payments connectorId anyway, not the payout one, making
      // the old gate unconditionally true (PAYOUT_LINK_PA never created,
      // regardless of CYPRESS_CONNECTOR). If adyenplatform genuinely
      // doesn't support it, let this fail honestly rather than silently
      // skip - that's real, actionable information.
      //
      // Must use fixtures.createPayoutLinkBody, not the generic
      // fixtures.createPayoutBody - the dedicated fixture sets
      // payout_link: true and confirm: false (the payout method is
      // deferred to the customer filling in the hosted link, not supplied
      // upfront). Using the generic body (confirmed live) omits
      // payout_link entirely, so the server tries normal eligible-connector
      // routing with no payout_type/payout_method_data at all and fails
      // with IR_39 "No eligible connector was found" - a test-fixture bug,
      // not a real adyenplatform capability gap.
      //
      // Also needs payout_link_config set on PA's business profile first -
      // confirmed live in two steps: (1) 400 IR_40 "Payout links cannot be
      // used without setting allowed_domains in profile... set test_mode
      // to true..." without it, then (2) a plain deserialize error for a
      // field named "test_mode" - the actual request struct
      // (BusinessPayoutLinkConfig, api_models/src/admin.rs:3584-3595, via
      // its flattened BusinessGenericLinkConfig at :3635-3645) names the
      // boolean field payout_test_mode, not test_mode (the error message's
      // prose doesn't match the real field name), and allowed_domains
      // (HashSet<String>, no default) is required regardless of
      // payout_test_mode - both together, matching this repo's own
      // Payout/00008-PayoutLinkCreate.cy.js bpWithPayoutLink fixture.
      globalState.set("payoutLinkExecution", true);
      cy.then(() => {
        globalState.set("profileId", globalState.get("profileIdPA"));
      });

      cy.UpdateBusinessProfileTest(
        {
          payout_link_config: {
            payout_test_mode: true,
            allowed_domains: ["*.hyperswitch.io"],
          },
        },
        false,
        false,
        false,
        false,
        false,
        globalState,
        "pa"
      );

      cy.createPayoutWithLinkTest(
        structuredClone(fixtures.createPayoutLinkBody),
        { Request: { currency: "EUR" }, Response: { status: 200 } },
        globalState
      );

      cy.then(() => {
        globalState.set("payoutLinkPayoutId", globalState.get("payoutID"));
      });
    });
  });

  context("PC - dedicated profile for incremental/extend authorization", () => {
    // incremental_authorization and extend_authorization both gate on
    // payment status (requires_capture, plus incremental_authorization_allowed
    // for the former) in their own get_trackers, BEFORE the generic
    // cross-profile check ever runs - a payment stuck in
    // requires_payment_method (like PI_PA/PI_PB, never confirmed) can't
    // reach that check at all, confirmed live (01-Payments.cy.js's two
    // tests on PI_PB returned 400/412 from the status gate itself, not a
    // real profile-mismatch denial). Reaching the real check needs a
    // payment genuinely confirmed into requires_capture with incremental
    // auth enabled, which in this repo's test matrix is cybersource-only
    // (see 29-IncrementalAuth.cy.js). A dedicated profile with ONLY
    // cybersource enabled avoids routing ambiguity with PA/PB's existing
    // stripe/paypal connectors.
    it("creates profile PC and a dedicated cybersource MCA", () => {
      const originalConnectorId = globalState.get("connectorId");

      cy.then(() => {
        globalState.set("merchantId", globalState.get("merchantIdM1"));
        globalState.set("apiKey", globalState.get("apiKeyM1"));
      });

      cy.createBusinessProfileTest(
        fixtures.businessProfile.bpCreate,
        globalState,
        "pc"
      );

      cy.then(() => {
        globalState.set("profileIdPC", globalState.get("pcId"));
        globalState.set("connectorId", "cybersource");
        globalState.set("profileId", globalState.get("profileIdPC"));
      });

      cy.createConnectorCallTest(
        "payment_processor",
        structuredClone(fixtures.createConnectorBody),
        payment_methods_enabled,
        globalState
      );

      cy.then(() => {
        globalState.set("connectorId", originalConnectorId);
      });
    });

    it("creates PI_INCREMENTAL_PC - a cybersource payment confirmed into requires_capture with incremental auth enabled", () => {
      cy.then(() => {
        return cy.ompGetMerchantAccount(
          globalState.get("merchantIdM1"),
          globalState
        );
      }).then((res) => {
        globalState.set("publishableKeyM1", res.body?.publishable_key);
      });

      cy.then(() => {
        globalState.set("profileId", globalState.get("profileIdPC"));
        globalState.set("apiKey", globalState.get("apiKeyM1"));
        globalState.set("publishableKey", globalState.get("publishableKeyM1"));
      });

      const cybersourceCardPm = getConnectorDetails("cybersource")["card_pm"];

      const intentData = {
        ...cybersourceCardPm["PaymentIntentOffSession"],
        Request: {
          ...cybersourceCardPm["PaymentIntentOffSession"].Request,
          request_incremental_authorization: true,
        },
      };

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        intentData,
        "no_three_ds",
        "manual",
        globalState
      );

      cy.confirmCallTest(
        fixtures.confirmBody,
        cybersourceCardPm["SaveCardUseNo3DSManualCaptureOffSession"],
        true,
        globalState
      );

      // Not cy.incrementalAuth: that command strictly asserts the
      // connector-side amount increase actually lands (amount_capturable
      // equals the requested bump) - cybersource's sandbox accepted this
      // call (200) but didn't apply the increase, confirmed live. That's a
      // connector-sandbox quirk unrelated to what this fixture needs: only
      // incremental_authorization_allowed=true and status=requires_capture
      // matter for the OMP cross-profile tests that consume this payment,
      // not whether the increment itself numerically succeeded.
      cy.then(() => {
        const paymentId = globalState.get("paymentID");
        return cy.ompIncrementalAuthorization(
          paymentId,
          { type: "apiKey", apiKey: globalState.get("apiKeyM1") },
          { amount: 8000 },
          globalState
        );
      }).then((res) => {
        expect(res.status, "incremental_authorization call").to.eq(200);
        expect(
          res.body?.incremental_authorization_allowed,
          "incremental_authorization_allowed"
        ).to.eq(true);
        expect(res.body?.status, "status").to.eq("requires_capture");
      });

      cy.then(() => {
        globalState.set("paymentIdIncrementalPC", globalState.get("paymentID"));
      });
    });
  });

  context(
    "M3 and a merchant_admin user, for Analytics role-scope tests",
    () => {
      // M1's own dashboard user is org_admin (signup_with_merchant_id grants
      // ROLE_ID_ORGANIZATION_ADMIN, core/user.rs:96-103) and M1 sits in its
      // own, freshly-created organization - M2 (above) is a SEPARATE org
      // entirely, so M1-vs-M2 never actually exercised "two merchants under
      // one org." M3 here is a genuine sibling of M1: created with
      // organization_id explicitly set to M1's own org, so
      // CreateOrValidateOrganization validates/reuses that org instead of
      // minting a new one (core/admin.rs:505-523, 708-719).
      it("creates M3, a second merchant in M1's own organization", () => {
        const m1OrganizationId = globalState.get("organizationId");
        const m3Body = {
          ...structuredClone(fixtures.merchantCreateBody),
          organization_id: m1OrganizationId,
        };

        cy.merchantCreateCallTest(m3Body, globalState, {
          merchantIdStateKey: "merchantIdM3",
          profileIdStateKey: "profileIdM3Default",
          publishableKeyStateKey: "publishableKeyM3",
        });

        cy.then(() => {
          // merchantCreateCallTest unconditionally overwrites globalState
          // "organizationId" with whatever the server echoes back - confirm
          // it's still M1's org (validated/reused), not a freshly minted one.
          expect(
            globalState.get("organizationId"),
            "M3 must land in M1's existing org, not a fresh one"
          ).to.eq(m1OrganizationId);
        });
      });

      // GAP: bootstrapping a second, merchant_admin-scoped user under M1
      // would need the real invite flow (POST /user/user/invite_multiple),
      // but confirmed live: this environment never returns a temp password
      // in the invite response (is_email_sent-style flow instead - a real
      // inbox is required to accept the invite), so the no-email-feature
      // shortcut this was built against (core/user.rs:1080-1249, returns
      // a password directly when the "email" cargo feature is off) doesn't
      // apply here. There is no black-box way to complete this invite
      // without real email access, so this is skipped gracefully rather
      // than chased further - same convention as the Disputes/Relay
      // "no fabrication command exists" gaps elsewhere in this file. The
      // merchant_admin-specific tests in 17-Analytics.cy.js already gate
      // on jwtMerchantAdminM1 being present and skip themselves when it's
      // not; the org_admin-vs-sibling tests there don't depend on this
      // user at all and still run for real.
      it("invites a merchant_admin user scoped to M1 (best effort - see GAP above)", () => {
        const inviteEmail = `omp_scoping_merchant_admin_${Date.now()}@example.com`;

        cy.ompInviteUser(
          { type: "jwt", token: globalState.get("userInfoTokenPA") },
          [
            {
              email: inviteEmail,
              name: "OMP Scoping Merchant Admin",
              role_id: "merchant_admin",
            },
          ],
          globalState
        ).then((res) => {
          expect(res.status, "invite_multiple call").to.eq(200);
          const invitee = Array.isArray(res.body) ? res.body[0] : res.body;

          if (!invitee?.password) {
            cy.task(
              "cli_log",
              "Skipping merchant_admin bootstrap: invite_multiple did not " +
                "return a temp password (this environment requires a real " +
                "email invite accept, which this suite can't automate). " +
                "17-Analytics.cy.js's merchant_admin tests will skip."
            );
            return;
          }

          cy.then(() =>
            completeInvitedUserLogin(inviteEmail, invitee.password, globalState)
          );
        });
      });
    }
  );
});
