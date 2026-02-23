import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Crypto Payment", () => {
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

  it("Crypto Currency Payment flow", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))[
      "crypto_pm"
    ]["PaymentIntent"];

    cy.step("Create Payment Intent", () =>
      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      )
    );

    if (!utils.should_continue_further(data)) return;

    cy.step("Payment Methods Call", () =>
      cy.paymentMethodsCallTest(globalState)
    );

    const confirmData = getConnectorDetails(globalState.get("connectorId"))[
      "crypto_pm"
    ]["CryptoCurrency"];

    cy.step("Confirm Crypto Currency Payment", () =>
      cy.confirmRewardCallTest(
        fixtures.confirmBody,
        confirmData,
        true,
        globalState
      )
    );

    if (!utils.should_continue_further(confirmData)) return;

    const expected_redirection = fixtures.confirmBody["return_url"];
    const payment_method_type = globalState.get("paymentMethodType");

    cy.step("Handle Redirection", () =>
      cy.handleCryptoRedirection(
        globalState,
        payment_method_type,
        expected_redirection
      )
    );

    cy.step("Retrieve Payment", () =>
      cy.retrievePaymentCallTest({ globalState })
    );
  });

  it("Crypto Currency manual capture flow", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))[
      "crypto_pm"
    ]["PaymentIntent"];

    cy.step("Create Payment Intent (Manual)", () =>
      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "manual",
        globalState
      )
    );

    if (!utils.should_continue_further(data)) return;

    cy.step("Payment Methods Call", () =>
      cy.paymentMethodsCallTest(globalState)
    );

    const confirmData = getConnectorDetails(globalState.get("connectorId"))[
      "crypto_pm"
    ]["CryptoCurrencyManualCapture"];

    cy.step("Confirm Crypto Currency Payment (Manual Capture)", () =>
      cy.confirmRewardCallTest(
        fixtures.confirmBody,
        confirmData,
        true,
        globalState
      )
    );

    if (!utils.should_continue_further(confirmData)) return;

    const expected_redirection = fixtures.confirmBody["return_url"];
    const payment_method_type = globalState.get("paymentMethodType");

    cy.step("Handle Redirection", () =>
      cy.handleCryptoRedirection(
        globalState,
        payment_method_type,
        expected_redirection
      )
    );

    cy.step("Retrieve Payment", () =>
      cy.retrievePaymentCallTest({ globalState })
    );
  });
});
