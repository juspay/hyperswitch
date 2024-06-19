import confirmBody from "../../fixtures/confirm-body.json";
import createPaymentBody from "../../fixtures/create-payment-body.json";
import State from "../../utils/State";
import getConnectorDetails, * as utils from "../PaymentUtils/utils";

let globalState;

describe("Bank Transfers", () => {
  let should_continue = true; // variable that will be used to skip tests if a previous test fails

  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  beforeEach(function () {
    if (!should_continue) {
      this.skip();
    }
  });

  context("Bank transfer - Pix forward flow", () => {
    it("create-payment-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["PaymentIntent"];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.createPaymentIntentTest(
        createPaymentBody,
        req_data,
        res_data,
        "three_ds",
        "automatic",
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm bank transfer", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["Pix"];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.confirmBankTransferCallTest(
        confirmBody,
        req_data,
        res_data,
        true,
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("Handle bank transfer redirection", () => {
      let expected_redirection = confirmBody["return_url"];
      let payment_method_type = globalState.get("paymentMethodType");
      cy.handleBankTransferRedirection(
        globalState,
        payment_method_type,
        expected_redirection
      );
    });
  });
});
