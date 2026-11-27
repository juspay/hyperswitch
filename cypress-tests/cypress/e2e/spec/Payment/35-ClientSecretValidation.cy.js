import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import { connectorDetails } from "../../configs/Payment/Commons";

let globalState;

// The expiring payment intent lives 60 seconds.
const SESSION_EXPIRY_WAIT = 65000;

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
      // 15 minutes, longer than the 60s profile expiry left by earlier specs.
      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        connectorDetails.eligibility_api.PaymentIntentForBlocklist,
        "no_three_ds",
        "automatic",
        globalState
      );
    });

    it("Eligibility with a wrong client_secret is rejected (IR_09)", () => {
      cy.clientSecretValidationCallTest(
        "eligibility",
        fixtures.eligibilityCheckBody,
        connectorDetails.eligibility_api.EligibilityWrongClientSecret,
        globalState
      );
    });

    it("Eligibility with the right client_secret is accepted", () => {
      cy.clientSecretValidationCallTest(
        "eligibility",
        fixtures.eligibilityCheckBody,
        connectorDetails.eligibility_api.NonBlocklistedCardAllowed,
        globalState
      );
    });

    // An API key call sends no client_secret, so there is nothing to validate.
    it("Eligibility with the API key and no client_secret is accepted", () => {
      cy.clientSecretValidationCallTest(
        "eligibility",
        fixtures.eligibilityCheckBody,
        connectorDetails.eligibility_api.NonBlocklistedCardAllowed,
        globalState,
        { useApiKey: true }
      );
    });

    it("3DS authentication with a wrong client_secret is rejected (IR_09)", () => {
      cy.clientSecretValidationCallTest(
        "3ds/authentication",
        fixtures.threeDSAuthenticationBody,
        connectorDetails.eligibility_api.ThreeDSWrongClientSecret,
        globalState
      );
    });

    it("3DS authentication with the right client_secret passes the check (IR_16 later)", () => {
      // The secret is accepted and the call fails later with IR_16, since the
      // payment has no external 3DS authentication attempt.
      cy.clientSecretValidationCallTest(
        "3ds/authentication",
        fixtures.threeDSAuthenticationBody,
        connectorDetails.step_up_auth.ThreeDSAuthenticationUnconfirmed,
        globalState
      );
    });
  });

  context("Payment whose client_secret has expired", () => {
    it("Create Payment Intent that expires in 60 seconds", () => {
      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        connectorDetails.card_pm.PaymentIntentWithSessionExpiry,
        "no_three_ds",
        "automatic",
        globalState
      );
    });

    it("Eligibility with the expired client_secret is rejected (IR_08)", () => {
      cy.wait(SESSION_EXPIRY_WAIT);
      cy.clientSecretValidationCallTest(
        "eligibility",
        fixtures.eligibilityCheckBody,
        connectorDetails.eligibility_api.EligibilityExpiredClientSecret,
        globalState
      );
    });

    it("3DS authentication with the expired client_secret is rejected (IR_08)", () => {
      cy.clientSecretValidationCallTest(
        "3ds/authentication",
        fixtures.threeDSAuthenticationBody,
        connectorDetails.eligibility_api.ThreeDSExpiredClientSecret,
        globalState
      );
    });
  });
});
