import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";
import step from "../../../utils/customStep";

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
      it("Create Customer -> Create Payment Intent -> CIT - Create Mandate (Auto Capture) -> Retrieve CIT Payment -> MIT - Auto Capture using PM Id -> Retrieve MIT Payment", () => {
        let shouldContinue = true;

        step("Create Customer", shouldContinue, () => {
          cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
        });

        step("Create Payment Intent", shouldContinue, () => {
          const paymentIntentData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["PaymentIntentOffSession"];
          cy.createPaymentIntentTest(
            fixtures.createPaymentBody,
            paymentIntentData,
            "no_three_ds",
            "automatic",
            globalState
          );
          if (!utils.should_continue_further(paymentIntentData)) {
            shouldContinue = false;
          }
        });

        step("CIT - Create Mandate (Auto Capture)", shouldContinue, () => {
          const citData = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentMethodIdMandateNo3DSAutoCapture"];
          cy.citForMandatesCallTest(
            fixtures.citConfirmBody,
            citData,
            6000,
            true,
            "automatic",
            "new_mandate",
            globalState
          );
          if (!utils.should_continue_further(citData)) {
            shouldContinue = false;
          }
        });

        step("Retrieve CIT Payment", shouldContinue, () => {
          const citData = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentMethodIdMandateNo3DSAutoCapture"];
          cy.retrievePaymentCallTest({ globalState, data: citData });
          if (!utils.should_continue_further(citData)) {
            shouldContinue = false;
          }
        });

        step("MIT - Auto Capture using PM Id", shouldContinue, () => {
          const mitData = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["MITAutoCapture"];
          cy.mitUsingPMId(
            fixtures.pmIdConfirmBody,
            mitData,
            6000,
            true,
            "automatic",
            globalState
          );
          if (!utils.should_continue_further(mitData)) {
            shouldContinue = false;
          }
        });

        step("Retrieve MIT Payment", shouldContinue, () => {
          const mitData = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["MITAutoCapture"];
          cy.retrievePaymentCallTest({ globalState, data: mitData });
        });
      });
    }
  );

  context(
    "Card - NoThreeDS Create and Confirm Manual CIT and MIT payment flow test",
    () => {
      it("Create Payment Intent -> CIT - Create Mandate (Manual Capture) -> Capture CIT Payment -> Retrieve CIT Payment -> MIT - Auto Capture using PM Id -> Retrieve MIT Payment", () => {
        let shouldContinue = true;

        step("Create Payment Intent", shouldContinue, () => {
          const paymentIntentData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["PaymentIntentOffSession"];
          cy.createPaymentIntentTest(
            fixtures.createPaymentBody,
            paymentIntentData,
            "no_three_ds",
            "manual",
            globalState
          );
          if (!utils.should_continue_further(paymentIntentData)) {
            shouldContinue = false;
          }
        });

        step("CIT - Create Mandate (Manual Capture)", shouldContinue, () => {
          const citData = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentMethodIdMandateNo3DSManualCapture"];
          cy.citForMandatesCallTest(
            fixtures.citConfirmBody,
            citData,
            6000,
            true,
            "manual",
            "new_mandate",
            globalState
          );
          if (!utils.should_continue_further(citData)) {
            shouldContinue = false;
          }
        });

        step("Capture CIT Payment", shouldContinue, () => {
          const captureData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["Capture"];
          cy.captureCallTest(fixtures.captureBody, captureData, globalState);
          if (!utils.should_continue_further(captureData)) {
            shouldContinue = false;
          }
        });

        step("Retrieve CIT Payment", shouldContinue, () => {
          const captureData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["Capture"];
          cy.retrievePaymentCallTest({ globalState, data: captureData });
          if (!utils.should_continue_further(captureData)) {
            shouldContinue = false;
          }
        });

        step("MIT - Auto Capture using PM Id", shouldContinue, () => {
          const mitData = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["MITAutoCapture"];
          cy.mitUsingPMId(
            fixtures.pmIdConfirmBody,
            mitData,
            6000,
            true,
            "automatic",
            globalState
          );
          if (!utils.should_continue_further(mitData)) {
            shouldContinue = false;
          }
        });

        step("Retrieve MIT Payment", shouldContinue, () => {
          const mitData = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["MITAutoCapture"];
          cy.retrievePaymentCallTest({ globalState, data: mitData });
        });
      });
    }
  );

  context(
    "Card - NoThreeDS Create + Confirm Automatic CIT and MIT payment flow test",
    () => {
      it("CIT - Create Mandate (Auto Capture) -> Retrieve CIT Payment -> MIT 1 - Auto Capture using PM Id -> Retrieve MIT 1 Payment -> MIT 2 - Auto Capture using PM Id -> Retrieve MIT 2 Payment", () => {
        let shouldContinue = true;

        step("CIT - Create Mandate (Auto Capture)", shouldContinue, () => {
          const citData = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentMethodIdMandateNo3DSAutoCapture"];
          cy.citForMandatesCallTest(
            fixtures.citConfirmBody,
            citData,
            6000,
            true,
            "automatic",
            "new_mandate",
            globalState
          );
          if (!utils.should_continue_further(citData)) {
            shouldContinue = false;
          }
        });

        step("Retrieve CIT Payment", shouldContinue, () => {
          const citData = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentMethodIdMandateNo3DSAutoCapture"];
          cy.retrievePaymentCallTest({ globalState, data: citData });
          if (!utils.should_continue_further(citData)) {
            shouldContinue = false;
          }
        });

        step("MIT 1 - Auto Capture using PM Id", shouldContinue, () => {
          const mitData = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["MITAutoCapture"];
          cy.mitUsingPMId(
            fixtures.pmIdConfirmBody,
            mitData,
            6000,
            true,
            "automatic",
            globalState
          );
          if (!utils.should_continue_further(mitData)) {
            shouldContinue = false;
          }
        });

        step("Retrieve MIT 1 Payment", shouldContinue, () => {
          const mitData = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["MITAutoCapture"];
          cy.retrievePaymentCallTest({ globalState, data: mitData });
          if (!utils.should_continue_further(mitData)) {
            shouldContinue = false;
          }
        });

        step("MIT 2 - Auto Capture using PM Id", shouldContinue, () => {
          const mitData = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["MITAutoCapture"];
          cy.mitUsingPMId(
            fixtures.pmIdConfirmBody,
            mitData,
            6000,
            true,
            "automatic",
            globalState
          );
          if (!utils.should_continue_further(mitData)) {
            shouldContinue = false;
          }
        });

        step("Retrieve MIT 2 Payment", shouldContinue, () => {
          const mitData = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["MITAutoCapture"];
          cy.retrievePaymentCallTest({ globalState, data: mitData });
        });
      });
    }
  );

  context(
    "Card - NoThreeDS Create + Confirm Manual CIT and MIT payment flow test",
    () => {
      it("CIT - Create Mandate (Manual Capture) -> Capture CIT Payment -> Retrieve CIT Payment -> MIT 1 - Manual Capture using PM Id -> Capture MIT 1 Payment -> Retrieve MIT 1 Payment -> MIT 2 - Manual Capture using PM Id -> Capture MIT 2 Payment -> Retrieve MIT 2 Payment", () => {
        let shouldContinue = true;

        step("CIT - Create Mandate (Manual Capture)", shouldContinue, () => {
          const citData = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentMethodIdMandateNo3DSManualCapture"];
          cy.citForMandatesCallTest(
            fixtures.citConfirmBody,
            citData,
            6000,
            true,
            "manual",
            "new_mandate",
            globalState
          );
          if (!utils.should_continue_further(citData)) {
            shouldContinue = false;
          }
        });

        step("Capture CIT Payment", shouldContinue, () => {
          const captureData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["Capture"];
          cy.captureCallTest(fixtures.captureBody, captureData, globalState);
          if (!utils.should_continue_further(captureData)) {
            shouldContinue = false;
          }
        });

        step("Retrieve CIT Payment", shouldContinue, () => {
          const captureData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["Capture"];
          cy.retrievePaymentCallTest({ globalState, data: captureData });
          if (!utils.should_continue_further(captureData)) {
            shouldContinue = false;
          }
        });

        step("MIT 1 - Manual Capture using PM Id", shouldContinue, () => {
          const mitManualData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["MITManualCapture"];
          cy.mitUsingPMId(
            fixtures.pmIdConfirmBody,
            mitManualData,
            6000,
            true,
            "manual",
            globalState
          );
          if (!utils.should_continue_further(mitManualData)) {
            shouldContinue = false;
          }
        });

        step("Capture MIT 1 Payment", shouldContinue, () => {
          const captureData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["Capture"];
          cy.captureCallTest(fixtures.captureBody, captureData, globalState);
          if (!utils.should_continue_further(captureData)) {
            shouldContinue = false;
          }
        });

        step("Retrieve MIT 1 Payment", shouldContinue, () => {
          const captureData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["Capture"];
          cy.retrievePaymentCallTest({ globalState, data: captureData });
          if (!utils.should_continue_further(captureData)) {
            shouldContinue = false;
          }
        });

        step("MIT 2 - Manual Capture using PM Id", shouldContinue, () => {
          const mitManualData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["MITManualCapture"];
          cy.mitUsingPMId(
            fixtures.pmIdConfirmBody,
            mitManualData,
            6000,
            true,
            "manual",
            globalState
          );
          if (!utils.should_continue_further(mitManualData)) {
            shouldContinue = false;
          }
        });

        step("Capture MIT 2 Payment", shouldContinue, () => {
          const captureData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["Capture"];
          cy.captureCallTest(fixtures.captureBody, captureData, globalState);
          if (!utils.should_continue_further(captureData)) {
            shouldContinue = false;
          }
        });

        step("Retrieve MIT 2 Payment", shouldContinue, () => {
          const captureData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["Capture"];
          cy.retrievePaymentCallTest({ globalState, data: captureData });
        });
      });
    }
  );

  context("Card - MIT without billing address", () => {
    it("Create Payment Intent -> CIT - Create Mandate (Auto Capture) -> MIT - Auto Capture without Billing Address", () => {
      let shouldContinue = true;

      step("Create Payment Intent", shouldContinue, () => {
        const paymentIntentData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["PaymentIntentOffSession"];
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          paymentIntentData,
          "no_three_ds",
          "automatic",
          globalState
        );
        if (!utils.should_continue_further(paymentIntentData)) {
          shouldContinue = false;
        }
      });

      step("CIT - Create Mandate (Auto Capture)", shouldContinue, () => {
        const citData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentMethodIdMandateNo3DSAutoCapture"];
        cy.citForMandatesCallTest(
          fixtures.citConfirmBody,
          citData,
          6000,
          true,
          "automatic",
          "new_mandate",
          globalState
        );
        if (!utils.should_continue_further(citData)) {
          shouldContinue = false;
        }
      });

      step("MIT - Auto Capture without Billing Address", shouldContinue, () => {
        const mitData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["MITWithoutBillingAddress"];
        cy.mitUsingPMId(
          fixtures.pmIdConfirmBody,
          mitData,
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
      it("CIT - Create 3DS Mandate (Auto Capture) -> Handle 3DS Redirection -> Retrieve CIT Payment -> MIT 1 - Auto Capture using PM Id -> MIT 2 - Auto Capture using PM Id", () => {
        let shouldContinue = true;

        step("CIT - Create 3DS Mandate (Auto Capture)", shouldContinue, () => {
          const citData = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentMethodIdMandate3DSAutoCapture"];
          cy.citForMandatesCallTest(
            fixtures.citConfirmBody,
            citData,
            6000,
            true,
            "automatic",
            "new_mandate",
            globalState
          );
          if (!utils.should_continue_further(citData)) {
            shouldContinue = false;
          }
        });

        step("Handle 3DS Redirection", shouldContinue, () => {
          const expected_redirection = fixtures.citConfirmBody["return_url"];
          cy.handleRedirection(globalState, expected_redirection);
        });

        step("Retrieve CIT Payment", shouldContinue, () => {
          const citData = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentMethodIdMandate3DSAutoCapture"];
          cy.retrievePaymentCallTest({ globalState, data: citData });
          if (!utils.should_continue_further(citData)) {
            shouldContinue = false;
          }
        });

        step("MIT 1 - Auto Capture using PM Id", shouldContinue, () => {
          const mitData = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["MITAutoCapture"];
          cy.mitUsingPMId(
            fixtures.pmIdConfirmBody,
            mitData,
            6000,
            true,
            "automatic",
            globalState
          );
          if (!utils.should_continue_further(mitData)) {
            shouldContinue = false;
          }
        });

        step("MIT 2 - Auto Capture using PM Id", shouldContinue, () => {
          const mitData = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["MITAutoCapture"];
          cy.mitUsingPMId(
            fixtures.pmIdConfirmBody,
            mitData,
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
      it("CIT - Create 3DS Mandate (Manual Capture) -> Handle 3DS Redirection -> Capture CIT Payment -> Retrieve CIT Payment -> MIT - Auto Capture using PM Id", () => {
        let shouldContinue = true;

        step("CIT - Create 3DS Mandate (Manual Capture)", shouldContinue, () => {
          const citData = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentMethodIdMandate3DSManualCapture"];
          cy.citForMandatesCallTest(
            fixtures.citConfirmBody,
            citData,
            6000,
            true,
            "manual",
            "new_mandate",
            globalState
          );
          if (!utils.should_continue_further(citData)) {
            shouldContinue = false;
          }
        });

        step("Handle 3DS Redirection", shouldContinue, () => {
          const expected_redirection = fixtures.citConfirmBody["return_url"];
          cy.handleRedirection(globalState, expected_redirection);
        });

        step("Capture CIT Payment", shouldContinue, () => {
          const captureData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["Capture"];
          cy.captureCallTest(fixtures.captureBody, captureData, globalState);
          if (!utils.should_continue_further(captureData)) {
            shouldContinue = false;
          }
        });

        step("Retrieve CIT Payment", shouldContinue, () => {
          const captureData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["Capture"];
          cy.retrievePaymentCallTest({ globalState, data: captureData });
          if (!utils.should_continue_further(captureData)) {
            shouldContinue = false;
          }
        });

        step("MIT - Auto Capture using PM Id", shouldContinue, () => {
          const mitData = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["MITAutoCapture"];
          cy.mitUsingPMId(
            fixtures.pmIdConfirmBody,
            mitData,
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
