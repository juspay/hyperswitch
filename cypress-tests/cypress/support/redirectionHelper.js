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

  cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);

  if (!utils.should_continue_further(confirmData)) {
    shouldContinue = false;
  }

  if (includeRedirection && shouldContinue) {
    const expected_redirection = fixtures.confirmBody["return_url"];
    cy.handleRedirection(globalState, expected_redirection);
  }

  globalState.set("_setup3DSContinue", shouldContinue);
});
