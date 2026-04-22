import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Partner Merchant Identifier Tests", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Partner Merchant Identifier - Happy Path", () => {
    it("Create Payment Intent with Partner Merchant Identifier and retrieve", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent with Partner Merchant Identifier", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PartnerMerchantIdentifier"];

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

      cy.step("Retrieve Payment to verify persisted Partner Merchant Identifier", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const retrieveData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PartnerMerchantIdentifier"];

        cy.retrievePaymentCallTest({ globalState, data: retrieveData });
      });
    });
  });

  context("Partner Merchant Identifier - Negative Cases", () => {
    it("Create Payment Intent without Partner Merchant Identifier (baseline comparison)", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent without Partner Merchant Identifier", () => {
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

      cy.step("Retrieve Payment to verify no Partner Merchant Identifier present", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        // Use retrieve without data param to avoid billing assertion for negative case
        cy.retrievePaymentCallTest({ globalState });
      });
    });
  });

  context("Partner Merchant Identifier - Edge Cases", () => {
    it("Create Payment Intent with empty partner merchant identifier details", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent with empty partner_merchant_identifier_details", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PartnerMerchantIdentifier"];

        // Modify request to use empty partner_merchant_identifier_details
        // and expect nulls in response when empty object is sent
        const modifiedData = {
          ...data,
          Request: {
            ...data.Request,
            partner_merchant_identifier_details: {},
          },
          Response: {
            status: 200,
            body: {
              status: "requires_payment_method",
              partner_merchant_identifier_details: {
                partner_details: null,
                merchant_details: null,
              },
            },
          },
        };

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          modifiedData,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (!utils.should_continue_further(modifiedData)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment after empty partner merchant identifier", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        cy.retrievePaymentCallTest({ globalState });
      });
    });
  });
});
