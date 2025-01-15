import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Card - Sync payment flow test", () => {
  let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

  beforeEach(function () {
    if (!shouldContinue) {
      this.skip();
    }
  });

  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });
  it("create-payment-call-test", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
      "PaymentIntent"
    ];

    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      data,
      "no_three_ds",
      "automatic",
      globalState
    );

    if (shouldContinue) shouldContinue = utils.should_continue_further(data);
  });

  it("payment_methods-call-test", () => {
    cy.paymentMethodsCallTest(globalState);
  });

  it("confirm-call-test", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
      "No3DSAutoCapture"
    ];

    cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

    if (shouldContinue) shouldContinue = utils.should_continue_further(data);
  });

  it("retrieve-payment-call-test", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
      "No3DSAutoCapture"
    ];
    cy.retrievePaymentCallTest(globalState, data);
  });
});
