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
});
