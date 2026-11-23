import * as fixtures from "../fixtures/imports";
import getConnectorDetails, {
  CONNECTOR_LISTS,
  shouldExcludeConnector,
  should_continue_further,
} from "../e2e/configs/Payment/Utils";

// Shared steps of the external surcharge specs (InterPayments as the
// surcharge_processor connector).

export const cardData = (globalState, flowKey) =>
  getConnectorDetails(globalState.get("connectorId"))["card_pm"][flowKey];

// The InterPayments surcharge connector is created once, with the other
// connectors, by 03-ConnectorCreate, unless the connector is on the exclude list
// and only when creds.json has an InterPayments entry.
export const createSurchargeConnectorIfConfigured = (globalState) => {
  if (
    shouldExcludeConnector(
      globalState.get("connectorId"),
      CONNECTOR_LISTS.EXCLUDE.EXTERNAL_SURCHARGE
    )
  ) {
    cy.task(
      "cli_log",
      "External surcharge is excluded for this connector, not creating the surcharge connector"
    );
    return;
  }
  cy.connectorCredentialsAvailable("interpayments", globalState).then(
    (available) => {
      if (!available) {
        cy.task(
          "cli_log",
          "No InterPayments credentials in creds.json, not creating the surcharge connector"
        );
        return;
      }
      cy.createNamedConnectorCallTest(
        "surcharge_processor",
        structuredClone(fixtures.createConnectorBody),
        [],
        globalState,
        "interpayments",
        "interpayments_surcharge",
        "profile",
        "surchargeConnector"
      );
    }
  );
};

// Yields true when the external surcharge specs cannot run here: 03-ConnectorCreate
// did not create the surcharge connector.
export const shouldSkipExternalSurcharge = (globalState) => {
  const skip = !globalState.get("surchargeConnectorId");
  if (skip) {
    cy.task(
      "cli_log",
      "No surcharge connector was created, skipping the external surcharge spec"
    );
  }
  return cy.wrap(skip);
};

// Yields true when the surcharge connector can calculate a surcharge in this
// environment. Links the profile, creates a payment and asks for eligibility; a
// connector error (the router cannot reach the surcharge service) yields false.
export const isSurchargeServiceAvailable = (globalState) => {
  cy.linkSurchargeConnectorToProfile(globalState);
  cy.createPaymentIntentTest(
    fixtures.createPaymentBody,
    cardData(globalState, "ExternalSurchargePaymentIntent"),
    "no_three_ds",
    "automatic",
    globalState
  );
  cy.paymentMethodsCallTest(globalState);
  return cy.surchargeServiceAvailabilityCheck(
    fixtures.eligibilityCheckBody,
    cardData(globalState, "ExternalSurchargeEligibility"),
    globalState
  );
};

// A customer-initiated card payment with an external surcharge: create the
// payment intent, eligibility check (which calculates the surcharge), confirm
// (the response must carry the surcharge). Returns an object whose
// shouldContinue turns false once a step fails, so the caller can skip its own
// steps too.
export const runSurchargePayment = (
  globalState,
  { captureMethod = "automatic" } = {}
) => {
  const state = { shouldContinue: true };

  cy.step("Create Payment Intent", () => {
    const data = cardData(globalState, "ExternalSurchargePaymentIntent");
    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      data,
      "no_three_ds",
      captureMethod,
      globalState
    );
    if (!should_continue_further(data)) {
      state.shouldContinue = false;
    }
  });

  cy.step("Payment Methods Call", () => {
    if (!state.shouldContinue) {
      cy.task("cli_log", "Skipping step: Payment Methods Call");
      return;
    }
    cy.paymentMethodsCallTest(globalState);
  });

  cy.step("Eligibility Check", () => {
    if (!state.shouldContinue) {
      cy.task("cli_log", "Skipping step: Eligibility Check");
      return;
    }
    cy.paymentsEligibilityCheck(
      fixtures.eligibilityCheckBody,
      cardData(globalState, "ExternalSurchargeEligibility"),
      globalState
    );
  });

  cy.step("Confirm Payment (the response carries the surcharge)", () => {
    if (!state.shouldContinue) {
      cy.task("cli_log", "Skipping step: Confirm Payment");
      return;
    }
    const data = cardData(
      globalState,
      captureMethod === "manual"
        ? "ExternalSurchargeManualConfirm"
        : "ExternalSurchargeConfirm"
    );
    cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
    if (!should_continue_further(data)) {
      state.shouldContinue = false;
    }
  });

  return state;
};

// Captures the whole charged total (order amount + surcharge).
export const captureFullAmount = (globalState) => {
  const data = cardData(globalState, "ExternalSurchargeManualCapture");
  // The status after a capture depends on the connector (Stripe: succeeded,
  // Cybersource: processing), so take it from the connector's own Capture config.
  const captureStatus = cardData(globalState, "Capture")?.Response?.body
    ?.status;
  cy.captureCallTest(
    Cypress._.cloneDeep(fixtures.captureBody),
    captureStatus
      ? { ...data, Response: { status: 200, body: { status: captureStatus } } }
      : data,
    globalState
  );
};
