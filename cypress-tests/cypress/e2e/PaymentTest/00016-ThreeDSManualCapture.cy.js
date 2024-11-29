import captureBody from "../../fixtures/capture-flow-body.json";
import * as fixtures from "../../fixtures/imports";
import State from "../../utils/State";
import getConnectorDetails, * as utils from "../PaymentUtils/Utils";

let globalState;

describe("Card - ThreeDS Manual payment flow test", () => {
  let should_continue = true;

  beforeEach(function () {
    if (!should_continue) {
      this.skip();
    }
  });

  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Card - ThreeDS Manual Full Capture payment flow test", () => {
    context("payment Create and Confirm", () => {
      let should_continue = true; // variable that will be used to skip tests if a previous test fails

      beforeEach(function () {
        if (!should_continue) {
          this.skip();
        }
      });

      it("create-payment-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
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
          fixtures.confirmBody,
          req_data,
          res_data,
          true,
          globalState
        );
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("Handle redirection", () => {
        let expected_redirection = fixtures.confirmBody["return_url"];
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
      let should_continue = true; // variable that will be used to skip tests if a previous test fails

      beforeEach(function () {
        if (!should_continue) {
          this.skip();
        }
      });

      it("create+confirm-payment-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["3DSManualCapture"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
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
        let expected_redirection =
          fixtures.createConfirmPaymentBody["return_url"];
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
        let should_continue = true; // variable that will be used to skip tests if a previous test fails

        beforeEach(function () {
          if (!should_continue) {
            this.skip();
          }
        });

        it("create-payment-call-test", () => {
          let data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentIntent"];
          let req_data = data["Request"];
          let res_data = data["Response"];
          cy.createPaymentIntentTest(
            fixtures.createPaymentBody,
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
            fixtures.confirmBody,
            req_data,
            res_data,
            true,
            globalState
          );
          if (should_continue)
            should_continue = utils.should_continue_further(res_data);
        });

        it("Handle redirection", () => {
          let expected_redirection = fixtures.confirmBody["return_url"];
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
        let should_continue = true; // variable that will be used to skip tests if a previous test fails

        beforeEach(function () {
          if (!should_continue) {
            this.skip();
          }
        });

        it("create+confirm-payment-call-test", () => {
          let data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["3DSManualCapture"];
          let req_data = data["Request"];
          let res_data = data["Response"];
          cy.createConfirmPaymentTest(
            fixtures.createConfirmPaymentBody,
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
          let expected_redirection =
            fixtures.createConfirmPaymentBody["return_url"];
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
