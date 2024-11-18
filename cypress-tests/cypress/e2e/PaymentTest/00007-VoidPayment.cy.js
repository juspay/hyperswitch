import * as fixtures from "../../fixtures/imports";
import State from "../../utils/State";
import { validateConfig } from "../../utils/featureFlags";
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

  context("Card - void payment in Requires_capture state flow test", () => {
    let should_continue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!should_continue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "PaymentIntent"
      ];

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
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "No3DSManualCapture"
      ];

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

    it("void-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "VoidAfterConfirm"
      ];

      let configs = validateConfig(data["Configs"]);
      let req_data = data["Request"];
      let res_data = data["Response"];

      cy.voidCallTest(
        fixtures.voidBody,
        req_data,
        res_data,
        globalState,
        configs
      );

      if (should_continue)
        should_continue = utils.should_continue_further(res_data, configs);
    });
  });

  context(
    "Card - void payment in Requires_payment_method state flow test",
    () => {
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

      it("void-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Void"];

        let configs = validateConfig(data["Configs"]);
        let req_data = data["Request"];
        let res_data = data["Response"];

        cy.voidCallTest(
          fixtures.voidBody,
          req_data,
          res_data,
          globalState,
          configs
        );

        if (should_continue)
          should_continue = utils.should_continue_further(res_data, configs);
      });
    }
  );

  context("Card - void payment in success state flow test", () => {
    let should_continue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!should_continue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "PaymentIntent"
      ];

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
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "No3DSManualCapture"
      ];

      let configs = validateConfig(data["Configs"]);
      let req_data = data["Request"];
      let res_data = data["Response"];

      cy.confirmCallTest(
        fixtures.confirmBody,
        req_data,
        res_data,
        false,
        globalState,
        configs
      );

      if (should_continue)
        should_continue = utils.should_continue_further(res_data, configs);
    });

    it("void-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "VoidAfterConfirm"
      ];

      let configs = validateConfig(data["Configs"]);
      let req_data = data["Request"];
      let res_data = data["Response"];

      cy.voidCallTest(
        fixtures.voidBody,
        req_data,
        res_data,
        globalState,
        configs
      );

      if (should_continue)
        should_continue = utils.should_continue_further(res_data, configs);
    });
  });
});
