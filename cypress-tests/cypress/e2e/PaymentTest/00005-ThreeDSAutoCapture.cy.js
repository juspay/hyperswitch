import * as fixtures from "../../fixtures/imports";
import State from "../../utils/State";
import getConnectorDetails, * as utils from "../PaymentUtils/Utils";

let globalState;

describe("Card - ThreeDS payment flow test", () => {
  let should_continue = true; // variable that will be used to skip tests if a previous test fails

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

  it("create-payment-call-test", () => {
    let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
      "PaymentIntent"
    ];

    let configs = validateConfig(data["Configs"]);
    let req_data = data["Request"];
    let res_data = data["Response"];

    cy.createPaymentIntentTest(
      configs,
      fixtures.createPaymentBody,
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

    let configs = validateConfig(data["Configs"]);
    let req_data = data["Request"];
    let res_data = data["Response"];

    cy.confirmCallTest(
      configs,
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
});
