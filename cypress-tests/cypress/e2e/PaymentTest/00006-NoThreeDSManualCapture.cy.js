import * as fixtures from "../../fixtures/imports";
import { validateConfig } from "../../utils/featureFlags";
import State from "../../utils/State";
import getConnectorDetails, * as utils from "../PaymentUtils/Utils";

let globalState;

describe("Card - NoThreeDS Manual payment flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Card - NoThreeDS Manual Full Capture payment flow test", () => {
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

        let configs = validateConfig(data["Configs"]);
        let req_data = data["Request"];
        let res_data = data["Response"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          req_data,
          res_data,
          "no_three_ds",
          "manual",
          globalState,
          configs
        );

        if (should_continue)
          should_continue = utils.should_continue_further(res_data, configs);
      });

      it("payment_methods-call-test", () => {
        cy.paymentMethodsCallTest(globalState);
      });

      it("confirm-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSManualCapture"];

        let configs = validateConfig(data["Configs"]);
        let req_data = data["Request"];
        let res_data = data["Response"];

        cy.confirmCallTest(
          fixtures.confirmBody,
          req_data,
          res_data,
          true,
          globalState,
          configs
        );

        if (should_continue)
          should_continue = utils.should_continue_further(res_data, configs);
      });

      it("retrieve-payment-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSManualCapture"];

        let configs = validateConfig(data["Configs"]);

        cy.retrievePaymentCallTest(globalState, configs);
      });

      it("capture-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];

        let configs = validateConfig(data["Configs"]);
        let req_data = data["Request"];
        let res_data = data["Response"];

        cy.captureCallTest(
          fixtures.captureBody,
          req_data,
          res_data,
          6500,
          globalState,
          configs
        );

        if (should_continue)
          should_continue = utils.should_continue_further(res_data, configs);
      });

      it("retrieve-payment-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];

        let configs = validateConfig(data["Configs"]);

        cy.retrievePaymentCallTest(globalState, configs);
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
        ]["No3DSManualCapture"];

        let configs = validateConfig(data["Configs"]);
        let req_data = data["Request"];
        let res_data = data["Response"];

        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          req_data,
          res_data,
          "no_three_ds",
          "manual",
          globalState,
          configs
        );

        if (should_continue)
          should_continue = utils.should_continue_further(res_data, configs);
      });

      it("retrieve-payment-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSManualCapture"];

        let configs = validateConfig(data["Configs"]);

        cy.retrievePaymentCallTest(globalState, configs);
      });

      it("capture-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];

        let configs = validateConfig(data["Configs"]);
        let req_data = data["Request"];
        let res_data = data["Response"];

        cy.captureCallTest(
          fixtures.captureBody,
          req_data,
          res_data,
          6500,
          globalState,
          configs
        );

        if (should_continue)
          should_continue = utils.should_continue_further(res_data, configs);
      });

      it("retrieve-payment-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];

        let configs = validateConfig(data["Configs"]);

        cy.retrievePaymentCallTest(globalState, configs);
      });
    });
  });

  context(
    "Card - NoThreeDS Manual Partial Capture payment flow test - Create and Confirm",
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

          let configs = validateConfig(data["Configs"]);
          let req_data = data["Request"];
          let res_data = data["Response"];

          cy.createPaymentIntentTest(
            fixtures.createPaymentBody,
            req_data,
            res_data,
            "no_three_ds",
            "manual",
            globalState,
            configs
          );

          if (should_continue)
            should_continue = utils.should_continue_further(res_data, configs);
        });

        it("payment_methods-call-test", () => {
          cy.paymentMethodsCallTest(globalState);
        });

        it("confirm-call-test", () => {
          let data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["No3DSManualCapture"];

          let configs = validateConfig(data["Configs"]);
          let req_data = data["Request"];
          let res_data = data["Response"];

          cy.confirmCallTest(
            fixtures.confirmBody,
            req_data,
            res_data,
            true,
            globalState,
            configs
          );

          if (should_continue)
            should_continue = utils.should_continue_further(res_data, configs);
        });

        it("retrieve-payment-call-test", () => {
          let data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["No3DSManualCapture"];

          let configs = validateConfig(data["Configs"]);

          cy.retrievePaymentCallTest(globalState, configs);
        });

        it("capture-call-test", () => {
          let data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PartialCapture"];

          let configs = validateConfig(data["Configs"]);
          let req_data = data["Request"];
          let res_data = data["Response"];

          cy.captureCallTest(
            fixtures.captureBody,
            req_data,
            res_data,
            100,
            globalState,
            configs
          );

          if (should_continue)
            should_continue = utils.should_continue_further(res_data, configs);
        });

        it("retrieve-payment-call-test", () => {
          let data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PartialCapture"];

          let configs = validateConfig(data["Configs"]);

          cy.retrievePaymentCallTest(globalState, configs);
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
          ]["No3DSManualCapture"];

          let configs = validateConfig(data["Configs"]);
          let req_data = data["Request"];
          let res_data = data["Response"];

          cy.createConfirmPaymentTest(
            fixtures.createConfirmPaymentBody,
            req_data,
            res_data,
            "no_three_ds",
            "manual",
            globalState,
            configs
          );

          if (should_continue)
            should_continue = utils.should_continue_further(res_data, configs);
        });

        it("retrieve-payment-call-test", () => {
          let data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["No3DSManualCapture"];

          let configs = validateConfig(data["Configs"]);

          cy.retrievePaymentCallTest(globalState, configs);
        });

        it("capture-call-test", () => {
          let data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PartialCapture"];

          let configs = validateConfig(data["Configs"]);
          let req_data = data["Request"];
          let res_data = data["Response"];

          cy.captureCallTest(
            fixtures.captureBody,
            req_data,
            res_data,
            100,
            globalState,
            configs
          );

          if (should_continue)
            should_continue = utils.should_continue_further(res_data, configs);
        });

        it("retrieve-payment-call-test", () => {
          let data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PartialCapture"];

          let configs = validateConfig(data["Configs"]);

          cy.retrievePaymentCallTest(globalState, configs);
        });
      });
    }
  );
});
