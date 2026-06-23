import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, {
  CONNECTOR_LISTS,
  shouldIncludeConnector,
} from "../../configs/Payment/Utils";
import * as utils from "../../configs/Payment/Utils";

let globalState;
let connector;

describe("Partner Merchant Identifier Tests", () => {
  before("seed global state", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        connector = globalState.get("connectorId");

        if (
          shouldIncludeConnector(
            connector,
            CONNECTOR_LISTS.INCLUDE.PARTNER_MERCHANT_IDENTIFIER
          )
        ) {
          skip = true;
          return;
        }
      })
      .then(() => {
        if (skip) {
          this.skip();
        }
      });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Partner Merchant Identifier - Happy Path", () => {
    it("Create Payment Intent -> Confirm Payment -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent with Partner Merchant Identifier", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PartnerMerchantIdentifier"];

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

      cy.step("Confirm Payment with Partner Merchant Identifier", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment with PMI");
          return;
        }

        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PartnerMerchantIdentifierConfirm"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

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
              "Skipping step: Retrieve Payment to verify persisted PMI"
            );
            return;
          }

          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PartnerMerchantIdentifier"];

          cy.retrievePaymentCallTest({ globalState, data });
        }
      );
    });
  });

  context("Partner Merchant Identifier - Negative Cases", () => {
    it("Create Payment Intent -> Confirm Payment -> Retrieve Payment (without PMI)", () => {
      let shouldContinue = true;

      cy.step(
        "Create Payment Intent without Partner Merchant Identifier",
        () => {
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

      cy.step("Confirm Payment without Partner Merchant Identifier", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment without PMI");
          return;
        }

        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PartnerMerchantIdentifierConfirm"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
      });

      cy.step(
        "Retrieve Payment to verify no Partner Merchant Identifier present",
        () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Retrieve Payment to verify no PMI present"
            );
            return;
          }

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

          cy.retrievePaymentCallTest({ globalState, data });
        }
      );
    });
  });

  context("Partner Merchant Identifier - Edge Cases", () => {
    it("Create Payment Intent -> Confirm Payment -> Retrieve Payment (with empty PMI)", () => {
      let shouldContinue = true;

      cy.step(
        "Create Payment Intent with empty partner merchant identifier details",
        () => {
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
        "Confirm Payment with empty partner merchant identifier details",
        () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm Payment with empty PMI");
            return;
          }

          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PartnerMerchantIdentifierConfirm"];

          cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
        }
      );

      cy.step(
        "Verify empty partner_merchant_identifier_details returns nulls",
        () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Verify empty PMI returns nulls");
            return;
          }

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

          cy.retrievePaymentCallTest({ globalState, data: modifiedData });
        }
      );
    });
  });
});
