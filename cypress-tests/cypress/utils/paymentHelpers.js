import getConnectorDetails, * as utils from "../e2e/configs/Payment/Utils";

/**
 * Helper function to setup a 3DS payment flow
 * @param {Object} gs - globalState object for managing test state
 * @param {Object} options - Configuration options
 * @param {boolean} options.includeRedirection - Whether to handle redirect flow (default: true)
 * @param {Object} options.fixtures - Test fixtures object with createPaymentBody and confirmBody
 */
export function setup3DSPayment(gs, options = {}) {
  const { includeRedirection = true, fixtures } = options;
  let shouldContinue = true;

  const data = getConnectorDetails(gs.get("connectorId"))["card_pm"][
    "PaymentIntent"
  ];

  cy.createPaymentIntentTest(
    fixtures.createPaymentBody,
    data,
    "three_ds",
    "automatic",
    gs
  );

  if (!utils.should_continue_further(data)) {
    shouldContinue = false;
  }

  if (!shouldContinue) {
    cy.task("cli_log", "setup3DSPayment: stopping after createPaymentIntent");
    gs.set("_setup3DSContinue", false);
    return;
  }

  cy.paymentMethodsCallTest(gs);

  const confirmData = getConnectorDetails(gs.get("connectorId"))["card_pm"][
    "3DSAutoCapture"
  ];

  cy.confirmCallForHashTest(fixtures.confirmBody, confirmData, true, gs);

  if (!utils.should_continue_further(confirmData)) {
    shouldContinue = false;
  }

  if (shouldContinue) {
    cy.captureRedirectReturnUrl(gs);
  }

  if (includeRedirection && shouldContinue) {
    const expected_redirection = fixtures.confirmBody["return_url"];
    cy.handleRedirection(gs, expected_redirection);
  }

  gs.set("_setup3DSContinue", shouldContinue);
}
