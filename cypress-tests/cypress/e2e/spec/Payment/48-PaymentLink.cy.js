import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Payment Link - Hosted payment link generation", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Payment Link - Basic creation and retrieval", () => {
    it("Create Payment Intent with Payment Link -> Initiate Payment Link -> Retrieve Payment Link -> List Payment Links", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent with Payment Link", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentLink"];

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

      cy.step("Initiate Payment Link (Customer-Facing)", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Initiate Payment Link");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentLink"];

        cy.initiatePaymentLinkTest(data, globalState);
      });

      cy.step("Retrieve Payment Link (Merchant API)", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment Link");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentLink"];

        cy.retrievePaymentLinkTest(data, globalState);
      });

      cy.step("List Payment Links", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Payment Links");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentLink"];

        cy.listPaymentLinksTest(data, globalState);
      });
    });
  });

  context("Payment Link - With Metadata", () => {
    it("Create Payment Intent with Payment Link and metadata -> Initiate Payment Link", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent with Payment Link and metadata", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentLinkWithMetadata"];

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

      cy.step("Initiate Payment Link (Customer-Facing)", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Initiate Payment Link");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentLinkWithMetadata"];

        cy.initiatePaymentLinkTest(data, globalState);
      });
    });
  });

  context("Payment Link - Edge Cases", () => {
    it("Create Payment Intent without Payment Link -> Should not have payment_link in response", () => {
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

      cy.wrap(null).then(() => {
        expect(globalState.get("paymentLinkId")).to.be.undefined;
      });
    });
  });
});
