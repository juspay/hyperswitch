import * as fixtures from "../../fixtures/imports";
import State from "../../utils/State";
import getConnectorDetails, * as utils from "../PaymentUtils/Utils";

let globalState;

describe("Customer Create flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

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
      fixtures.createPaymentBody,
      req_data,
      res_data,
      "no_three_ds",
      "automatic",
      globalState
    );
    if (should_continue)
      should_continue = utils.should_continue_further(res_data);
  });
  it("session-call-test", () => {
    let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
      "SessionToken"
    ];
    let res_data = data["Response"];

    cy.sessionTokenCall(fixtures.sessionTokenBody, res_data, globalState);
  });
});
