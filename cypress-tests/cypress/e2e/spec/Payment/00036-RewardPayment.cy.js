import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Reward Payment - Cashtocode", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  it("Evoucher payment method flow", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))[
      "reward_pm"
    ]["PaymentIntentUSD"];

    cy.task("cli_log", "Create Payment Intent for Evoucher");
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
      "reward_pm"
    ]["Evoucher"];

    cy.task("cli_log", "Confirm Evoucher Payment");
    cy.confirmRewardCallTest(
      fixtures.confirmBody,
      confirmData,
      true,
      globalState
    );

    if (!utils.should_continue_further(confirmData)) return;

    const expected_redirection = fixtures.confirmBody["return_url"];
    const payment_method_type = globalState.get("paymentMethodType");

    cy.task("cli_log", "Handle Redirection");
    cy.handleRewardRedirection(
      globalState,
      payment_method_type,
      expected_redirection
    );

    cy.task("cli_log", "Retrieve Payment");
    cy.retrievePaymentCallTest({ globalState, data: confirmData });
  });

  it("Classic payment method flow", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))[
      "reward_pm"
    ]["PaymentIntentEUR"];

    cy.task("cli_log", "Create Payment Intent for Classic");
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
      "reward_pm"
    ]["Classic"];

    cy.task("cli_log", "Confirm Classic Payment");
    cy.confirmRewardCallTest(
      fixtures.confirmBody,
      confirmData,
      true,
      globalState
    );

    if (!utils.should_continue_further(confirmData)) return;

    const expected_redirection = fixtures.confirmBody["return_url"];
    const payment_method_type = globalState.get("paymentMethodType");

    cy.task("cli_log", "Handle Redirection for Classic");
    cy.handleRewardRedirection(
      globalState,
      payment_method_type,
      expected_redirection
    );

    cy.task("cli_log", "Retrieve Payment");
    cy.retrievePaymentCallTest({ globalState, data: confirmData });
  });
});
