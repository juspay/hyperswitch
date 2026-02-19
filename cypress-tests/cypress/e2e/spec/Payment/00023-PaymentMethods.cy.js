import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Payment Methods Tests", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  it("Create payment method for customer", () => {
    cy.task("cli_log", "Create Customer");
    cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);

    const data = getConnectorDetails("commons")["card_pm"]["PaymentMethod"];

    cy.task("cli_log", "Create Payment Method");
    cy.createPaymentMethodTest(globalState, data);

    cy.task("cli_log", "List PM for Customer");
    cy.listCustomerPMCallTest(globalState);
  });

  it("Set default payment method", () => {
    cy.task("cli_log", "List PM for Customer");
    cy.listCustomerPMCallTest(globalState);

    const pmData = getConnectorDetails("commons")["card_pm"]["PaymentMethod"];

    cy.task("cli_log", "Create Payment Method");
    cy.createPaymentMethodTest(globalState, pmData);

    const intentData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["PaymentIntentOffSession"];

    cy.task("cli_log", "Create Payment Intent");
    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      intentData,
      "no_three_ds",
      "automatic",
      globalState
    );

    if (!utils.should_continue_further(intentData)) return;

    const confirmData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["SaveCardUseNo3DSAutoCaptureOffSession"];

    cy.task("cli_log", "Confirm Payment");
    cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);

    if (!utils.should_continue_further(confirmData)) return;

    cy.task("cli_log", "List PM for Customer");
    cy.listCustomerPMCallTest(globalState);

    cy.task("cli_log", "Set Default Payment Method");
    cy.setDefaultPaymentMethodTest(globalState);
  });

  it("Delete payment method for customer", () => {
    cy.task("cli_log", "Create Customer");
    cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);

    const data = getConnectorDetails("commons")["card_pm"]["PaymentMethod"];

    cy.task("cli_log", "Create Payment Method");
    cy.createPaymentMethodTest(globalState, data);

    cy.task("cli_log", "List PM for Customer");
    cy.listCustomerPMCallTest(globalState);

    cy.task("cli_log", "Delete Payment Method for Customer");
    cy.deletePaymentMethodTest(globalState);
  });

  it("'Last Used' off-session token payments - No 3DS save card", () => {
    cy.task("cli_log", "Create Customer");
    cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);

    const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
      "SaveCardUseNo3DSAutoCaptureOffSession"
    ];

    cy.task("cli_log", "Create+Confirm Payment (No 3DS Off Session)");
    cy.createConfirmPaymentTest(
      fixtures.createConfirmPaymentBody,
      data,
      "no_three_ds",
      "automatic",
      globalState
    );

    if (!utils.should_continue_further(data)) return;

    cy.task("cli_log", "List PM for Customer");
    cy.listCustomerPMCallTest(globalState);
  });

  it("'Last Used' off-session token payments - 3DS save card", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
      "SaveCardUse3DSAutoCaptureOffSession"
    ];

    cy.task("cli_log", "Create+Confirm Payment (3DS Off Session)");
    cy.createConfirmPaymentTest(
      fixtures.createConfirmPaymentBody,
      data,
      "three_ds",
      "automatic",
      globalState
    );

    if (!utils.should_continue_further(data)) return;

    cy.task("cli_log", "Handle Redirection");
    const expectedRedirection = fixtures.confirmBody["return_url"];
    cy.handleRedirection(globalState, expectedRedirection);

    cy.task("cli_log", "List PM for Customer");
    cy.listCustomerPMCallTest(globalState);
  });

  it("'Last Used' off-session token payments - 3DS save card with token", () => {
    const saveCardBody = Cypress._.cloneDeep(fixtures.saveCardConfirmBody);

    const intentData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["PaymentIntent"];

    cy.task("cli_log", "Create Payment Intent");
    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      intentData,
      "three_ds",
      "automatic",
      globalState
    );

    if (!utils.should_continue_further(intentData)) return;

    const confirmData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["SaveCardUseNo3DSAutoCapture"];

    const newData = {
      ...confirmData,
      Response: {
        ...confirmData.Response,
        body: {
          ...confirmData.Response.body,
          status: "requires_customer_action",
        },
      },
    };

    cy.task("cli_log", "Confirm Save Card Payment");
    cy.saveCardConfirmCallTest(saveCardBody, newData, globalState);

    if (!utils.should_continue_further(confirmData)) return;

    cy.task("cli_log", "Handle Redirection");
    const expectedRedirection = fixtures.confirmBody["return_url"];
    cy.handleRedirection(globalState, expectedRedirection);

    cy.task("cli_log", "List PM for Customer");
    cy.listCustomerPMCallTest(globalState, 1);
  });

  it("'Last Used' off-session token payments - No 3DS save card with token", () => {
    const saveCardBody = Cypress._.cloneDeep(fixtures.saveCardConfirmBody);

    const intentData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["PaymentIntent"];

    cy.task("cli_log", "Create Payment Intent");
    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      intentData,
      "no_three_ds",
      "automatic",
      globalState
    );

    if (!utils.should_continue_further(intentData)) return;

    const confirmData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["SaveCardUseNo3DSAutoCapture"];

    cy.task("cli_log", "Confirm Save Card Payment");
    cy.saveCardConfirmCallTest(saveCardBody, confirmData, globalState);

    if (!utils.should_continue_further(confirmData)) return;

    cy.task("cli_log", "List PM for Customer");
    cy.listCustomerPMCallTest(globalState);
  });
});
