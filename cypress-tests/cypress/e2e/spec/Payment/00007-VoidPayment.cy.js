import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Card - NoThreeDS Manual payment flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  it("Card - void payment in Requires_capture state flow test", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
      "PaymentIntent"
    ];

    cy.task("cli_log", "Create Payment Intent");
    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      data,
      "no_three_ds",
      "manual",
      globalState
    );

    if (!utils.should_continue_further(data)) return;

    cy.task("cli_log", "Payment Methods Call");
    cy.paymentMethodsCallTest(globalState);

    const confirmData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["No3DSManualCapture"];

    cy.task("cli_log", "Confirm Payment Intent");
    cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);

    if (!utils.should_continue_further(confirmData)) return;

    cy.task("cli_log", "Retrieve Payment after Confirmation");
    cy.retrievePaymentCallTest({ globalState, data: confirmData });

    const voidData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["VoidAfterConfirm"];

    cy.task("cli_log", "Void Payment after Confirmation");
    cy.voidCallTest(fixtures.voidBody, voidData, globalState);
  });

  it("Card - void payment in Requires_payment_method state flow test", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
      "PaymentIntent"
    ];

    cy.task("cli_log", "Create Payment Intent");
    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      data,
      "no_three_ds",
      "manual",
      globalState
    );

    if (!utils.should_continue_further(data)) return;

    cy.task("cli_log", "Payment Methods Call");
    cy.paymentMethodsCallTest(globalState);

    const voidData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["Void"];

    cy.task("cli_log", "Void Payment without Confirmation");
    cy.voidCallTest(fixtures.voidBody, voidData, globalState);
  });

  it("Card - void payment in success state flow test", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
      "PaymentIntent"
    ];

    cy.task("cli_log", "Create Payment Intent");
    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      data,
      "no_three_ds",
      "manual",
      globalState
    );

    if (!utils.should_continue_further(data)) return;

    cy.task("cli_log", "Payment Methods Call");
    cy.paymentMethodsCallTest(globalState);

    const confirmData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["No3DSManualCapture"];

    cy.task("cli_log", "Confirm Payment Intent");
    cy.confirmCallTest(fixtures.confirmBody, confirmData, false, globalState);

    if (!utils.should_continue_further(confirmData)) return;

    cy.task("cli_log", "Retrieve Payment after Confirmation");
    cy.retrievePaymentCallTest({ globalState, data: confirmData });

    const voidData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["VoidAfterConfirm"];

    cy.task("cli_log", "Void Payment after Confirmation");
    cy.voidCallTest(fixtures.voidBody, voidData, globalState);
  });
});
