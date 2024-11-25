import * as fixtures from "../../fixtures/imports";
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

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "manual",
        globalState
      );

      if (should_continue)
        should_continue = utils.should_continue_further(data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("confirm-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "No3DSManualCapture"
      ];

      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

      if (should_continue)
        should_continue = utils.should_continue_further(data);
    });

    it("void-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "VoidAfterConfirm"
      ];

      cy.voidCallTest(fixtures.voidBody, data, globalState);

      if (should_continue)
        should_continue = utils.should_continue_further(data);
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

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "manual",
          globalState
        );

        if (should_continue)
          should_continue = utils.should_continue_further(data);
      });

      it("payment_methods-call-test", () => {
        cy.paymentMethodsCallTest(globalState);
      });

      it("void-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Void"];

        cy.voidCallTest(fixtures.voidBody, data, globalState);

        if (should_continue)
          should_continue = utils.should_continue_further(data);
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

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "manual",
        globalState
      );

      if (should_continue)
        should_continue = utils.should_continue_further(data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("confirm-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "No3DSManualCapture"
      ];

      cy.confirmCallTest(fixtures.confirmBody, data, false, globalState);

      if (should_continue)
        should_continue = utils.should_continue_further(data);
    });

    it("void-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "VoidAfterConfirm"
      ];

      cy.voidCallTest(fixtures.voidBody, data, globalState);

      if (should_continue)
        should_continue = utils.should_continue_further(data);
    });
  });
});
