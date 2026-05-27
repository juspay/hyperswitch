import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import { payment_methods_enabled } from "../../configs/Payment/Commons";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

const TAX_PROFILE_CONFIG = {
  Configs: {
    CONNECTOR_CREDENTIAL: {
      value: "connector_3",
    },
  },
};

describe("Tax Connector Business Profile Flag", () => {
  let connectorSupported = true;

  before("seed global state and check inclusion gate", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      if (
        utils.shouldIncludeConnector(
          globalState.get("connectorId"),
          utils.CONNECTOR_LISTS.INCLUDE.TAX_CONNECTOR
        )
      ) {
        connectorSupported = false;
      }
    });
  });

  beforeEach(function () {
    if (!connectorSupported) {
      this.skip();
    }
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  after("cleanup and flush global state", () => {
    cy.task("setGlobalState", globalState.data);
    if (connectorSupported) {
      cy.deleteBusinessProfileTest(globalState, "taxProfile");
    }
  });

  context("Setup - Create business profile and payment connector", () => {
    it("create-business-profile-test", () => {
      cy.createBusinessProfileTest(
        fixtures.businessProfile.bpCreate,
        globalState,
        "taxProfile"
      );
    });

    it("create-payment-connector-test", () => {
      cy.createConnectorCallTest(
        "payment_processor",
        fixtures.createConnectorBody,
        payment_methods_enabled,
        globalState,
        "taxProfile",
        "taxConnector"
      );
    });

    it("enable-tax-connector-on-profile-test", () => {
      // taxConnectorId references the payment connector created in the previous step
      // (mcPrefix="taxConnector" → globalState key "taxConnectorId")
      const taxConnectorId = globalState.get("taxConnectorId");
      cy.updateBusinessProfileWithTaxConnector(
        fixtures.businessProfile.bpUpdate,
        true,
        taxConnectorId,
        globalState,
        "taxProfile"
      );
    });
  });

  context(
    "Tax enabled - payment creates with tax calculation attempted",
    () => {
      it("tax-enabled-create-confirm-retrieve-payment-test", () => {
        let shouldProceed = true;

        cy.step("Create Payment Intent", () => {
          const baseData = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentIntent"];
          const data = {
            ...baseData,
            Configs: { ...baseData.Configs, ...TAX_PROFILE_CONFIG.Configs },
          };

          cy.createPaymentIntentTest(
            fixtures.createPaymentBody,
            data,
            "no_three_ds",
            "automatic",
            globalState
          );

          if (!utils.should_continue_further(data)) {
            shouldProceed = false;
          }
        });

        cy.step("Confirm Payment", () => {
          if (!shouldProceed) {
            cy.task("cli_log", "Skipping step: Confirm Payment");
            return;
          }
          const baseData = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["No3DSAutoCapture"];
          const data = utils.withNullCardMetadata({
            ...baseData,
            Configs: { ...baseData.Configs, ...TAX_PROFILE_CONFIG.Configs },
          });

          cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

          if (!utils.should_continue_further(data)) {
            shouldProceed = false;
          }
        });

        cy.step("Retrieve Payment", () => {
          if (!shouldProceed) {
            cy.task("cli_log", "Skipping step: Retrieve Payment");
            return;
          }
          const baseData = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["No3DSAutoCapture"];
          const data = utils.withNullCardMetadata({
            ...baseData,
            Configs: { ...baseData.Configs, ...TAX_PROFILE_CONFIG.Configs },
          });

          cy.retrievePaymentCallTest({ globalState, data });

          // Tax-specific assertions: when tax connector is enabled,
          // tax_details and order_tax_amount should be present on the payment.
          // Note: In sandbox, tax_details may be null if TaxJar does not
          // calculate tax for the given address/currency — in that case this
          // assertion will surface the gap.
          cy.request({
            method: "GET",
            url: `${globalState.get("baseUrl")}/payments/${globalState.get("paymentID")}?force_sync=true&expand_attempts=true`,
            headers: {
              "Content-Type": "application/json",
              "api-key": globalState.get("apiKey"),
            },
            failOnStatusCode: false,
          }).then((taxResponse) => {
            if (taxResponse.status === 200) {
              expect(
                taxResponse.body.tax_details,
                "tax_details should be present when tax connector enabled"
              ).to.not.be.null;
              expect(
                taxResponse.body.order_tax_amount,
                "order_tax_amount should be present when tax connector enabled"
              ).to.not.be.null;
            }
          });
        });
      });
    }
  );

  context("Tax disabled - payment creates without tax calculation", () => {
    it("disable-tax-connector-on-profile-test", () => {
      cy.updateBusinessProfileWithTaxConnector(
        fixtures.businessProfile.bpUpdate,
        false,
        null,
        globalState,
        "taxProfile"
      );
    });

    it("tax-disabled-create-confirm-retrieve-payment-test", () => {
      let shouldProceed = true;

      cy.step("Create Payment Intent", () => {
        const baseData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];
        const data = {
          ...baseData,
          Configs: { ...baseData.Configs, ...TAX_PROFILE_CONFIG.Configs },
        };

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldProceed = false;
        }
      });

      cy.step("Confirm Payment", () => {
        if (!shouldProceed) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const baseData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];
        const data = utils.withNullCardMetadata({
          ...baseData,
          Configs: { ...baseData.Configs, ...TAX_PROFILE_CONFIG.Configs },
        });

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

        if (!utils.should_continue_further(data)) {
          shouldProceed = false;
        }
      });

      cy.step("Retrieve Payment", () => {
        if (!shouldProceed) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const baseData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];
        const data = utils.withNullCardMetadata({
          ...baseData,
          Configs: { ...baseData.Configs, ...TAX_PROFILE_CONFIG.Configs },
        });

        cy.retrievePaymentCallTest({ globalState, data });

        // Tax-specific assertions: when tax connector is disabled,
        // tax_details and order_tax_amount should be null.
        cy.request({
          method: "GET",
          url: `${globalState.get("baseUrl")}/payments/${globalState.get("paymentID")}?force_sync=true&expand_attempts=true`,
          headers: {
            "Content-Type": "application/json",
            "api-key": globalState.get("apiKey"),
          },
          failOnStatusCode: false,
        }).then((taxResponse) => {
          if (taxResponse.status === 200) {
            expect(
              taxResponse.body.tax_details,
              "tax_details should be null when tax connector disabled"
            ).to.be.null;
            expect(
              taxResponse.body.order_tax_amount,
              "order_tax_amount should be null when tax connector disabled"
            ).to.be.null;
          }
        });
      });
    });
  });

  context(
    "Skip external tax calculation - tax bypassed even when enabled",
    () => {
      it("re-enable-tax-connector-on-profile-test", () => {
        const taxConnectorId = globalState.get("taxConnectorId");
        cy.updateBusinessProfileWithTaxConnector(
          fixtures.businessProfile.bpUpdate,
          true,
          taxConnectorId,
          globalState,
          "taxProfile"
        );
      });

      it("skip-tax-create-confirm-retrieve-payment-test", () => {
        let shouldProceed = true;

        cy.step("Create Payment Intent with skip flag", () => {
          const baseData = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentIntent"];
          const data = {
            ...baseData,
            Configs: { ...baseData.Configs, ...TAX_PROFILE_CONFIG.Configs },
          };

          const paymentBody = { ...fixtures.createPaymentBody };
          paymentBody.skip_external_tax_calculation = true;

          cy.createPaymentIntentTest(
            paymentBody,
            data,
            "no_three_ds",
            "automatic",
            globalState
          );

          if (!utils.should_continue_further(data)) {
            shouldProceed = false;
          }
        });

        cy.step("Confirm Payment", () => {
          if (!shouldProceed) {
            cy.task("cli_log", "Skipping step: Confirm Payment");
            return;
          }
          const baseData = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["No3DSAutoCapture"];
          const data = utils.withNullCardMetadata({
            ...baseData,
            Configs: { ...baseData.Configs, ...TAX_PROFILE_CONFIG.Configs },
          });

          cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

          if (!utils.should_continue_further(data)) {
            shouldProceed = false;
          }
        });

        cy.step("Retrieve Payment", () => {
          if (!shouldProceed) {
            cy.task("cli_log", "Skipping step: Retrieve Payment");
            return;
          }
          const baseData = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["No3DSAutoCapture"];
          const data = utils.withNullCardMetadata({
            ...baseData,
            Configs: { ...baseData.Configs, ...TAX_PROFILE_CONFIG.Configs },
          });

          cy.retrievePaymentCallTest({ globalState, data });

          // Tax-specific assertions: when skip_external_tax_calculation is set,
          // tax_details and order_tax_amount should be null even though the
          // tax connector is enabled on the profile.
          cy.request({
            method: "GET",
            url: `${globalState.get("baseUrl")}/payments/${globalState.get("paymentID")}?force_sync=true&expand_attempts=true`,
            headers: {
              "Content-Type": "application/json",
              "api-key": globalState.get("apiKey"),
            },
            failOnStatusCode: false,
          }).then((taxResponse) => {
            if (taxResponse.status === 200) {
              expect(
                taxResponse.body.tax_details,
                "tax_details should be null when skip_external_tax_calculation is set"
              ).to.be.null;
              expect(
                taxResponse.body.order_tax_amount,
                "order_tax_amount should be null when skip_external_tax_calculation is set"
              ).to.be.null;
            }
          });
        });
      });
    }
  );

  context("Retrieve business profile - tax fields verified", () => {
    it("retrieve-profile-tax-enabled-fields-test", () => {
      cy.retrieveBusinessProfileTest(globalState, "taxProfile", true);
    });
  });

  context("Disable tax connector and verify profile fields", () => {
    it("disable-tax-connector-verify-profile-test", () => {
      cy.updateBusinessProfileWithTaxConnector(
        fixtures.businessProfile.bpUpdate,
        false,
        null,
        globalState,
        "taxProfile"
      );
    });

    it("retrieve-profile-tax-disabled-fields-test", () => {
      cy.retrieveBusinessProfileTest(
        globalState,
        "taxProfile",
        false,
        globalState.get("taxConnectorId")
      );
    });
  });

  context("Toggle tax flag - re-enable after disable", () => {
    it("re-enable-tax-after-disable-test", () => {
      const taxConnectorId = globalState.get("taxConnectorId");
      cy.updateBusinessProfileWithTaxConnector(
        fixtures.businessProfile.bpUpdate,
        true,
        taxConnectorId,
        globalState,
        "taxProfile"
      );
    });

    it("toggle-tax-create-confirm-retrieve-payment-test", () => {
      let shouldProceed = true;

      cy.step("Create Payment Intent", () => {
        const baseData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];
        const data = {
          ...baseData,
          Configs: { ...baseData.Configs, ...TAX_PROFILE_CONFIG.Configs },
        };

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldProceed = false;
        }
      });

      cy.step("Confirm Payment", () => {
        if (!shouldProceed) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const baseData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];
        const data = utils.withNullCardMetadata({
          ...baseData,
          Configs: { ...baseData.Configs, ...TAX_PROFILE_CONFIG.Configs },
        });

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

        if (!utils.should_continue_further(data)) {
          shouldProceed = false;
        }
      });

      cy.step("Retrieve Payment", () => {
        if (!shouldProceed) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const baseData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];
        const data = utils.withNullCardMetadata({
          ...baseData,
          Configs: { ...baseData.Configs, ...TAX_PROFILE_CONFIG.Configs },
        });

        cy.retrievePaymentCallTest({ globalState, data });

        // Tax-specific assertions: after re-enabling tax connector,
        // tax_details and order_tax_amount should be present again.
        // Note: Same sandbox caveat as the tax-enabled context above.
        cy.request({
          method: "GET",
          url: `${globalState.get("baseUrl")}/payments/${globalState.get("paymentID")}?force_sync=true&expand_attempts=true`,
          headers: {
            "Content-Type": "application/json",
            "api-key": globalState.get("apiKey"),
          },
          failOnStatusCode: false,
        }).then((taxResponse) => {
          if (taxResponse.status === 200) {
            expect(
              taxResponse.body.tax_details,
              "tax_details should be present after re-enabling tax connector"
            ).to.not.be.null;
            expect(
              taxResponse.body.order_tax_amount,
              "order_tax_amount should be present after re-enabling tax connector"
            ).to.not.be.null;
          }
        });
      });
    });
  });
});
