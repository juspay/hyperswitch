import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import { connectorDetails } from "../../configs/Payment/Commons";

let globalState;

// The expiring payment intent lives 60 seconds.
const SESSION_EXPIRY_WAIT = 65000;

const createPaymentIntent = (flowKey) => {
  cy.createPaymentIntentTest(
    fixtures.createPaymentBody,
    connectorDetails.eligibility_api[flowKey],
    "no_three_ds",
    "automatic",
    globalState
  );
};

const callEligibility = (flowKey, options) => {
  cy.clientSecretValidationCallTest(
    "eligibility",
    fixtures.eligibilityCheckBody,
    connectorDetails.eligibility_api[flowKey],
    globalState,
    options
  );
};

const callThreeDsAuthentication = (flowKey) => {
  cy.clientSecretValidationCallTest(
    "3ds/authentication",
    fixtures.threeDSAuthenticationBody,
    connectorDetails.eligibility_api[flowKey],
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
      createPaymentIntent("ClientSecretPaymentIntent");
    });

    it("Eligibility with a wrong client_secret is rejected (IR_09)", () => {
      callEligibility("EligibilityWrongClientSecret");
    });

    it("Eligibility with the right client_secret is accepted", () => {
      callEligibility("EligibilityValidClientSecret");
    });

    it("Eligibility with the API key and no client_secret is accepted", () => {
      callEligibility("EligibilityWithoutClientSecret", { useApiKey: true });
    });

    it("3DS authentication with a wrong client_secret is rejected (IR_09)", () => {
      callThreeDsAuthentication("ThreeDSWrongClientSecret");
    });

    it("3DS authentication with the right client_secret passes the check (IR_16 later)", () => {
      callThreeDsAuthentication("ThreeDSValidClientSecret");
    });
  });

  context("Payment whose client_secret has expired", () => {
    it("Create Payment Intent that expires in 60 seconds", () => {
      createPaymentIntent("ClientSecretExpiringPaymentIntent");
    });

    it("Eligibility with the expired client_secret is rejected (IR_08)", () => {
      cy.wait(SESSION_EXPIRY_WAIT);
      callEligibility("EligibilityExpiredClientSecret");
    });

    it("3DS authentication with the expired client_secret is rejected (IR_08)", () => {
      callThreeDsAuthentication("ThreeDSExpiredClientSecret");
    });
  });
});
