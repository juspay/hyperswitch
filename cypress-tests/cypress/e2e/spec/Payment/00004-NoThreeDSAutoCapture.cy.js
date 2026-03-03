import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import { clearSoftExpectErrors, validateAllSoftErrors } from "../../../utils/softExpectHelper";
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
    clearSoftExpectErrors(globalState);
    const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
      "PaymentIntent"
    ];

    cy.step("Create Payment Intent", () =>
      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      )
    );

    if (!utils.should_continue_further(data)) return;

    cy.step("Payment Methods Call", () =>
      cy.paymentMethodsCallTest(globalState)
    );

    const confirmData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["No3DSAutoCapture"];

    cy.step("Confirm Payment", () =>
      cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState)
    );

    if (!utils.should_continue_further(confirmData)) return;

    cy.step("Retrieve Payment", () =>
      cy.retrievePaymentCallTest({ globalState, data: confirmData })
    );

    cy.then(() => {
      validateAllSoftErrors(globalState, "Card-NoThreeDS payment flow test Create and confirm");
    });
  });

  it("Card-NoThreeDS payment flow test Create+Confirm", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
      "No3DSAutoCapture"
    ];

    cy.step("Create and Confirm Payment", () =>
      cy.createConfirmPaymentTest(
        fixtures.createConfirmPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      )
    );

    if (!utils.should_continue_further(data)) return;

    cy.step("Retrieve Payment", () =>
      cy.retrievePaymentCallTest({ globalState, data })
    );
  });

  it("Card-NoThreeDS payment with shipping cost", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
      "PaymentIntentWithShippingCost"
    ];

    cy.step("Create Payment Intent with shipping cost", () =>
      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      )
    );

    if (!utils.should_continue_further(data)) return;

    cy.step("Payment Methods Call", () =>
      cy.paymentMethodsCallTest(globalState)
    );

    const confirmData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["PaymentConfirmWithShippingCost"];

    cy.step("Confirm Payment with shipping cost", () =>
      cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState)
    );

    if (!utils.should_continue_further(confirmData)) return;

    cy.step("Retrieve Payment with shipping cost", () =>
      cy.retrievePaymentCallTest({ globalState, data: confirmData })
    );
  });
});
