import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import { payment_methods_enabled } from "../../configs/Payment/Commons";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Merchant Country Code Tests", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context(
    "Create Business Profile with merchant_country_code",
    () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("Create Business Profile with merchant_country_code US", () => {
        cy.createBusinessProfileTest(
          fixtures.businessProfile.bpCreate,
          globalState
        );
      });

      it("connector-create-call-test", () => {
        cy.createConnectorCallTest(
          "payment_processor",
          fixtures.createConnectorBody,
          payment_methods_enabled,
          globalState
        );
      });

      it("Create Customer", () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      it("Update business profile with merchant_country_code US", () => {
        cy.UpdateBusinessProfileTest(
          fixtures.businessProfile.bpUpdate,
          true, // is_connector_agnostic_mit_enabled
          false, // collect_billing_details_from_wallet_connector
          false, // collect_shipping_details_from_wallet_connector
          false, // always_collect_billing_details_from_wallet_connector
          false, // always_collect_shipping_details_from_wallet_connector
          "US", // merchant_country_code
          globalState
        );
      });

      it("Verify merchant_country_code is stored in globalState", () => {
        const storedCountryCode = globalState.get("merchantCountryCode");
        expect(storedCountryCode).to.equal("US");
      });

      it("Create and confirm payment with merchant_country_code US", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

        cy.createConfirmPaymentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });
    }
  );

  context(
    "Update Business Profile with different merchant_country_code",
    () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("Create Business Profile without merchant_country_code", () => {
        cy.createBusinessProfileTest(
          fixtures.businessProfile.bpCreate,
          globalState
        );
      });

      it("connector-create-call-test", () => {
        cy.createConnectorCallTest(
          "payment_processor",
          fixtures.createConnectorBody,
          payment_methods_enabled,
          globalState
        );
      });

      it("Create Customer", () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      it("Update business profile with merchant_country_code GB", () => {
        cy.UpdateBusinessProfileTest(
          fixtures.businessProfile.bpUpdate,
          true, // is_connector_agnostic_mit_enabled
          false, // collect_billing_details_from_wallet_connector
          false, // collect_shipping_details_from_wallet_connector
          false, // always_collect_billing_details_from_wallet_connector
          false, // always_collect_shipping_details_from_wallet_connector
          "GB", // merchant_country_code
          globalState
        );
      });

      it("Verify merchant_country_code GB is stored correctly", () => {
        const storedCountryCode = globalState.get("merchantCountryCode");
        expect(storedCountryCode).to.equal("GB");
      });

      it("Create and confirm payment with merchant_country_code GB", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

        cy.createConfirmPaymentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });
    }
  );

  context(
    "3DS Payment Flow with merchant_country_code",
    () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("Create Business Profile with merchant_country_code DE", () => {
        cy.createBusinessProfileTest(
          fixtures.businessProfile.bpCreate,
          globalState
        );
      });

      it("connector-create-call-test", () => {
        cy.createConnectorCallTest(
          "payment_processor",
          fixtures.createConnectorBody,
          payment_methods_enabled,
          globalState
        );
      });

      it("Create Customer", () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      it("Update business profile with merchant_country_code DE", () => {
        cy.UpdateBusinessProfileTest(
          fixtures.businessProfile.bpUpdate,
          true, // is_connector_agnostic_mit_enabled
          false, // collect_billing_details_from_wallet_connector
          false, // collect_shipping_details_from_wallet_connector
          false, // always_collect_billing_details_from_wallet_connector
          false, // always_collect_shipping_details_from_wallet_connector
          "DE", // merchant_country_code
          globalState
        );
      });

      it("Verify merchant_country_code DE is stored correctly", () => {
        const storedCountryCode = globalState.get("merchantCountryCode");
        expect(storedCountryCode).to.equal("DE");
      });

      it("Create and confirm 3DS payment with merchant_country_code DE", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["3DSAutoCapture"];

        cy.createConfirmPaymentTest(
          fixtures.createPaymentBody,
          data,
          "three_ds",
          "automatic",
          globalState
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });
    }
  );

  context(
    "Edge case - Update merchant_country_code on existing profile",
    () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("Create Business Profile", () => {
        cy.createBusinessProfileTest(
          fixtures.businessProfile.bpCreate,
          globalState
        );
      });

      it("connector-create-call-test", () => {
        cy.createConnectorCallTest(
          "payment_processor",
          fixtures.createConnectorBody,
          payment_methods_enabled,
          globalState
        );
      });

      it("Create Customer", () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      it("Initial update with merchant_country_code FR", () => {
        cy.UpdateBusinessProfileTest(
          fixtures.businessProfile.bpUpdate,
          true, // is_connector_agnostic_mit_enabled
          false, // collect_billing_details_from_wallet_connector
          false, // collect_shipping_details_from_wallet_connector
          false, // always_collect_billing_details_from_wallet_connector
          false, // always_collect_shipping_details_from_wallet_connector
          "FR", // merchant_country_code
          globalState
        );
      });

      it("Verify initial merchant_country_code FR", () => {
        const storedCountryCode = globalState.get("merchantCountryCode");
        expect(storedCountryCode).to.equal("FR");
      });

      it("Update to different merchant_country_code IN", () => {
        cy.UpdateBusinessProfileTest(
          fixtures.businessProfile.bpUpdate,
          true, // is_connector_agnostic_mit_enabled
          false, // collect_billing_details_from_wallet_connector
          false, // collect_shipping_details_from_wallet_connector
          false, // always_collect_billing_details_from_wallet_connector
          false, // always_collect_shipping_details_from_wallet_connector
          "IN", // merchant_country_code
          globalState
        );
      });

      it("Verify updated merchant_country_code IN", () => {
        const storedCountryCode = globalState.get("merchantCountryCode");
        expect(storedCountryCode).to.equal("IN");
      });

      it("Create and confirm payment after update", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

        cy.createConfirmPaymentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });
    }
  );

  context(
    "Edge case - merchant_country_code SG with manual capture",
    () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("Create Business Profile with merchant_country_code SG", () => {
        cy.createBusinessProfileTest(
          fixtures.businessProfile.bpCreate,
          globalState
        );
      });

      it("connector-create-call-test", () => {
        cy.createConnectorCallTest(
          "payment_processor",
          fixtures.createConnectorBody,
          payment_methods_enabled,
          globalState
        );
      });

      it("Create Customer", () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      it("Update business profile with merchant_country_code SG", () => {
        cy.UpdateBusinessProfileTest(
          fixtures.businessProfile.bpUpdate,
          true, // is_connector_agnostic_mit_enabled
          false, // collect_billing_details_from_wallet_connector
          false, // collect_shipping_details_from_wallet_connector
          false, // always_collect_billing_details_from_wallet_connector
          false, // always_collect_shipping_details_from_wallet_connector
          "SG", // merchant_country_code
          globalState
        );
      });

      it("Verify merchant_country_code SG is stored correctly", () => {
        const storedCountryCode = globalState.get("merchantCountryCode");
        expect(storedCountryCode).to.equal("SG");
      });

      it("Create and confirm manual capture payment", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSManualCapture"];

        cy.createConfirmPaymentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "manual",
          globalState
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("Capture the payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Capture payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];
        cy.captureCallTest(fixtures.captureBody, data, globalState);
      });
    }
  );
});
