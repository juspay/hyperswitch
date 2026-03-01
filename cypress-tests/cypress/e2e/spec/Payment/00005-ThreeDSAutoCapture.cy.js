import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Card - ThreeDS payment flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Card-ThreeDS payment flow test Create and Confirm", () => {
    it("create payment intent + payment methods call + confirm payment intent + handle redirection", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "PaymentIntent"
      ];

      cy.step("create payment intent", () =>
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "three_ds",
          "automatic",
          globalState
        )
      );

      if (!utils.should_continue_further(data)) return;

      cy.step("payment methods call", () =>
        cy.paymentMethodsCallTest(globalState)
      );

      const confirmData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["3DSAutoCapture"];

      cy.step("confirm payment intent", () =>
        cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState)
      );

      if (!utils.should_continue_further(confirmData)) return;

      const expected_redirection = fixtures.confirmBody["return_url"];
      cy.step("handle redirection", () =>
        cy.handleRedirection(globalState, expected_redirection)
      );
    });
  });
});
