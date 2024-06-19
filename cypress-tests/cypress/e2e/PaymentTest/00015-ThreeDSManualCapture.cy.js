import captureBody from "../../fixtures/capture-flow-body.json";
import confirmBody from "../../fixtures/confirm-body.json";
import createConfirmPaymentBody from "../../fixtures/create-confirm-body.json";
import createPaymentBody from "../../fixtures/create-payment-body.json";
import State from "../../utils/State";
import getConnectorDetails, * as utils from "../PaymentUtils/utils";

let globalState;

describe("Card - ThreeDS Manual payment flow test", () => {
  let should_continue = true; // variable that will be used to skip tests if a previous test fails

  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  beforeEach(function () {
    if (!should_continue) {
      this.skip();
    }
  });
  context("Card - ThreeDS Manual Full Capture payment flow test", () => {
    context("payment Create and Confirm", () => {
      it("create-payment-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.createPaymentIntentTest(
          createPaymentBody,
          req_data,
          res_data,
          "three_ds",
          "manual",
          globalState
        );
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("payment_methods-call-test", () => {
        cy.paymentMethodsCallTest(globalState);
      });

      it("confirm-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["3DSManualCapture"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.confirmCallTest(confirmBody, req_data, res_data, true, globalState);
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("Handle redirection", () => {
        let expected_redirection = confirmBody["return_url"];
        cy.handleRedirection(globalState, expected_redirection);
      });

      it("retrieve-payment-call-test", () => {
        cy.retrievePaymentCallTest(globalState);
      });

      it("capture-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.captureCallTest(captureBody, req_data, res_data, 6500, globalState);
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("retrieve-payment-call-test", () => {
        cy.retrievePaymentCallTest(globalState);
      });
    });

    context("Payment Create+Confirm", () => {
      it("create+confirm-payment-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["3DSManualCapture"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.createConfirmPaymentTest(
          createConfirmPaymentBody,
          req_data,
          res_data,
          "three_ds",
          "manual",
          globalState
        );
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("Handle redirection", () => {
        let expected_redirection = createConfirmPaymentBody["return_url"];
        cy.handleRedirection(globalState, expected_redirection);
      });

      it("retrieve-payment-call-test", () => {
        cy.retrievePaymentCallTest(globalState);
      });

      it("capture-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.captureCallTest(captureBody, req_data, res_data, 6500, globalState);
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("retrieve-payment-call-test", () => {
        cy.retrievePaymentCallTest(globalState);
      });
    });
  });

  context(
    "Card - ThreeDS Manual Partial Capture payment flow test - Create and Confirm",
    () => {
      context("payment Create and Payment Confirm", () => {
        it("create-payment-call-test", () => {
          let data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentIntent"];
          let req_data = data["Request"];
          let res_data = data["Response"];
          cy.createPaymentIntentTest(
            createPaymentBody,
            req_data,
            res_data,
            "three_ds",
            "manual",
            globalState
          );
          if (should_continue)
            should_continue = utils.should_continue_further(res_data);
        });

        it("payment_methods-call-test", () => {
          cy.paymentMethodsCallTest(globalState);
        });

        it("confirm-call-test", () => {
          let data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["3DSManualCapture"];
          let req_data = data["Request"];
          let res_data = data["Response"];
          cy.confirmCallTest(
            confirmBody,
            req_data,
            res_data,
            true,
            globalState
          );
          if (should_continue)
            should_continue = utils.should_continue_further(res_data);
        });

        it("Handle redirection", () => {
          let expected_redirection = confirmBody["return_url"];
          cy.handleRedirection(globalState, expected_redirection);
        });

        it("retrieve-payment-call-test", () => {
          cy.retrievePaymentCallTest(globalState);
        });

        it("capture-call-test", () => {
          let data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PartialCapture"];
          let req_data = data["Request"];
          let res_data = data["Response"];
          cy.captureCallTest(captureBody, req_data, res_data, 100, globalState);
          if (should_continue)
            should_continue = utils.should_continue_further(res_data);
        });

        it("retrieve-payment-call-test", () => {
          cy.retrievePaymentCallTest(globalState);
        });
      });

      context("payment + Confirm", () => {
        it("create+confirm-payment-call-test", () => {
          let data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["3DSManualCapture"];
          let req_data = data["Request"];
          let res_data = data["Response"];
          cy.createConfirmPaymentTest(
            createConfirmPaymentBody,
            req_data,
            res_data,
            "three_ds",
            "manual",
            globalState
          );
          if (should_continue)
            should_continue = utils.should_continue_further(res_data);
        });

        it("Handle redirection", () => {
          let expected_redirection = createConfirmPaymentBody["return_url"];
          cy.handleRedirection(globalState, expected_redirection);
        });

        it("retrieve-payment-call-test", () => {
          cy.retrievePaymentCallTest(globalState);
        });

        it("capture-call-test", () => {
          let data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PartialCapture"];
          let req_data = data["Request"];
          let res_data = data["Response"];
          cy.captureCallTest(captureBody, req_data, res_data, 100, globalState);
          if (should_continue)
            should_continue = utils.should_continue_further(res_data);
        });

        it("retrieve-payment-call-test", () => {
          cy.retrievePaymentCallTest(globalState);
        });
      });
    }
  );
});
