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
        if (!shouldContinue) return;
        const payment_id = globalState.get("paymentID");
        const headers = {
          "Content-Type": "application/json",
          "api-key": globalState.get("apiKey"),
        };
        cy.request({
          method: "GET",
          url: `${globalState.get("baseUrl")}/payments/${payment_id}?force_sync=true`,
          headers,
          failOnStatusCode: false,
        }).then((response) => {
          expect(response.status).to.eq(200);
          expect(response.body.payment_id).to.eq(payment_id);
          expect(
            response.body.partner_merchant_identifier_details,
            "partner_merchant_identifier_details"
          ).to.deep.eq({
            partner_details: {
              name: "TestPartner",
              version: "1.0.0",
              integrator: "TestIntegrator123",
            },
            merchant_details: {
              name: "TestMerchantApp",
              version: "2.0.0",
            },
          });
        });
      });
    });
  });

  context("Partner Merchant Identifier - Negative Cases", () => {
    it("Create Payment Intent without Partner Merchant Identifier (baseline comparison)", () => {
      let shouldContinue = true;

      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PaymentIntent"];

      cy.step("Create Payment Intent without Partner Merchant Identifier", () => {
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
        if (!shouldContinue) return;
        const payment_id = globalState.get("paymentID");
        const headers = {
          "Content-Type": "application/json",
          "api-key": globalState.get("apiKey"),
        };
        cy.request({
          method: "GET",
          url: `${globalState.get("baseUrl")}/payments/${payment_id}?force_sync=true`,
          headers,
          failOnStatusCode: false,
        }).then((response) => {
          expect(response.status).to.eq(200);
          expect(response.body.payment_id).to.eq(payment_id);
          expect(
            response.body,
            "payment without PMI"
          ).to.not.have.property("partner_merchant_identifier_details");
        });
      });
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

      cy.step("Create Payment Intent with empty partner_merchant_identifier_details", () => {
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

      cy.step("Verify empty partner_merchant_identifier_details returns nulls", () => {
        if (!shouldContinue) return;
        const payment_id = globalState.get("paymentID");
        const headers = {
          "Content-Type": "application/json",
          "api-key": globalState.get("apiKey"),
        };
        cy.request({
          method: "GET",
          url: `${globalState.get("baseUrl")}/payments/${payment_id}?force_sync=true`,
          headers,
          failOnStatusCode: false,
        }).then((response) => {
          expect(response.status).to.eq(200);
          expect(response.body.payment_id).to.eq(payment_id);
          expect(
            response.body.partner_merchant_identifier_details,
            "partner_merchant_identifier_details"
          ).to.deep.eq({
            partner_details: null,
            merchant_details: null,
          });
        });
      });
    });
  });
});
