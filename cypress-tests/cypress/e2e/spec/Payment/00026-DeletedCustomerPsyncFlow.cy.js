import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Card - Customer Deletion and Psync", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  it("No3DS Card - Psync after Customer Deletion (Automatic Capture)", () => {
    cy.task("cli_log", "Create Customer");
    cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);

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

    const confirmData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["No3DSAutoCapture"];

    cy.task("cli_log", "Confirm Payment");
    cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);

    if (!utils.should_continue_further(confirmData)) return;

    cy.task("cli_log", "Retrieve Payment");
    cy.retrievePaymentCallTest({ globalState, data: confirmData });

    cy.task("cli_log", "Delete Customer");
    cy.customerDeleteCall(globalState);

    cy.task("cli_log", "Retrieve Payment (After Customer Deletion)");
    cy.retrievePaymentCallTest({ globalState, data: confirmData });
  });

  it("3DS Card - Psync after Customer Deletion (Automatic Capture)", () => {
    cy.task("cli_log", "Create Customer");
    cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);

    const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
      "PaymentIntent"
    ];

    cy.task("cli_log", "Create Payment Intent");
    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      data,
      "three_ds",
      "automatic",
      globalState
    );

    if (!utils.should_continue_further(data)) return;

    const confirmData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["3DSAutoCapture"];

    cy.task("cli_log", "Confirm Payment");
    cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);

    if (!utils.should_continue_further(confirmData)) return;

    cy.task("cli_log", "Handle Redirection");
    const expected_redirection = fixtures.confirmBody["return_url"];
    cy.handleRedirection(globalState, expected_redirection);

    const retrieveData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["No3DSAutoCapture"];

    cy.task("cli_log", "Retrieve Payment");
    cy.retrievePaymentCallTest({ globalState, data: retrieveData });

    cy.task("cli_log", "Delete Customer");
    cy.customerDeleteCall(globalState);

    cy.task("cli_log", "Retrieve Payment (After Customer Deletion)");
    cy.retrievePaymentCallTest({ globalState, data: retrieveData });
  });

  it("No3DS Card - Psync after Customer Deletion (Manual Capture)", () => {
    cy.task("cli_log", "Create Customer");
    cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);

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

    const confirmData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["No3DSManualCapture"];

    cy.task("cli_log", "Confirm Payment");
    cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);

    if (!utils.should_continue_further(confirmData)) return;

    cy.task("cli_log", "Retrieve Payment");
    cy.retrievePaymentCallTest({ globalState, data: confirmData });

    const captureData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["Capture"];

    cy.task("cli_log", "Capture Payment");
    cy.captureCallTest(fixtures.captureBody, captureData, globalState);

    if (!utils.should_continue_further(captureData)) return;

    cy.task("cli_log", "Retrieve Payment (After Capture)");
    cy.retrievePaymentCallTest({ globalState, data: captureData });

    cy.task("cli_log", "Delete Customer");
    cy.customerDeleteCall(globalState);

    cy.task("cli_log", "Retrieve Payment (After Customer Deletion)");
    cy.retrievePaymentCallTest({ globalState, data: confirmData });
  });

  it("3DS Card - Psync after Customer Deletion (Manual Capture)", () => {
    cy.task("cli_log", "Create Customer");
    cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);

    const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
      "PaymentIntent"
    ];

    cy.task("cli_log", "Create Payment Intent");
    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      data,
      "three_ds",
      "manual",
      globalState
    );

    if (!utils.should_continue_further(data)) return;

    const confirmData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["3DSManualCapture"];

    cy.task("cli_log", "Confirm Payment");
    cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);

    if (!utils.should_continue_further(confirmData)) return;

    cy.task("cli_log", "Handle Redirection");
    const expected_redirection = fixtures.confirmBody["return_url"];
    cy.handleRedirection(globalState, expected_redirection);

    cy.task("cli_log", "Retrieve Payment");
    cy.retrievePaymentCallTest({ globalState, data: confirmData });

    const captureData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["Capture"];

    cy.task("cli_log", "Capture Payment");
    cy.captureCallTest(fixtures.captureBody, captureData, globalState);

    if (!utils.should_continue_further(captureData)) return;

    cy.task("cli_log", "Retrieve Payment (After Capture)");
    cy.retrievePaymentCallTest({ globalState, data: captureData });

    cy.task("cli_log", "Delete Customer");
    cy.customerDeleteCall(globalState);

    cy.task("cli_log", "Retrieve Payment (After Customer Deletion)");
    cy.retrievePaymentCallTest({ globalState, data: confirmData });
  });
});
