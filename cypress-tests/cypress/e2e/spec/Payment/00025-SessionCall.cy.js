import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

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

  const shouldContinue = true; // variable that will be used to skip tests if a previous test fails

  beforeEach(function () {
    if (!shouldContinue) {
      this.skip();
    }
  });
  it("create-payment-call-test", () => {
    let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

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

  it("session-call-test", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
      "SessionToken"
    ];

    cy.sessionTokenCall(fixtures.sessionTokenBody, data, globalState);
  });
});
