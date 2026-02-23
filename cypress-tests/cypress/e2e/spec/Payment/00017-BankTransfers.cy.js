import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Bank Transfers", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  it("Bank transfer - Pix forward flow", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))[
      "bank_transfer_pm"
    ]["PaymentIntent"]("Pix");

    cy.step("Create Payment Intent for Pix", () =>
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
      "bank_transfer_pm"
    ]["Pix"];

    cy.step("Confirm Bank Transfer for Pix", () =>
      cy.confirmBankTransferCallTest(
        fixtures.confirmBody,
        confirmData,
        true,
        globalState
      )
    );

    if (!utils.should_continue_further(confirmData)) return;

    const expected_redirection = fixtures.confirmBody["return_url"];
    const payment_method_type = globalState.get("paymentMethodType");

    cy.step("Handle Bank Transfer Redirection for Pix", () =>
      cy.handleBankTransferRedirection(
        globalState,
        payment_method_type,
        expected_redirection
      )
    );
  });

  it("Bank transfer - Instant Bank Transfer Finland forward flow", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))[
      "bank_transfer_pm"
    ]["PaymentIntent"]("InstantBankTransferFinland");

    cy.step("Create Payment Intent ", () =>
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
      "bank_transfer_pm"
    ]["InstantBankTransferFinland"];

    cy.step("Confirm Bank Transfer ", () =>
      cy.confirmBankTransferCallTest(
        fixtures.confirmBody,
        confirmData,
        true,
        globalState
      )
    );

    if (!utils.should_continue_further(confirmData)) return;

    const expected_redirection = fixtures.confirmBody["return_url"];
    const payment_method_type = globalState.get("paymentMethodType");

    cy.step("Handle Bank Transfer Redirection ", () =>
      cy.handleBankTransferRedirection(
        globalState,
        payment_method_type,
        expected_redirection
      )
    );
  });

  it("Bank transfer - Instant Bank Transfer Poland forward flow", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))[
      "bank_transfer_pm"
    ]["PaymentIntent"]("InstantBankTransferPoland");

    cy.step("Create Payment Intent ", () =>
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
      "bank_transfer_pm"
    ]["InstantBankTransferPoland"];

    cy.step("Confirm Bank Transfer ", () =>
      cy.confirmBankTransferCallTest(
        fixtures.confirmBody,
        confirmData,
        true,
        globalState
      )
    );

    if (!utils.should_continue_further(confirmData)) return;

    const expected_redirection = fixtures.confirmBody["return_url"];
    const payment_method_type = globalState.get("paymentMethodType");

    cy.step("Handle Bank Transfer Redirection ", () =>
      cy.handleBankTransferRedirection(
        globalState,
        payment_method_type,
        expected_redirection
      )
    );
  });

  it("Bank transfer - Ach flow", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))[
      "bank_transfer_pm"
    ]["PaymentIntent"]("Ach");

    cy.step("Create Payment Intent ", () =>
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
      "bank_transfer_pm"
    ]["Ach"];

    cy.step("Confirm Bank Transfer ", () =>
      cy.confirmBankTransferCallTest(
        fixtures.confirmBody,
        confirmData,
        true,
        globalState
      )
    );

    if (!utils.should_continue_further(confirmData)) return;

    const expected_redirection = fixtures.confirmBody["return_url"];
    const payment_method_type = globalState.get("paymentMethodType");

    if (globalState.get("connectorId") != "checkbook") {
      cy.step("Handle Bank Transfer Redirection ", () =>
        cy.handleBankTransferRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        )
      );
    }
  });
});
