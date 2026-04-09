import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let connector;
let globalState;

describe("[Payment] Incremental Auth", () => {
  before(function () {
    // Changed to regular function instead of arrow function
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        connector = globalState.get("connectorId");

        // Skip the test if the connector is not in the inclusion list
        // This is done because only cybersource is known to support at present
        if (
          utils.shouldIncludeConnector(
            connector,
            utils.CONNECTOR_LISTS.INCLUDE.INCREMENTAL_AUTH
          )
        ) {
          skip = true;
        }
      })
      .then(() => {
        if (skip) {
          this.skip();
        }
      });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("[Payment] Incremental Pre-Auth", () => {
    let shouldContinue = true;

    it("[Payment] Create Payment Intent", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PaymentIntentOffSession"];

      const newData = {
        ...data,
        Request: {
          ...data.Request,
          request_incremental_authorization: true,
        },
      };

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        newData,
        "no_three_ds",
        "manual",
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
    it("[Payment] Confirm Payment Intent", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["SaveCardUseNo3DSManualCaptureOffSession"];

      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
    it("[Payment] Incremental Authorization", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["IncrementalAuth"];
      cy.incrementalAuth(globalState, data);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
    it("[Payment] Capture Payment Intent", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["Capture"];

      const newData = {
        ...data,
        Request: { amount_to_capture: data.Request.amount_to_capture + 2000 },
        Response: data.ResponseCustom || data.Response,
      };

      cy.captureCallTest(fixtures.captureBody, newData, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
  });

  context("[Payment] [Saved Card] Incremental Pre-Auth", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue || connector !== "cybersource") {
        this.skip();
      }
    });

    it("[Payment] List customer payment methods", () => {
      cy.listCustomerPMCallTest(globalState);
    });
    it("[Payment] Create Payment Intent", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PaymentIntentOffSession"];

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "manual",
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
    it("[Payment] Confirm Payment Intent", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["SaveCardUseNo3DSManualCaptureOffSession"];

      cy.saveCardConfirmCallTest(
        fixtures.saveCardConfirmBody,
        data,
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
    it("[Payment] Incremental Authorization", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["IncrementalAuth"];

      cy.incrementalAuth(globalState, data);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
    it("[Payment] Capture Payment Intent", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["Capture"];

      cy.captureCallTest(fixtures.captureBody, data, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
  });
});
