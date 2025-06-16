import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let connector;
let globalState;

describe.skip("[Payment] Incremental Auth", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      connector = globalState.get("connectorId");
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("[Payment] Incremental Pre-Auth", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue || connector !== "cybersource") {
        this.skip();
      }
    });

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

      cy.captureCallTest(fixtures.captureBody, data, globalState);

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
