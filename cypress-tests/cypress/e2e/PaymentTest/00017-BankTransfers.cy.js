import * as fixtures from "../../fixtures/imports";
import State from "../../utils/State";
import getConnectorDetails, * as utils from "../PaymentUtils/Utils";

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

  context("Bank transfer - Pix forward flow", () => {
    let should_continue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!should_continue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["PaymentIntent"];

      let configs = validateConfig(data["Configs"]);
      let req_data = data["Request"];
      let res_data = data["Response"];

      cy.createPaymentIntentTest(
        configs,
        fixtures.createPaymentBody,
        req_data,
        res_data,
        "three_ds",
        "automatic",
        globalState
      );

      if (should_continue)
        should_continue = utils.should_continue_further(res_data, configs);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm bank transfer", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["Pix"];

      let configs = validateConfig(data["Configs"]);
      let req_data = data["Request"];
      let res_data = data["Response"];

      cy.confirmBankTransferCallTest(
        configs,
        fixtures.confirmBody,
        req_data,
        res_data,
        true,
        globalState
      );

      if (should_continue)
        should_continue = utils.should_continue_further(res_data, configs);
    });

    it("Handle bank transfer redirection", () => {
      let expected_redirection = fixtures.confirmBody["return_url"];
      let payment_method_type = globalState.get("paymentMethodType");

      cy.handleBankTransferRedirection(
        globalState,
        payment_method_type,
        expected_redirection
      );
    });
  });
});
