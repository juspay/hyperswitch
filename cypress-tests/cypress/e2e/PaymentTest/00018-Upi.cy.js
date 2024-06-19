import confirmBody from "../../fixtures/confirm-body.json";
import createPaymentBody from "../../fixtures/create-payment-body.json";
import State from "../../utils/State";
import getConnectorDetails, * as utils from "../PaymentUtils/utils";

let globalState;

describe("Upi tests", () => {
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
    afterEach("flush global state", () => {
        cy.task("setGlobalState", globalState.data);
      });

      context("UPI Create and Confirm flow test", () => {
        it("create-payment-call-test", () => {
          let data = getConnectorDetails(globalState.get("connectorId"))[
            "upi_pm"
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
    });

    it("Confirm upi", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "upi_pm"
        ]["UpiAutoCapture"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.confirmUpiCallTest(
          confirmBody,
          req_data,
          res_data,
          globalState
        );
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

    it("Handle upi redirection", () => {
        let expected_redirection = confirmBody["return_url"];
        let payment_method_type = globalState.get("paymentMethodType");
        cy.handleUpiRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
    });

    it("retrieve-payment-call-test", () => {
        cy.retrievePaymentCallTest(globalState);
    });
})