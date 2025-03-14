/*
No 3DS Auto capture with Confirm True
No 3DS Auto capture with Confirm False
No 3DS Manual capture with Confirm True
No 3DS Manual capture with Confirm False
No 3DS Manual multiple capture with Confirm True
No 3DS Manual multiple capture with Confirm False
*/

import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails from "../../configs/Payment/Utils";

let globalState;

// Below is an example of a test that is skipped just because it is not implemented yet
describe("[Payment] [No 3DS] [Payment Method: Card]", () => {
  context("[Payment] [No 3DS] [Capture: Automatic] [Confirm: True]", () => {
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

    it.skip("Create payment intent", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "PaymentIntent"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.paymentIntentCreateCall(
        fixtures.createPaymentBody,
        req_data,
        res_data,
        "no_three_ds",
        "automatic",
        globalState
      );
    });

    it.skip("List payment methods", () => {
      cy.paymentMethodsListCall(globalState);
    });

    it.skip("Confirm payment intent", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "No3DSAutoCapture"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.paymentIntentConfirmCall(
        fixtures.confirmBody,
        req_data,
        res_data,
        true,
        globalState
      );
    });

    it.skip("Retrieve payment intent", () => {
      cy.paymentIntentRetrieveCall(globalState);
    });
  });
  context("[Payment] [No 3DS] [Capture: Automatic] [Confirm: False]", () => {
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

    it.skip("Create Payment Intent", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "PaymentIntent"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.paymentIntentCreateCall(
        fixtures.createPaymentBody,
        req_data,
        res_data,
        "no_three_ds",
        "automatic",
        globalState
      );
    });

    it.skip("Payment Methods", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it.skip("Confirm No 3DS", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "No3DSAutoCapture"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.paymentIntentConfirmCall(
        fixtures.confirmBody,
        req_data,
        res_data,
        true,
        globalState
      );
    });

    it.skip("Retrieve payment intent", () => {
      cy.paymentIntentRetrieveCall(globalState);
    });
  });
});
