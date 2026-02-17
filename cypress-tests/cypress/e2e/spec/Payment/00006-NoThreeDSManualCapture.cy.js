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

  it("Card - NoThreeDS Manual Full Capture payment flow test - Create and Confirm", () => {
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

    const captureData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["Capture"];

    cy.task("cli_log", "Capture Payment");
    cy.captureCallTest(fixtures.captureBody, captureData, globalState);

    if (!utils.should_continue_further(captureData)) return;

    cy.task("cli_log", "Retrieve Payment after Capture");
    cy.retrievePaymentCallTest({ globalState, data: captureData });
  });

  it("Card - NoThreeDS Manual Full Capture payment flow test - Create+Confirm", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
      "No3DSManualCapture"
    ];

    cy.task("cli_log", "Create and Confirm Payment Intent");
    cy.createConfirmPaymentTest(
      fixtures.createConfirmPaymentBody,
      data,
      "no_three_ds",
      "manual",
      globalState
    );

    if (!utils.should_continue_further(data)) return;

    cy.task("cli_log", "Retrieve Payment after Confirmation");
    cy.retrievePaymentCallTest({ globalState, data });

    const captureData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["Capture"];

    cy.task("cli_log", "Capture Payment");
    cy.captureCallTest(fixtures.captureBody, captureData, globalState);

    if (!utils.should_continue_further(captureData)) return;

    cy.task("cli_log", "Retrieve Payment after Capture");
    cy.retrievePaymentCallTest({ globalState, data: captureData });
  });

  it("Card - NoThreeDS Manual Partial Capture payment flow test - Create and Confirm", () => {
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

    const partialCaptureData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["PartialCapture"];

    cy.task("cli_log", "Partial Capture Payment");
    cy.captureCallTest(fixtures.captureBody, partialCaptureData, globalState);

    if (!utils.should_continue_further(partialCaptureData)) return;

    cy.task("cli_log", "Retrieve Payment after Partial Capture");
    cy.retrievePaymentCallTest({ globalState, data: partialCaptureData });
  });

  it("Card - NoThreeDS Manual Partial Capture payment flow test - Create+Confirm", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
      "No3DSManualCapture"
    ];
    cy.task("cli_log", "Create and Confirm Payment Intent");
    cy.createConfirmPaymentTest(
      fixtures.createConfirmPaymentBody,
      data,
      "no_three_ds",
      "manual",
      globalState
    );

    if (!utils.should_continue_further(data)) return;

    cy.task("cli_log", "Retrieve Payment after Confirmation");
    cy.retrievePaymentCallTest({ globalState, data });

    const partialCaptureData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["PartialCapture"];

    cy.task("cli_log", "Partial Capture Payment");
    cy.captureCallTest(fixtures.captureBody, partialCaptureData, globalState);

    if (!utils.should_continue_further(partialCaptureData)) return;

    cy.task("cli_log", "Retrieve Payment after Partial Capture");
    cy.retrievePaymentCallTest({ globalState, data: partialCaptureData });
  });
});
