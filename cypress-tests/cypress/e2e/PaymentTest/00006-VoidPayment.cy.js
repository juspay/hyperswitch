import confirmBody from "../../fixtures/confirm-body.json";
import createPaymentBody from "../../fixtures/create-payment-body.json";
import voidBody from "../../fixtures/void-payment-body.json";
import State from "../../utils/State";
import getConnectorDetails from "../PaymentUtils/utils";
import * as utils from "../PaymentUtils/utils";

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
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.createPaymentIntentTest(
        createPaymentBody,
        req_data,
        res_data,
        "no_three_ds",
        "manual",
        globalState,
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("confirm-call-test", () => {
      console.log("confirm -> " + globalState.get("connectorId"));
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "No3DSManualCapture"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      console.log("det -> " + data.card);
      cy.confirmCallTest(confirmBody, req_data, res_data, true, globalState);
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("void-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "Void"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.voidCallTest(voidBody, req_data, res_data, globalState);
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
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
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.createPaymentIntentTest(
          createPaymentBody,
          req_data,
          res_data,
          "no_three_ds",
          "manual",
          globalState,
        );
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("payment_methods-call-test", () => {
        cy.paymentMethodsCallTest(globalState);
      });

      it("void-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Void"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.voidCallTest(voidBody, req_data, res_data, globalState);
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });
    },
  );

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
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.createPaymentIntentTest(
          createPaymentBody,
          req_data,
          res_data,
          "no_three_ds",
          "manual",
          globalState,
        );
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("payment_methods-call-test", () => {
        cy.paymentMethodsCallTest(globalState);
      });

      it("confirm-call-test", () => {
        console.log("confirm -> " + globalState.get("connectorId"));
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSManualCapture"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        console.log("det -> " + data.card);
        cy.confirmCallTest(confirmBody, req_data, res_data, false, globalState);
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("void-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Void"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.voidCallTest(voidBody, req_data, res_data, globalState);
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });
    },
  );
});
