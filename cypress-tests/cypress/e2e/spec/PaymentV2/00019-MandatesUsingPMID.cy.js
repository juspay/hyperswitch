import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Card - Mandates using Payment Method Id flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context(
    "Card - NoThreeDS Create and Confirm Automatic CIT and MIT payment flow test",
    () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("Create No 3DS Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("Confirm No 3DS CIT", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentMethodIdMandateNo3DSAutoCapture"];

        cy.citForMandatesCallTest(
          fixtures.citConfirmBody,
          data,
          6000,
          true,
          "automatic",
          "new_mandate",
          globalState
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("Confirm No 3DS MIT", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["MITAutoCapture"];

        cy.mitUsingPMId(
          fixtures.pmIdConfirmBody,
          data,
          6000,
          true,
          "automatic",
          globalState
        );
      });
    }
  );

  context(
    "Card - NoThreeDS Create and Confirm Manual CIT and MIT payment flow test",
    () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("Create No 3DS Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "manual",
          globalState
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("Confirm No 3DS CIT", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentMethodIdMandateNo3DSManualCapture"];

        cy.citForMandatesCallTest(
          fixtures.citConfirmBody,
          data,
          6000,
          true,
          "manual",
          "new_mandate",
          globalState
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("cit-capture-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];

        cy.captureCallTest(fixtures.captureBody, data, globalState);

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("Confirm No 3DS MIT", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["MITAutoCapture"];

        cy.mitUsingPMId(
          fixtures.pmIdConfirmBody,
          data,
          6000,
          true,
          "automatic",
          globalState
        );
      });
    }
  );

  context(
    "Card - NoThreeDS Create + Confirm Automatic CIT and MIT payment flow test",
    () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("Confirm No 3DS CIT", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentMethodIdMandateNo3DSAutoCapture"];

        cy.citForMandatesCallTest(
          fixtures.citConfirmBody,
          data,
          6000,
          true,
          "automatic",
          "new_mandate",
          globalState
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("Confirm No 3DS MIT", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["MITAutoCapture"];

        cy.mitUsingPMId(
          fixtures.pmIdConfirmBody,
          data,
          6000,
          true,
          "automatic",
          globalState
        );
      });
      it("Confirm No 3DS MIT", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["MITAutoCapture"];

        cy.mitUsingPMId(
          fixtures.pmIdConfirmBody,
          data,
          6000,
          true,
          "automatic",
          globalState
        );
      });
    }
  );

  context(
    "Card - NoThreeDS Create + Confirm Manual CIT and MIT payment flow test",
    () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("Confirm No 3DS CIT", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentMethodIdMandateNo3DSManualCapture"];

        cy.citForMandatesCallTest(
          fixtures.citConfirmBody,
          data,
          6000,
          true,
          "manual",
          "new_mandate",
          globalState
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("cit-capture-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];

        cy.captureCallTest(fixtures.captureBody, data, globalState);

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("Confirm No 3DS MIT 1", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["MITManualCapture"];

        cy.mitUsingPMId(
          fixtures.pmIdConfirmBody,
          data,
          6000,
          true,
          "manual",
          globalState
        );
      });

      it("mit-capture-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];

        cy.captureCallTest(fixtures.captureBody, data, globalState);

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("Confirm No 3DS MIT 2", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["MITManualCapture"];

        cy.mitUsingPMId(
          fixtures.pmIdConfirmBody,
          data,
          6000,
          true,
          "manual",
          globalState
        );
      });

      it("mit-capture-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];

        cy.captureCallTest(fixtures.captureBody, data, globalState);

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });
    }
  );

  context(
    "Card - ThreeDS Create + Confirm Automatic CIT and MIT payment flow test",
    () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("Confirm 3DS CIT", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentMethodIdMandate3DSAutoCapture"];

        cy.citForMandatesCallTest(
          fixtures.citConfirmBody,
          data,
          6000,
          true,
          "automatic",
          "new_mandate",
          globalState
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("Handle redirection", () => {
        const expected_redirection = fixtures.citConfirmBody["return_url"];
        cy.handleRedirection(globalState, expected_redirection);
      });

      it("Confirm No 3DS MIT", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["MITAutoCapture"];

        cy.mitUsingPMId(
          fixtures.pmIdConfirmBody,
          data,
          6000,
          true,
          "automatic",
          globalState
        );
      });
      it("Confirm No 3DS MIT", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["MITAutoCapture"];

        cy.mitUsingPMId(
          fixtures.pmIdConfirmBody,
          data,
          6000,
          true,
          "automatic",
          globalState
        );
      });
    }
  );

  context(
    "Card - ThreeDS Create + Confirm Manual CIT and MIT payment flow",
    () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("Confirm 3DS CIT", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentMethodIdMandate3DSManualCapture"];

        cy.citForMandatesCallTest(
          fixtures.citConfirmBody,
          data,
          6000,
          true,
          "manual",
          "new_mandate",
          globalState
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("Handle redirection", () => {
        const expected_redirection = fixtures.citConfirmBody["return_url"];
        cy.handleRedirection(globalState, expected_redirection);
      });

      it("cit-capture-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];

        cy.captureCallTest(fixtures.captureBody, data, globalState);

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("Confirm No 3DS MIT", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["MITAutoCapture"];

        cy.mitUsingPMId(
          fixtures.pmIdConfirmBody,
          data,
          6000,
          true,
          "automatic",
          globalState
        );
      });
    }
  );
});
