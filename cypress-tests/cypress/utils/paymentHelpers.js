import getConnectorDetails, * as utils from "../e2e/configs/Payment/Utils";

/**
 * Helper function to setup a 3DS payment flow
 * @param {Object} globalState - globalState object for managing test state and persisting data across test steps
 * @param {Object} options - Configuration options for customizing 3DS payment setup behavior
 * @param {boolean} options.includeRedirection - Controls whether to handle browser redirection after 3DS authentication (default: true). Set to false when testing signature/HMAC verification without full redirect flow.
 * @param {Object} options.fixtures - Test fixtures object containing createPaymentBody and confirmBody for payment setup
 */
export function setup3DSPayment(globalState, options = {}) {
  // Destructure options with defaults: includeRedirection controls redirect handling, fixtures provides test data
  const { includeRedirection = true, fixtures } = options;
  let shouldContinue = true;

  // Fetch PaymentIntent configuration for card payment method from the connector's config
  // This determines the request/response expectations based on the configured connector
  const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
    "PaymentIntent"
  ];

  cy.createPaymentIntentTest(
    fixtures.createPaymentBody,
    data,
    "three_ds",
    "automatic",
    globalState
  );

  if (!utils.should_continue_further(data)) {
    shouldContinue = false;
  }

  if (!shouldContinue) {
    cy.task("cli_log", "setup3DSPayment: stopping after createPaymentIntent");
    globalState.set("_setup3DSContinue", false);
    return;
  }

  cy.paymentMethodsCallTest(globalState);

  // Fetch 3DS Auto-Capture configuration for card payment method from the connector's config
  const confirmData = getConnectorDetails(globalState.get("connectorId"))[
    "card_pm"
  ]["3DSAutoCapture"];

  cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);

  if (!utils.should_continue_further(confirmData)) {
    shouldContinue = false;
  }

  if (shouldContinue) {
    cy.captureRedirectReturnUrl(globalState);
  }

  if (includeRedirection && shouldContinue) {
    const expected_redirection = fixtures.confirmBody["return_url"];
    cy.handleRedirection(globalState, expected_redirection);
  }

  globalState.set("_setup3DSContinue", shouldContinue);
}
