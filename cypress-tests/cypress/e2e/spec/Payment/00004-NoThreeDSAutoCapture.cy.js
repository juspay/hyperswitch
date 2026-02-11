import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Card - NoThreeDS payment flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  it("Card-NoThreeDS payment flow test Create and confirm", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"];

    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      data["PaymentIntent"],
      "no_three_ds",
      "automatic",
      globalState
    );

    cy.paymentMethodsCallTest(globalState);

    cy.confirmCallTest(
      fixtures.confirmBody,
      data["No3DSAutoCapture"],
      true,
      globalState
    );

    cy.retrievePaymentCallTest({
      globalState,
      data: data["No3DSAutoCapture"],
    });
  });

  it("Card-NoThreeDS payment flow test Create+Confirm", () => {
    const data =
      getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "No3DSAutoCapture"
      ];

    cy.createConfirmPaymentTest(
      fixtures.createConfirmPaymentBody,
      data,
      "no_three_ds",
      "automatic",
      globalState
    );

    cy.retrievePaymentCallTest({ globalState, data });
  });

  it("Card-NoThreeDS payment with shipping cost", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"];

    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      data["PaymentIntentWithShippingCost"],
      "no_three_ds",
      "automatic",
      globalState
    );

    cy.paymentMethodsCallTest(globalState);

    cy.confirmCallTest(
      fixtures.confirmBody,
      data["PaymentConfirmWithShippingCost"],
      true,
      globalState
    );

    cy.retrievePaymentCallTest({
      globalState,
      data: data["PaymentConfirmWithShippingCost"],
    });
  });
});
