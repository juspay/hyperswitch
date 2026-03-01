import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

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

  context("No3DS Card - Psync after Customer Deletion (Automatic Capture)", () => {
    it("Create Customer + Create Payment Intent + Confirm Payment + Retrieve Payment + Delete Customer + Retrieve Payment (After Customer Deletion)", () => {
      cy.step("Create Customer", () =>
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState)
      );

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

      cy.step("Delete Customer", () => cy.customerDeleteCall(globalState));

      cy.step("Retrieve Payment (After Customer Deletion)", () =>
        cy.retrievePaymentCallTest({ globalState, data: confirmData })
      );
    });
  });

  context("3DS Card - Psync after Customer Deletion (Automatic Capture)", () => {
    it("Create Customer + Create Payment Intent + Confirm Payment + Handle 3DS Redirection + Retrieve Payment + Delete Customer + Retrieve Payment (After Customer Deletion)", () => {
      cy.step("Create Customer", () =>
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState)
      );

      const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "PaymentIntent"
      ];

      cy.step("Create Payment Intent", () =>
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "three_ds",
          "automatic",
          globalState
        )
      );

      if (!utils.should_continue_further(data)) return;

      const confirmData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["3DSAutoCapture"];

      cy.step("Confirm Payment", () =>
        cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState)
      );

      if (!utils.should_continue_further(confirmData)) return;

      const expected_redirection = fixtures.confirmBody["return_url"];
      cy.step("Handle 3DS Redirection", () =>
        cy.handleRedirection(globalState, expected_redirection)
      );

      const retrieveData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSAutoCapture"];

      cy.step("Retrieve Payment", () =>
        cy.retrievePaymentCallTest({ globalState, data: retrieveData })
      );

      cy.step("Delete Customer", () => cy.customerDeleteCall(globalState));

      cy.step("Retrieve Payment (After Customer Deletion)", () =>
        cy.retrievePaymentCallTest({ globalState, data: retrieveData })
      );
    });
  });

  context("No3DS Card - Psync after Customer Deletion (Manual Capture)", () => {
    it("Create Customer + Create Payment Intent + Confirm Payment + Retrieve Payment + Capture Payment + Retrieve Payment (After Capture) + Delete Customer + Retrieve Payment (After Customer Deletion)", () => {
      cy.step("Create Customer", () =>
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState)
      );

      const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "PaymentIntent"
      ];

      cy.step("Create Payment Intent", () =>
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "manual",
          globalState
        )
      );

      if (!utils.should_continue_further(data)) return;

      const confirmData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSManualCapture"];

      cy.step("Confirm Payment", () =>
        cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState)
      );

      if (!utils.should_continue_further(confirmData)) return;

      cy.step("Retrieve Payment", () =>
        cy.retrievePaymentCallTest({ globalState, data: confirmData })
      );

      const captureData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["Capture"];

      cy.step("Capture Payment", () =>
        cy.captureCallTest(fixtures.captureBody, captureData, globalState)
      );

      if (!utils.should_continue_further(captureData)) return;

      cy.step("Retrieve Payment (After Capture)", () =>
        cy.retrievePaymentCallTest({ globalState, data: captureData })
      );

      cy.step("Delete Customer", () => cy.customerDeleteCall(globalState));

      cy.step("Retrieve Payment (After Customer Deletion)", () =>
        cy.retrievePaymentCallTest({ globalState, data: confirmData })
      );
    });
  });

  context("3DS Card - Psync after Customer Deletion (Manual Capture)", () => {
    it("Create Customer + Create Payment Intent + Confirm Payment + Handle 3DS Redirection + Retrieve Payment + Capture Payment + Retrieve Payment (After Capture) + Delete Customer + Retrieve Payment (After Customer Deletion)", () => {
      cy.step("Create Customer", () =>
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState)
      );

      const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "PaymentIntent"
      ];

      cy.step("Create Payment Intent", () =>
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "three_ds",
          "manual",
          globalState
        )
      );

      if (!utils.should_continue_further(data)) return;

      const confirmData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["3DSManualCapture"];

      cy.step("Confirm Payment", () =>
        cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState)
      );

      if (!utils.should_continue_further(confirmData)) return;

      const expected_redirection = fixtures.confirmBody["return_url"];
      cy.step("Handle 3DS Redirection", () =>
        cy.handleRedirection(globalState, expected_redirection)
      );

      cy.step("Retrieve Payment", () =>
        cy.retrievePaymentCallTest({ globalState, data: confirmData })
      );

      const captureData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["Capture"];

      cy.step("Capture Payment", () =>
        cy.captureCallTest(fixtures.captureBody, captureData, globalState)
      );

      if (!utils.should_continue_further(captureData)) return;

      cy.step("Retrieve Payment (After Capture)", () =>
        cy.retrievePaymentCallTest({ globalState, data: captureData })
      );

      cy.step("Delete Customer", () => cy.customerDeleteCall(globalState));

      cy.step("Retrieve Payment (After Customer Deletion)", () =>
        cy.retrievePaymentCallTest({ globalState, data: confirmData })
      );
    });
  });
});
