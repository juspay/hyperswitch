/**
 * Helper to set up a 3DS payment intent + confirm so that signature/redirect
 * verification tests (in 52-PaymentResponseHash.cy.js) can reuse the same flow.
 *
 * Separating the setup here avoids duplicating create-payment + confirm logic
 * across multiple test contexts.  The caller can choose whether the helper should
 * also perform the browser redirection step (default true) or skip it so the
 * test can inspect the `next_action.redirect_to_url` directly.
 */
import * as fixtures from "../fixtures/imports";
import getConnectorDetails, * as utils from "../e2e/configs/Payment/Utils";

Cypress.Commands.add("setup3DSPayment", (globalState, options = {}) => {
  const { includeRedirection = true } = options;
  let shouldContinue = true;

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

  const confirmData = getConnectorDetails(globalState.get("connectorId"))[
    "card_pm"
  ]["3DSAutoCapture"];

  cy.confirmHashPaymentTest(
    fixtures.confirmBody,
    confirmData,
    true,
    globalState
  );

  if (!utils.should_continue_further(confirmData)) {
    shouldContinue = false;
  }

  if (includeRedirection && shouldContinue) {
    const expected_redirection = fixtures.confirmBody["return_url"];
    cy.handleRedirection(globalState, expected_redirection);
  }

  globalState.set("_setup3DSContinue", shouldContinue);
});
