import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Bank Redirect tests", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  it("Blik Create and Confirm flow test", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))[
      "bank_redirect_pm"
    ]["PaymentIntent"]("Blik");

    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      data,
      "three_ds",
      "automatic",
      globalState
    );

    if (!utils.should_continue_further(data)) return;

    cy.paymentMethodsCallTest(globalState);

    const confirmData = getConnectorDetails(globalState.get("connectorId"))[
      "bank_redirect_pm"
    ]["Blik"];

    cy.confirmBankRedirectCallTest(
      fixtures.confirmBody,
      confirmData,
      true,
      globalState
    );
  });

  it("EPS Create and Confirm flow test", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))[
      "bank_redirect_pm"
    ]["PaymentIntent"]("Eps");

    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      data,
      "three_ds",
      "automatic",
      globalState
    );

    if (!utils.should_continue_further(data)) return;

    cy.paymentMethodsCallTest(globalState);

    const confirmData = getConnectorDetails(globalState.get("connectorId"))[
      "bank_redirect_pm"
    ]["Eps"];

    cy.confirmBankRedirectCallTest(
      fixtures.confirmBody,
      confirmData,
      true,
      globalState
    );

    if (!utils.should_continue_further(confirmData)) return;

    const expected_redirection = fixtures.confirmBody["return_url"];
    const payment_method_type = globalState.get("paymentMethodType");

    cy.handleBankRedirectRedirection(
      globalState,
      payment_method_type,
      expected_redirection
    );
  });

  it("iDEAL Create and Confirm flow test", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))[
      "bank_redirect_pm"
    ]["PaymentIntent"]("Ideal");

    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      data,
      "three_ds",
      "automatic",
      globalState
    );

    if (!utils.should_continue_further(data)) return;

    cy.paymentMethodsCallTest(globalState);

    const confirmData = getConnectorDetails(globalState.get("connectorId"))[
      "bank_redirect_pm"
    ]["Ideal"];

    cy.confirmBankRedirectCallTest(
      fixtures.confirmBody,
      confirmData,
      true,
      globalState
    );

    if (!utils.should_continue_further(confirmData)) return;

    const expected_redirection = fixtures.confirmBody["return_url"];
    const payment_method_type = globalState.get("paymentMethodType");

    cy.handleBankRedirectRedirection(
      globalState,
      payment_method_type,
      expected_redirection
    );
  });

  it("Sofort Create and Confirm flow test", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))[
      "bank_redirect_pm"
    ]["PaymentIntent"]("Sofort");

    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      data,
      "three_ds",
      "automatic",
      globalState
    );

    if (!utils.should_continue_further(data)) return;

    cy.paymentMethodsCallTest(globalState);

    const confirmData = getConnectorDetails(globalState.get("connectorId"))[
      "bank_redirect_pm"
    ]["Sofort"];

    cy.confirmBankRedirectCallTest(
      fixtures.confirmBody,
      confirmData,
      true,
      globalState
    );

    if (!utils.should_continue_further(confirmData)) return;

    const expected_redirection = fixtures.confirmBody["return_url"];
    const payment_method_type = globalState.get("paymentMethodType");

    cy.handleBankRedirectRedirection(
      globalState,
      payment_method_type,
      expected_redirection
    );
  });

  it("Przelewy24 Create and Confirm flow test", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))[
      "bank_redirect_pm"
    ]["PaymentIntent"]("Przelewy24");

    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      data,
      "three_ds",
      "automatic",
      globalState
    );

    if (!utils.should_continue_further(data)) return;

    cy.paymentMethodsCallTest(globalState);

    const confirmData = getConnectorDetails(globalState.get("connectorId"))[
      "bank_redirect_pm"
    ]["Przelewy24"];

    cy.confirmBankRedirectCallTest(
      fixtures.confirmBody,
      confirmData,
      true,
      globalState
    );

    if (!utils.should_continue_further(confirmData)) return;

    const expected_redirection = fixtures.confirmBody["return_url"];
    const payment_method_type = globalState.get("paymentMethodType");

    cy.handleBankRedirectRedirection(
      globalState,
      payment_method_type,
      expected_redirection
    );
  });

  it("OpenBankingUk Create and Confirm flow test", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))[
      "bank_redirect_pm"
    ]["PaymentIntent"]("OpenBankingUk");

    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      data,
      "three_ds",
      "automatic",
      globalState
    );

    if (!utils.should_continue_further(data)) return;

    cy.paymentMethodsCallTest(globalState);

    const confirmData = getConnectorDetails(globalState.get("connectorId"))[
      "bank_redirect_pm"
    ]["OpenBankingUk"];

    cy.confirmBankRedirectCallTest(
      fixtures.confirmBody,
      confirmData,
      true,
      globalState
    );

    if (!utils.should_continue_further(confirmData)) return;

    const expected_redirection = fixtures.confirmBody["return_url"];
    const payment_method_type = globalState.get("paymentMethodType");

    cy.handleBankRedirectRedirection(
      globalState,
      payment_method_type,
      expected_redirection
    );

    cy.retrievePaymentCallTest({ globalState, data: confirmData });
  });

  it("OnlineBankingFpx Create and Confirm flow test", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))[
      "bank_redirect_pm"
    ]["PaymentIntent"]("OnlineBankingFpx");

    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      data,
      "three_ds",
      "automatic",
      globalState
    );

    if (!utils.should_continue_further(data)) return;

    cy.paymentMethodsCallTest(globalState);

    const confirmData = getConnectorDetails(globalState.get("connectorId"))[
      "bank_redirect_pm"
    ]["OnlineBankingFpx"];

    cy.confirmBankRedirectCallTest(
      fixtures.confirmBody,
      confirmData,
      true,
      globalState
    );

    if (!utils.should_continue_further(confirmData)) return;

    const expected_redirection = fixtures.confirmBody["return_url"];
    const payment_method_type = globalState.get("paymentMethodType");

    cy.handleBankRedirectRedirection(
      globalState,
      payment_method_type,
      expected_redirection
    );

    cy.retrievePaymentCallTest({ globalState, data: confirmData });
  });

  it("Interac Create and Confirm flow test", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))[
      "bank_redirect_pm"
    ]["PaymentIntent"]("Interac");

    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      data,
      "three_ds",
      "automatic",
      globalState
    );

    if (!utils.should_continue_further(data)) return;

    cy.paymentMethodsCallTest(globalState);

    const confirmData = getConnectorDetails(globalState.get("connectorId"))[
      "bank_redirect_pm"
    ]["Interac"];

    cy.confirmBankRedirectCallTest(
      fixtures.confirmBody,
      confirmData,
      true,
      globalState
    );

    if (!utils.should_continue_further(confirmData)) return;

    const expected_redirection = fixtures.confirmBody["return_url"];
    const payment_method_type = globalState.get("paymentMethodType");

    cy.handleBankRedirectRedirection(
      globalState,
      payment_method_type,
      expected_redirection
    );

    cy.retrievePaymentCallTest({ globalState, data: confirmData });
  });
});
