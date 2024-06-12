import confirmBody from "../../fixtures/confirm-body.json";
import createPaymentBody from "../../fixtures/create-payment-body.json";
import State from "../../utils/State";
import getConnectorDetails, * as utils from "../PaymentUtils/utils";

let globalState;

describe("Card - ThreeDS payment flow test", () => {
  let should_continue = true; // variable that will be used to skip tests if a previous test fails

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

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
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
      "three_ds",
      "automatic",
      globalState
    );
    if (should_continue)
      should_continue = utils.should_continue_further(res_data);
  });

  it("payment_methods-call-test", () => {
    cy.paymentMethodsCallTest(globalState);
  });

  it("Confirm 3DS", () => {
    let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
      "3DSAutoCapture"
    ];
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
});
