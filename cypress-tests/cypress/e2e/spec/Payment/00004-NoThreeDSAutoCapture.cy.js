import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Card - NoThreeDS payment flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  it("Card-NoThreeDS payment flow test Create and confirm", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
      "PaymentIntent"
    ];

    cy.task("cli_log", "Create Payment Intent");
    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      data,
      "no_three_ds",
      "automatic",
      globalState
    );

    if (!utils.should_continue_further(data)) return;

    cy.task("cli_log", "Payment Methods Call");
    cy.paymentMethodsCallTest(globalState);

    const confirmData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["No3DSAutoCapture"];

    cy.task("cli_log", "Confirm Payment");
    cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);

    if (!utils.should_continue_further(confirmData)) return;

    cy.task("cli_log", "Retrieve Payment");
    cy.retrievePaymentCallTest({ globalState, data: confirmData });
  });

  it("Card-NoThreeDS payment flow test Create+Confirm", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
      "No3DSAutoCapture"
    ];

    cy.task("cli_log", "Create and Confirm Payment");
    cy.createConfirmPaymentTest(
      fixtures.createConfirmPaymentBody,
      data,
      "no_three_ds",
      "automatic",
      globalState
    );

    if (!utils.should_continue_further(data)) return;

    cy.task("cli_log", "Retrieve Payment");
    cy.retrievePaymentCallTest({ globalState, data });
  });

  it("Card-NoThreeDS payment with shipping cost", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
      "PaymentIntentWithShippingCost"
    ];

    cy.task("cli_log", "Create Payment Intent");
    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      data,
      "no_three_ds",
      "automatic",
      globalState
    );

    if (!utils.should_continue_further(data)) return;

    cy.task("cli_log", "Payment Methods Call");
    cy.paymentMethodsCallTest(globalState);

    const confirmData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["PaymentConfirmWithShippingCost"];

    cy.task("cli_log", "Confirm Payment");
    cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);

    if (!utils.should_continue_further(confirmData)) return;

    cy.task("cli_log", "Retrieve Payment");
    cy.retrievePaymentCallTest({ globalState, data: confirmData });
  });
});