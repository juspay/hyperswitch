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

    cy.step("Create Payment Intent", () => 
      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      )
    );

    if (!utils.should_continue_further(data)) return;

    cy.step("List Merchant Payment Methods", () =>
      cy.paymentMethodsCallTest(globalState)
    );

    const confirmData = getConnectorDetails(globalState.get("connectorId"))[
      "bank_redirect_pm"
    ]["Blik"];

    cy.step("Confirm Payment", () =>
      cy.confirmBankRedirectCallTest(
        fixtures.confirmBody,
        confirmData,
        true,
        globalState
      )
    );
  });

  it("EPS Create and Confirm flow test", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))[
      "bank_redirect_pm"
    ]["PaymentIntent"]("Eps");

    cy.step("Create Payment Intent", () =>
      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      )
    );

    if (!utils.should_continue_further(data)) return;

    cy.step("List Merchant Payment Methods", () =>
      cy.paymentMethodsCallTest(globalState)
    );

    const confirmData = getConnectorDetails(globalState.get("connectorId"))[
      "bank_redirect_pm"
    ]["Eps"];

    cy.step("Confirm Payment", () =>
      cy.confirmBankRedirectCallTest(
        fixtures.confirmBody,
        confirmData,
        true,
        globalState
      )
    );

    if (!utils.should_continue_further(confirmData)) return;

    const expected_redirection = fixtures.confirmBody["return_url"];
    const payment_method_type = globalState.get("paymentMethodType");

    cy.step("Handle Bank Redirect Redirection", () =>
    cy.handleBankRedirectRedirection(
      globalState,
      payment_method_type,
      expected_redirection
    )
    );
  });

  it("iDEAL Create and Confirm flow test", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))[
      "bank_redirect_pm"
    ]["PaymentIntent"]("Ideal");

    cy.step("Create Payment Intent", () =>
    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      data,
      "three_ds",
      "automatic",
      globalState
    )
    );

    if (!utils.should_continue_further(data)) return;

    cy.step("List Merchant Payment Methods", () =>
    cy.paymentMethodsCallTest(globalState)
    );

    const confirmData = getConnectorDetails(globalState.get("connectorId"))[
      "bank_redirect_pm"
    ]["Ideal"];

    cy.step("Confirm Payment", () =>
    cy.confirmBankRedirectCallTest(
      fixtures.confirmBody,
      confirmData,
      true,
      globalState
    )
    );

    if (!utils.should_continue_further(confirmData)) return;

    const expected_redirection = fixtures.confirmBody["return_url"];
    const payment_method_type = globalState.get("paymentMethodType");

    cy.step("Handle Bank Redirect Redirection", () =>
    cy.handleBankRedirectRedirection(
      globalState,
      payment_method_type,
      expected_redirection
    )
    );
  });

  it("Sofort Create and Confirm flow test", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))[
      "bank_redirect_pm"
    ]["PaymentIntent"]("Sofort");

    cy.step("Create Payment Intent", () =>
    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      data,
      "three_ds",
      "automatic",
      globalState
    )
    );

    if (!utils.should_continue_further(data)) return;

    cy.step("List Merchant Payment Methods", () =>
    cy.paymentMethodsCallTest(globalState)
    );

    const confirmData = getConnectorDetails(globalState.get("connectorId"))[
      "bank_redirect_pm"
    ]["Sofort"];

    cy.step("Confirm Payment", () =>
    cy.confirmBankRedirectCallTest(
      fixtures.confirmBody,
      confirmData,
      true,
      globalState
    )
    );

    if (!utils.should_continue_further(confirmData)) return;

    const expected_redirection = fixtures.confirmBody["return_url"];
    const payment_method_type = globalState.get("paymentMethodType");

    cy.step("Handle Bank Redirect Redirection", () =>
    cy.handleBankRedirectRedirection(
      globalState,
      payment_method_type,
      expected_redirection
    )
    );
  });

  it("Przelewy24 Create and Confirm flow test", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))[
      "bank_redirect_pm"
    ]["PaymentIntent"]("Przelewy24");

    cy.step("Create Payment Intent", () =>
    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      data,
      "three_ds",
      "automatic",
      globalState
    )
    );

    if (!utils.should_continue_further(data)) return;

    cy.step("List Merchant Payment Methods", () =>
    cy.paymentMethodsCallTest(globalState)
    );

    const confirmData = getConnectorDetails(globalState.get("connectorId"))[
      "bank_redirect_pm"
    ]["Przelewy24"];

    cy.step("Confirm Payment", () =>
    cy.confirmBankRedirectCallTest(
      fixtures.confirmBody,
      confirmData,
      true,
      globalState
    )
    );

    if (!utils.should_continue_further(confirmData)) return;

    const expected_redirection = fixtures.confirmBody["return_url"];
    const payment_method_type = globalState.get("paymentMethodType");

    cy.step("Handle Bank Redirect Redirection", () =>
    cy.handleBankRedirectRedirection(
      globalState,
      payment_method_type,
      expected_redirection
    )
    );
  });

  it("OpenBankingUk Create and Confirm flow test", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))[
      "bank_redirect_pm"
    ]["PaymentIntent"]("OpenBankingUk");

    cy.step("Create Payment Intent", () =>
    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      data,
      "three_ds",
      "automatic",
      globalState
    )
    );

    if (!utils.should_continue_further(data)) return;

    cy.step("List Merchant Payment Methods", () =>
    cy.paymentMethodsCallTest(globalState)
    );

    const confirmData = getConnectorDetails(globalState.get("connectorId"))[
      "bank_redirect_pm"
    ]["OpenBankingUk"];

    cy.step("Confirm Payment", () =>
    cy.confirmBankRedirectCallTest(
      fixtures.confirmBody,
      confirmData,
      true,
      globalState
    )
    );

    if (!utils.should_continue_further(confirmData)) return;

    const expected_redirection = fixtures.confirmBody["return_url"];
    const payment_method_type = globalState.get("paymentMethodType");

    cy.step("Handle Bank Redirect Redirection", () =>
    cy.handleBankRedirectRedirection(
      globalState,
      payment_method_type,
      expected_redirection
    )
    );

    cy.step("Retrieve Payment", () =>
    cy.retrievePaymentCallTest({ globalState, data: confirmData })
    );
  });

  it("OnlineBankingFpx Create and Confirm flow test", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))[
      "bank_redirect_pm"
    ]["PaymentIntent"]("OnlineBankingFpx");

    cy.step("Create Payment Intent", () =>
    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      data,
      "three_ds",
      "automatic",
      globalState
    )
    );

    if (!utils.should_continue_further(data)) return;

    cy.step("List Merchant Payment Methods", () =>
    cy.paymentMethodsCallTest(globalState)
    );

    const confirmData = getConnectorDetails(globalState.get("connectorId"))[
      "bank_redirect_pm"
    ]["OnlineBankingFpx"];

    cy.step("Confirm Payment", () =>
    cy.confirmBankRedirectCallTest(
      fixtures.confirmBody,
      confirmData,
      true,
      globalState
    )
    );

    if (!utils.should_continue_further(confirmData)) return;

    const expected_redirection = fixtures.confirmBody["return_url"];
    const payment_method_type = globalState.get("paymentMethodType");

    cy.step("Handle Bank Redirect Redirection", () =>
    cy.handleBankRedirectRedirection(
      globalState,
      payment_method_type,
      expected_redirection
    )
    );

    cy.step("Retrieve Payment", () =>
    cy.retrievePaymentCallTest({ globalState, data: confirmData })
    );
  });

  it("Interac Create and Confirm flow test", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))[
      "bank_redirect_pm"
    ]["PaymentIntent"]("Interac");

    cy.step("Create Payment Intent", () =>
    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      data,
      "three_ds",
      "automatic",
      globalState
    )
    );

    if (!utils.should_continue_further(data)) return;

    cy.step("List Merchant Payment Methods", () =>
    cy.paymentMethodsCallTest(globalState)
    );

    const confirmData = getConnectorDetails(globalState.get("connectorId"))[
      "bank_redirect_pm"
    ]["Interac"];

    cy.step("Confirm Payment", () =>
    cy.confirmBankRedirectCallTest(
      fixtures.confirmBody,
      confirmData,
      true,
      globalState
    )
    );

    if (!utils.should_continue_further(confirmData)) return;

    const expected_redirection = fixtures.confirmBody["return_url"];
    const payment_method_type = globalState.get("paymentMethodType");

    cy.step("Handle Bank Redirect Redirection", () =>
    cy.handleBankRedirectRedirection(
      globalState,
      payment_method_type,
      expected_redirection
    )
    );

    cy.step("Retrieve Payment", () =>
    cy.retrievePaymentCallTest({ globalState, data: confirmData })
    );
  });
});
