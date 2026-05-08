import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("[Payment] Complete Authorize", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  context("[Payment] Complete Authorize - 3DS Manual Capture Flow", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    afterEach("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("[Payment] Create Payment Intent for 3DS Manual", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PaymentIntent"];

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "three_ds",
        "manual",
        globalState
      );

      if (shouldContinue) {
        shouldContinue = utils.should_continue_further(data);
      }
    });

    it("[Payment] Confirm Payment Intent (3DS Manual)", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["3DSManualCapture"];

      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

      if (shouldContinue) {
        shouldContinue = utils.should_continue_further(data);
      }
    });

    it("[Payment] Complete Authorize Call", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["CompleteAuthorize"];

      cy.completeAuthorizeCallTest(
        fixtures.completeAuthorizeBody,
        data,
        globalState
      );
    });
  });

  context("[Payment] Complete Authorize - 3DS Auto Capture Flow", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    afterEach("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("[Payment] Create Payment Intent for 3DS Auto", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PaymentIntent"];

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );

      if (shouldContinue) {
        shouldContinue = utils.should_continue_further(data);
      }
    });

    it("[Payment] Confirm Payment Intent (3DS Auto)", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["3DSAutoCapture"];

      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

      if (shouldContinue) {
        shouldContinue = utils.should_continue_further(data);
      }
    });

    it("[Payment] Complete Authorize Call", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["CompleteAuthorize"];

      cy.completeAuthorizeCallTest(
        fixtures.completeAuthorizeBody,
        data,
        globalState
      );
    });
  });
});
