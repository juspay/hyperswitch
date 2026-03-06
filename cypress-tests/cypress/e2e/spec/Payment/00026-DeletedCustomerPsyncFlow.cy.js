import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";
import step from "../../../utils/customStep";

let globalState;

describe("Card - Customer Deletion and Psync", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context(
    "No3DS Card - Psync after Customer Deletion (Automatic Capture)",
    () => {
      it("Create Customer -> Create Payment Intent -> Confirm Payment -> Retrieve Payment -> Delete Customer -> Retrieve Payment (After Customer Deletion)", () => {
        let shouldContinue = true;

        step("Create Customer", shouldContinue, () => {
          cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
        });

        step("Create Payment Intent", shouldContinue, () => {
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
          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        step("Confirm Payment", shouldContinue, () => {
          const confirmData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["No3DSAutoCapture"];
          cy.confirmCallTest(
            fixtures.confirmBody,
            confirmData,
            true,
            globalState
          );
          if (!utils.should_continue_further(confirmData)) {
            shouldContinue = false;
          }
        });

        step("Retrieve Payment", shouldContinue, () => {
          const confirmData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["No3DSAutoCapture"];
          cy.retrievePaymentCallTest({ globalState, data: confirmData });
          if (!utils.should_continue_further(confirmData)) {
            shouldContinue = false;
          }
        });

        step("Delete Customer", shouldContinue, () => {
          cy.customerDeleteCall(globalState);
        });

        step(
          "Retrieve Payment (After Customer Deletion)",
          shouldContinue,
          () => {
            const confirmData = getConnectorDetails(
              globalState.get("connectorId")
            )["card_pm"]["No3DSAutoCapture"];
            cy.retrievePaymentCallTest({ globalState, data: confirmData });
          }
        );
      });
    }
  );

  context(
    "3DS Card - Psync after Customer Deletion (Automatic Capture)",
    () => {
      it("Create Customer -> Create Payment Intent -> Confirm Payment -> Handle 3DS Redirection -> Retrieve Payment -> Delete Customer -> Retrieve Payment (After Customer Deletion)", () => {
        let shouldContinue = true;

        step("Create Customer", shouldContinue, () => {
          cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
        });

        step("Create Payment Intent", shouldContinue, () => {
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
          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        step("Confirm Payment", shouldContinue, () => {
          const confirmData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["3DSAutoCapture"];
          cy.confirmCallTest(
            fixtures.confirmBody,
            confirmData,
            true,
            globalState
          );
          if (!utils.should_continue_further(confirmData)) {
            shouldContinue = false;
          }
        });

        step("Handle 3DS Redirection", shouldContinue, () => {
          const expected_redirection = fixtures.confirmBody["return_url"];
          cy.handleRedirection(globalState, expected_redirection);
        });

        step("Retrieve Payment", shouldContinue, () => {
          const retrieveData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["No3DSAutoCapture"];
          cy.retrievePaymentCallTest({ globalState, data: retrieveData });
          if (!utils.should_continue_further(retrieveData)) {
            shouldContinue = false;
          }
        });

        step("Delete Customer", shouldContinue, () => {
          cy.customerDeleteCall(globalState);
        });

        step(
          "Retrieve Payment (After Customer Deletion)",
          shouldContinue,
          () => {
            const retrieveData = getConnectorDetails(
              globalState.get("connectorId")
            )["card_pm"]["No3DSAutoCapture"];
            cy.retrievePaymentCallTest({ globalState, data: retrieveData });
          }
        );
      });
    }
  );

  context("No3DS Card - Psync after Customer Deletion (Manual Capture)", () => {
    it("Create Customer -> Create Payment Intent -> Confirm Payment -> Retrieve Payment -> Capture Payment -> Retrieve Payment (After Capture) -> Delete Customer -> Retrieve Payment (After Customer Deletion)", () => {
      let shouldContinue = true;

      step("Create Customer", shouldContinue, () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      step("Create Payment Intent", shouldContinue, () => {
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
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      step("Confirm Payment", shouldContinue, () => {
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSManualCapture"];
        cy.confirmCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      step("Retrieve Payment", shouldContinue, () => {
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSManualCapture"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      step("Capture Payment", shouldContinue, () => {
        const captureData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];
        cy.captureCallTest(fixtures.captureBody, captureData, globalState);
        if (!utils.should_continue_further(captureData)) {
          shouldContinue = false;
        }
      });

      step("Retrieve Payment (After Capture)", shouldContinue, () => {
        const captureData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];
        cy.retrievePaymentCallTest({ globalState, data: captureData });
        if (!utils.should_continue_further(captureData)) {
          shouldContinue = false;
        }
      });

      step("Delete Customer", shouldContinue, () => {
        cy.customerDeleteCall(globalState);
      });

      step("Retrieve Payment (After Customer Deletion)", shouldContinue, () => {
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSManualCapture"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
      });
    });
  });

  context("3DS Card - Psync after Customer Deletion (Manual Capture)", () => {
    it("Create Customer -> Create Payment Intent -> Confirm Payment -> Handle 3DS Redirection -> Retrieve Payment -> Capture Payment -> Retrieve Payment (After Capture) -> Delete Customer -> Retrieve Payment (After Customer Deletion)", () => {
      let shouldContinue = true;

      step("Create Customer", shouldContinue, () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      step("Create Payment Intent", shouldContinue, () => {
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
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      step("Confirm Payment", shouldContinue, () => {
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["3DSManualCapture"];
        cy.confirmCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      step("Handle 3DS Redirection", shouldContinue, () => {
        const expected_redirection = fixtures.confirmBody["return_url"];
        cy.handleRedirection(globalState, expected_redirection);
      });

      step("Retrieve Payment", shouldContinue, () => {
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["3DSManualCapture"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      step("Capture Payment", shouldContinue, () => {
        const captureData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];
        cy.captureCallTest(fixtures.captureBody, captureData, globalState);
        if (!utils.should_continue_further(captureData)) {
          shouldContinue = false;
        }
      });

      step("Retrieve Payment (After Capture)", shouldContinue, () => {
        const captureData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];
        cy.retrievePaymentCallTest({ globalState, data: captureData });
        if (!utils.should_continue_further(captureData)) {
          shouldContinue = false;
        }
      });

      step("Delete Customer", shouldContinue, () => {
        cy.customerDeleteCall(globalState);
      });

      step("Retrieve Payment (After Customer Deletion)", shouldContinue, () => {
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["3DSManualCapture"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
      });
    });
  });
});
