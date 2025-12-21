/*
V2 Void/Cancel Payment Tests
Test scenarios:
1. Void payment in requires_capture state
2. Void payment in requires_payment_method state
3. Void payment in succeeded state (should fail)
*/

import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, {
  should_continue_further,
} from "../../configs/Payment/Utils";

let globalState;

describe("[Payment] [Void/Cancel] [Payment Method: Card]", () => {
  context("[Payment] [Void] [Requires Capture State]", () => {
    let should_continue = true;

    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    beforeEach(function () {
      if (!should_continue) {
        this.skip();
      }
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("Create payment intent", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PaymentIntent"];
      const req_data = data["Request"];
      const res_data = data["Response"];

      cy.paymentIntentCreateCall(
        globalState,
        req_data,
        res_data,
        "no_three_ds",
        "manual"
      );

      if (should_continue) should_continue = should_continue_further(data);
    });

    it("Confirm payment intent", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSManualCapture"];
      const req_data = data["Request"];

      cy.paymentConfirmCall(globalState, req_data, data);

      if (should_continue) should_continue = should_continue_further(data);
    });

    it("Void payment intent", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["VoidAfterConfirm"];

      cy.paymentVoidCall(globalState, fixtures.void_payment_body, data);

      if (should_continue) should_continue = should_continue_further(data);
    });
  });

  context(
    "[Payment] [Void] [Requires Payment Method State - Should Fail]",
    () => {
      let should_continue = true;

      before("seed global state", () => {
        cy.task("getGlobalState").then((state) => {
          globalState = new State(state);
        });
      });

      beforeEach(function () {
        if (!should_continue) {
          this.skip();
        }
      });

      after("flush global state", () => {
        cy.task("setGlobalState", globalState.data);
      });

      it("Create payment intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];
        const req_data = data["Request"];
        const res_data = data["Response"];

        cy.paymentIntentCreateCall(
          globalState,
          req_data,
          res_data,
          "no_three_ds",
          "manual"
        );

        if (should_continue) should_continue = should_continue_further(data);
      });

      it("Void payment intent - should fail", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Void"];

        // Use the ResponseCustom which contains the error response
        const void_data = {
          ...data,
          Response: data.ResponseCustom,
        };

        cy.paymentVoidCall(globalState, fixtures.void_payment_body, void_data);

        if (should_continue) should_continue = should_continue_further(data);
      });
    }
  );

  context("[Payment] [Void] [Succeeded State - Should Fail]", () => {
    let should_continue = true;

    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    beforeEach(function () {
      if (!should_continue) {
        this.skip();
      }
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("Create payment intent", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PaymentIntent"];

      const req_data = data["Request"];
      const res_data = data["Response"];

      cy.paymentIntentCreateCall(
        globalState,
        req_data,
        res_data,
        "no_three_ds",
        "automatic"
      );

      if (should_continue) should_continue = should_continue_further(data);
    });

    it("Confirm payment intent", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSAutoCapture"];
      const req_data = data["Request"];

      cy.paymentConfirmCall(globalState, req_data, data);

      if (should_continue) should_continue = should_continue_further(data);
    });

    it("Void payment intent - should fail", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["VoidAfterConfirm"];

      // Use the ResponseCustom which contains the error response
      const void_data = {
        ...data,
        Response: data.ResponseCustom,
      };

      cy.paymentVoidCall(globalState, fixtures.void_payment_body, void_data);

      if (should_continue) should_continue = should_continue_further(data);
    });
  });
});
