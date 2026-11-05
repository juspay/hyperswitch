// Shared test data for cypress/e2e/spec/Security/OMPScoping/, following the
// same getCustomExchange(Request/Response) convention as
// configs/Payment/Commons.js and configs/Payout/Commons.js - each uses its
// own domain's getCustomExchange (Payment vs Payout) since each merges
// overrides onto that domain's own default exchange (e.g. Payment's
// defaults to a 501 not-implemented Response; see Modifiers.js in each
// folder), not a generic shared one.
import { getCustomExchange as getPaymentCustomExchange } from "../Payment/Modifiers";
import { getCustomExchange as getPayoutCustomExchange } from "../Payout/Modifiers";

// PI_PA / PI_PB creation in 00-Setup.cy.js. Matches the shape
// configs/Payment/Commons.js's own PaymentIntent getter uses.
export const PaymentIntent = () =>
  getPaymentCustomExchange({
    Request: { currency: "USD" },
    Response: {
      status: 200,
      body: { status: "requires_payment_method" },
    },
  });

// PO_PA / PO_PB creation in 00-Setup.cy.js. Matches Adyenplatform's card_pm
// "Confirm" fixture shape (configs/Payout/AdyenPlatform.js) - with
// confirm=true, auto_fulfill=false, Adyenplatform returns
// "requires_fulfillment" rather than failing outright. Adyenplatform is
// hardcoded as the payout connector in 00-Setup.cy.js regardless of
// CYPRESS_CONNECTOR, since it's the only connector in this repo's creds
// file with a payout-capable ("_payout") credentials entry but no plain
// payment one - so this Request intentionally doesn't vary by connector.
export const CardPayout = () =>
  getPayoutCustomExchange({
    Request: {
      payout_method_data: {
        card: {
          card_number: "4111111111111111",
          expiry_month: "03",
          expiry_year: "2030",
          card_holder_name: "John Smith",
        },
      },
      currency: "EUR",
      payout_type: "card",
    },
    Response: { status: 200, body: {} },
  });

// Profile acquirer create/update bodies, used in 03-ProfileAcquirer.cy.js -
// a plain object (not getCustomExchange-wrapped) since profile_acquirer has
// no connector-specific default exchange of its own to merge onto.
export function acquirerBody(profileId, overrides = {}) {
  return {
    acquirer_assigned_merchant_id: "omp_test_merchant_id",
    merchant_name: "OMP Scoping Test Merchant",
    network: "Visa",
    acquirer_bin: "123456",
    profile_id: profileId,
    ...overrides,
  };
}

// Known, live-confirmed exact error responses shared across multiple specs
// in this folder, for use as `expectedError` on cy.ompAssertDeniedAndUnchanged
// (see cypress/support/security-commands.js). Defined once here rather than
// inlined per test so the same confirmed value isn't duplicated/drifted
// across files - add an entry only once its exact status/code/message has
// actually been observed on a live run, never guessed.
//
// IMPORTANT: these are NOT one universal "JWT profile mismatch" error - the
// exact response depends on which core module's check actually runs, and
// that has been seen to change with the router version. Payments'
// cross-profile retrieve used to go through the generic auth-layer
// comparison (validate_payment_profile / validate_profile_id_from_auth_layer,
// IR_16) but PR #15080 ("fix(payments): enforce authenticated payment
// scope", commit d07de03379) moved it to a profile-scoped SQL query that
// returns a plain not-found (HE_02) instead - confirmed by seeing BOTH
// responses live for the identical test, traced to version skew between
// router pods during a rolling deploy, not request-level non-determinism.
// Keep each resource's entry separate; don't assume one endpoint's exact
// error still applies to another after a fix like this lands.
export const OMP_ERRORS = {
  // Business Profile's cross-profile update check (JWTAuthMerchantAndProfileFromRoute
  // comparing the JWT's own profile against the path profile_id) - this
  // core module wasn't touched by PR #15080, so IR_16 is still expected
  // here. Confirmed live.
  JWT_PROFILE_MISMATCH: {
    status: 400,
    code: "IR_16",
    message:
      "Profile id authentication failed. Please use the correct JWT token",
  },
  // Payments' cross-profile retrieve (GET /payments/{id}?force_sync=true
  // with a JWT): post-#15080, the profile-scoped DB query simply doesn't
  // match the row, so this is PaymentNotFound, not an auth-layer mismatch.
  // Confirmed live on current main. If this starts failing again with
  // IR_16 400 instead, that's the test environment serving a stale,
  // pre-#15080 router build on some pods, not a test or server-logic bug -
  // check for a rolling-deploy/version-skew issue before changing this back.
  PAYMENT_PROFILE_SCOPED_NOT_FOUND: {
    status: 404,
    code: "HE_02",
    message: "Payment does not exist in our records",
  },
};
