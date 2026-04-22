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

      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PartnerMerchantIdentifier"];

      cy.step("Create Payment Intent with Partner Merchant Identifier", () => {
        const requestBody = { ...fixtures.createPaymentBody };
        cy.createPaymentIntentTest(
          requestBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step(
        "Retrieve Payment to verify persisted Partner Merchant Identifier",
        () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Retrieve Payment to verify persisted Partner Merchant Identifier"
            );
            return;
          }
          cy.retrievePaymentCallTest({ globalState, data });
        }
      );
    });
  });

  context("Partner Merchant Identifier - Negative Cases", () => {
    it("Create Payment Intent without Partner Merchant Identifier (baseline comparison)", () => {
      let shouldContinue = true;

      const baseData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PaymentIntent"];

      const data = {
        ...baseData,
        Request: {
          ...baseData.Request,
          billing: {
            address: {
              line1: "1467",
              line2: "Harrison Street",
              line3: "Harrison Street",
              city: "San Francisco",
              state: "California",
              zip: "94122",
              country: "US",
              first_name: "joseph",
              last_name: "Doe",
            },
          },
        },
      };

      cy.step(
        "Create Payment Intent without Partner Merchant Identifier",
        () => {
          const requestBody = { ...fixtures.createPaymentBody };
          cy.createPaymentIntentTest(
            requestBody,
            data,
            "no_three_ds",
            "automatic",
            globalState
          );

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        }
      );

      cy.step(
        "Retrieve Payment to verify no Partner Merchant Identifier present",
        () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Retrieve Payment to verify no Partner Merchant Identifier present"
            );
            return;
          }
          cy.retrievePaymentCallTest({ globalState, data });
        }
      );
    });
  });

  context("Partner Merchant Identifier - Edge Cases", () => {
    it("Create Payment Intent with empty partner merchant identifier details", () => {
      let shouldContinue = true;

      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PartnerMerchantIdentifier"];

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
          },
        },
      };

      cy.step(
        "Create Payment Intent with empty partner_merchant_identifier_details",
        () => {
          const requestBody = { ...fixtures.createPaymentBody };
          cy.createPaymentIntentTest(
            requestBody,
            modifiedData,
            "no_three_ds",
            "automatic",
            globalState
          );

          if (!utils.should_continue_further(modifiedData)) {
            shouldContinue = false;
          }
        }
      );

      cy.step(
        "Verify empty partner_merchant_identifier_details returns nulls",
        () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Verify empty partner_merchant_identifier_details returns nulls"
            );
            return;
          }
          cy.retrievePaymentCallTest({ globalState, data: modifiedData });
        }
      );
    });
  });
});
