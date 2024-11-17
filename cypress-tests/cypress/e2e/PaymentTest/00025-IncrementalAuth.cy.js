import * as fixtures from "../../fixtures/imports";
import State from "../../utils/State";
import getConnectorDetails, * as utils from "../PaymentUtils/Utils";

let globalState;

describe("[Payment] Incremental Auth", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("[Payment] Incremental Pre-Auth", () => {
    it("[Payment] Create Payment Intent", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "PaymentIntentIncrementalAuthorization"
      ];

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );

      if (should_continue)
        should_continue = utils.should_continue_further(data);
    });
    it("[Payment] Confirm Payment Intent", () => {});
    it("[Payment] Incremental Authorization", () => {});
    it("[Payment] Capture Payment Intent", () => {});
  });
  context("", () => {});
});
