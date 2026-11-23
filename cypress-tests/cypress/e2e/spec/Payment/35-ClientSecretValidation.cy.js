import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import { connectorDetails } from "../../configs/Payment/Commons";

let globalState;

// The expiring payment intent lives 60 seconds.
const SESSION_EXPIRY_WAIT = 65000;

// Existing configs, reused: a 200 eligibility check for a card, the payment
// intent configs, the IR_08 response of an expired session, and the 3DS
// authentication that fails later with IR_16.
const eligibility = connectorDetails.eligibility_api.NonBlocklistedCardAllowed;
const paymentIntent = connectorDetails.card_pm.PaymentIntent;
const expiringPaymentIntent =
  connectorDetails.card_pm.PaymentIntentWithSessionExpiry;
const clientSecretExpired =
  connectorDetails.card_pm.SessionExpiredConfirmPayment.Response;
const threeDsAuthentication =
  connectorDetails.step_up_auth.ThreeDSAuthenticationUnconfirmed;

// 15 minutes, longer than the 60s profile expiry left by earlier specs.
const longLivedPaymentIntent = {
  ...paymentIntent,
  Request: { ...paymentIntent.Request, session_expiry: 900 },
};

const wrongClientSecret = { client_secret: "pay_wrong_client_secret_value" };

const clientSecretInvalid = {
  status: 400,
  body: {
    error: {
      type: "invalid_request",
      code: "IR_09",
      message:
        "The client_secret provided does not match the client_secret associated with the Payment",
    },
  },
};

const createPaymentIntent = (data) => {
  cy.createPaymentIntentTest(
    fixtures.createPaymentBody,
    data,
    "no_three_ds",
    "automatic",
    globalState
  );
};

const callEligibility = (data, options) => {
  cy.clientSecretValidationCallTest(
    "eligibility",
    fixtures.eligibilityCheckBody,
    data,
    globalState,
    options
  );
};

const callThreeDsAuthentication = (data) => {
  cy.clientSecretValidationCallTest(
    "3ds/authentication",
    fixtures.threeDSAuthenticationBody,
    data,
    globalState
  );
};

describe("client_secret validation on eligibility and 3DS authentication", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Payment with a valid client_secret", () => {
    it("Create Payment Intent", () => {
      createPaymentIntent(longLivedPaymentIntent);
    });

    it("Eligibility with a wrong client_secret is rejected (IR_09)", () => {
      callEligibility({
        Request: { ...eligibility.Request, ...wrongClientSecret },
        Response: clientSecretInvalid,
      });
    });

    it("Eligibility with the right client_secret is accepted", () => {
      callEligibility(eligibility);
    });

    // An API key call sends no client_secret, so there is nothing to validate.
    it("Eligibility with the API key and no client_secret is accepted", () => {
      callEligibility(eligibility, { useApiKey: true });
    });

    it("3DS authentication with a wrong client_secret is rejected (IR_09)", () => {
      callThreeDsAuthentication({
        Request: wrongClientSecret,
        Response: clientSecretInvalid,
      });
    });

    it("3DS authentication with the right client_secret passes the check (IR_16 later)", () => {
      // The secret is accepted and the call fails later with IR_16, since the
      // payment has no external 3DS authentication attempt.
      callThreeDsAuthentication(threeDsAuthentication);
    });
  });

  context("Payment whose client_secret has expired", () => {
    it("Create Payment Intent that expires in 60 seconds", () => {
      createPaymentIntent(expiringPaymentIntent);
    });

    it("Eligibility with the expired client_secret is rejected (IR_08)", () => {
      cy.wait(SESSION_EXPIRY_WAIT);
      callEligibility({
        Request: eligibility.Request,
        Response: clientSecretExpired,
      });
    });

    it("3DS authentication with the expired client_secret is rejected (IR_08)", () => {
      callThreeDsAuthentication({
        Request: {},
        Response: clientSecretExpired,
      });
    });
  });
});
