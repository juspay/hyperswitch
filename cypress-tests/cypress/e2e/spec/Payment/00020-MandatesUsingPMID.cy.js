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
      it("customer-create-call-test -> Create No 3DS Payment Intent -> Confirm No 3DS CIT -> retrieve-payment-call-test -> Confirm No 3DS MIT -> retrieve-payment-call-test", () => {
        let shouldContinue = true;

        cy.step("customer-create-call-test", () => {
          cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
        });

        cy.step("Create No 3DS Payment Intent", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Create No 3DS Payment Intent");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentIntentOffSession"];

          cy.createPaymentIntentTest(
            fixtures.createPaymentBody,
            data,
            "no_three_ds",
            "automatic",
            globalState
          );

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Confirm No 3DS CIT", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm No 3DS CIT");
            return;
          }
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

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("retrieve-payment-call-test", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: retrieve-payment-call-test");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentMethodIdMandateNo3DSAutoCapture"];

          cy.retrievePaymentCallTest({ globalState, data });

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Confirm No 3DS MIT", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm No 3DS MIT");
            return;
          }
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

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("retrieve-payment-call-test", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: retrieve-payment-call-test");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["MITAutoCapture"];

          cy.retrievePaymentCallTest({ globalState, data });
        });
      });
    }
  );

  context(
    "Card - NoThreeDS Create and Confirm Manual CIT and MIT payment flow test",
    () => {
      it("Create No 3DS Payment Intent -> Confirm No 3DS CIT -> cit-capture-call-test -> retrieve-payment-call-test -> Confirm No 3DS MIT -> retrieve-payment-call-test", () => {
        let shouldContinue = true;

        cy.step("Create No 3DS Payment Intent", () => {
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

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Confirm No 3DS CIT", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm No 3DS CIT");
            return;
          }
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

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("cit-capture-call-test", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: cit-capture-call-test");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["Capture"];

          cy.captureCallTest(fixtures.captureBody, data, globalState);

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("retrieve-payment-call-test", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: retrieve-payment-call-test");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["Capture"];

          cy.retrievePaymentCallTest({ globalState, data });

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Confirm No 3DS MIT", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm No 3DS MIT");
            return;
          }
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

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("retrieve-payment-call-test", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: retrieve-payment-call-test");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["MITAutoCapture"];

          cy.retrievePaymentCallTest({ globalState, data });
        });
      });
    }
  );

  context(
    "Card - NoThreeDS Create + Confirm Automatic CIT and MIT payment flow test",
    () => {
      it("Confirm No 3DS CIT -> retrieve-payment-call-test -> Confirm No 3DS MIT -> retrieve-payment-call-test -> Confirm No 3DS MIT -> retrieve-payment-call-test", () => {
        let shouldContinue = true;

        cy.step("Confirm No 3DS CIT", () => {
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

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("retrieve-payment-call-test", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: retrieve-payment-call-test");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentMethodIdMandateNo3DSAutoCapture"];

          cy.retrievePaymentCallTest({ globalState, data });

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Confirm No 3DS MIT", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm No 3DS MIT");
            return;
          }
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

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("retrieve-payment-call-test", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: retrieve-payment-call-test");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["MITAutoCapture"];

          cy.retrievePaymentCallTest({ globalState, data });

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Confirm No 3DS MIT", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm No 3DS MIT");
            return;
          }
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

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("retrieve-payment-call-test", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: retrieve-payment-call-test");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["MITAutoCapture"];

          cy.retrievePaymentCallTest({ globalState, data });
        });
      });
    }
  );

  context(
    "Card - NoThreeDS Create + Confirm Manual CIT and MIT payment flow test",
    () => {
      it("Confirm No 3DS CIT -> cit-capture-call-test -> retrieve-payment-call-test -> Confirm No 3DS MIT 1 -> mit-capture-call-test -> retrieve-payment-call-test -> Confirm No 3DS MIT 2 -> mit-capture-call-test -> retrieve-payment-call-test", () => {
        let shouldContinue = true;

        cy.step("Confirm No 3DS CIT", () => {
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

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("cit-capture-call-test", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: cit-capture-call-test");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["Capture"];

          cy.captureCallTest(fixtures.captureBody, data, globalState);

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("retrieve-payment-call-test", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: retrieve-payment-call-test");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["Capture"];

          cy.retrievePaymentCallTest({ globalState, data });

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Confirm No 3DS MIT 1", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm No 3DS MIT 1");
            return;
          }
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

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("mit-capture-call-test", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: mit-capture-call-test");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["Capture"];

          cy.captureCallTest(fixtures.captureBody, data, globalState);

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("retrieve-payment-call-test", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: retrieve-payment-call-test");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["Capture"];

          cy.retrievePaymentCallTest({ globalState, data });

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Confirm No 3DS MIT 2", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm No 3DS MIT 2");
            return;
          }
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

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("mit-capture-call-test", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: mit-capture-call-test");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["Capture"];

          cy.captureCallTest(fixtures.captureBody, data, globalState);

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("retrieve-payment-call-test", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: retrieve-payment-call-test");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["Capture"];

          cy.retrievePaymentCallTest({ globalState, data });
        });
      });
    }
  );

  context("Card - MIT without billing address", () => {
    it("Create No 3DS Payment Intent -> Confirm No 3DS CIT -> Confirm No 3DS MIT", () => {
      let shouldContinue = true;

      cy.step("Create No 3DS Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntentOffSession"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Confirm No 3DS CIT", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm No 3DS CIT");
          return;
        }
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

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Confirm No 3DS MIT", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm No 3DS MIT");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["MITWithoutBillingAddress"];

        cy.mitUsingPMId(
          fixtures.pmIdConfirmBody,
          data,
          6000,
          true,
          "automatic",
          globalState
        );
      });
    });
  });

  context(
    "Card - ThreeDS Create + Confirm Automatic CIT and MIT payment flow test",
    () => {
      it("Confirm 3DS CIT -> Handle redirection -> retrieve-payment-call-test -> Confirm No 3DS MIT -> Confirm No 3DS MIT", () => {
        let shouldContinue = true;

        cy.step("Confirm 3DS CIT", () => {
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

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Handle redirection", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Handle redirection");
            return;
          }
          const expected_redirection = fixtures.citConfirmBody["return_url"];
          cy.handleRedirection(globalState, expected_redirection);
        });

        cy.step("retrieve-payment-call-test", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: retrieve-payment-call-test");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentMethodIdMandate3DSAutoCapture"];

          cy.retrievePaymentCallTest({ globalState, data });

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Confirm No 3DS MIT", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm No 3DS MIT");
            return;
          }
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

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Confirm No 3DS MIT", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm No 3DS MIT");
            return;
          }
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
      });
    }
  );

  context(
    "Card - ThreeDS Create + Confirm Manual CIT and MIT payment flow",
    () => {
      it("Confirm 3DS CIT -> Handle redirection -> cit-capture-call-test -> retrieve-payment-call-test -> Confirm No 3DS MIT", () => {
        let shouldContinue = true;

        cy.step("Confirm 3DS CIT", () => {
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

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Handle redirection", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Handle redirection");
            return;
          }
          const expected_redirection = fixtures.citConfirmBody["return_url"];
          cy.handleRedirection(globalState, expected_redirection);
        });

        cy.step("cit-capture-call-test", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: cit-capture-call-test");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["Capture"];

          cy.captureCallTest(fixtures.captureBody, data, globalState);

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("retrieve-payment-call-test", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: retrieve-payment-call-test");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["Capture"];

          cy.retrievePaymentCallTest({ globalState, data });

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Confirm No 3DS MIT", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm No 3DS MIT");
            return;
          }
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
      });
    }
  );
});
