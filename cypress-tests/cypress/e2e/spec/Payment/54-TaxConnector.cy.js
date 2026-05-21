import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import { payment_methods_enabled } from "../../configs/Payment/Commons";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Tax Connector Business Profile Flag", () => {
  let shouldContinue = true;

  before("seed global state and check inclusion gate", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      if (
        !utils.CONNECTOR_LISTS.INCLUDE.TAX_CONNECTOR.includes(
          globalState.get("connectorId")
        )
      ) {
        shouldContinue = false;
      }
    });
  });

  beforeEach(function () {
    if (!shouldContinue) {
      this.skip();
    }
  });

  after("cleanup and flush global state", () => {
    if (shouldContinue && globalState) {
      cy.deleteBusinessProfileTest(globalState);
      cy.task("setGlobalState", globalState.data);
    }
  });

  context("Setup - Create business profile and payment connector", () => {
    it("create-business-profile-test", () => {
      cy.createBusinessProfileTest(
        fixtures.businessProfile.bpCreate,
        globalState
      );
    });

    it("create-payment-connector-test", () => {
      cy.createConnectorCallTest(
        "payment_processor",
        fixtures.createConnectorBody,
        payment_methods_enabled,
        globalState
      );
    });

    it("enable-tax-connector-on-profile-test", () => {
      const merchantConnectorId = globalState.get("merchantConnectorId");
      cy.updateBusinessProfileWithTaxConnector(
        fixtures.businessProfile.bpUpdate,
        true,
        merchantConnectorId,
        globalState
      );
    });
  });

  context(
    "Tax enabled - payment creates with tax calculation attempted",
    () => {
      it("tax-enabled-create-confirm-retrieve-payment-test", () => {
        let shouldContinue = true;

        cy.step("Create Payment Intent", () => {
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

        cy.step("Confirm Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm Payment");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["No3DSAutoCapture"];

          cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Retrieve Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Retrieve Payment");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["No3DSAutoCapture"];

          cy.retrievePaymentCallTest({ globalState, data });
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
        globalState
      );
    });

    it("tax-disabled-create-confirm-retrieve-payment-test", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
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

      cy.step("Confirm Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });

  context(
    "Skip external tax calculation - tax bypassed even when enabled",
    () => {
      it("re-enable-tax-connector-on-profile-test", () => {
        const merchantConnectorId = globalState.get("merchantConnectorId");
        cy.updateBusinessProfileWithTaxConnector(
          fixtures.businessProfile.bpUpdate,
          true,
          merchantConnectorId,
          globalState
        );
      });

      it("skip-tax-create-confirm-retrieve-payment-test", () => {
        let shouldContinue = true;

        cy.step("Create Payment Intent with skip flag", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentIntent"];

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
            shouldContinue = false;
          }
        });

        cy.step("Confirm Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm Payment");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["No3DSAutoCapture"];

          cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Retrieve Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Retrieve Payment");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["No3DSAutoCapture"];

          cy.retrievePaymentCallTest({ globalState, data });
        });
      });
    }
  );

  context("Retrieve business profile - tax fields verified", () => {
    it("retrieve-profile-tax-enabled-fields-test", () => {
      cy.retrieveBusinessProfileTest(globalState);
    });
  });

  context("Disable tax connector and verify profile fields", () => {
    it("disable-tax-connector-verify-profile-test", () => {
      cy.updateBusinessProfileWithTaxConnector(
        fixtures.businessProfile.bpUpdate,
        false,
        null,
        globalState
      );
    });

    it("retrieve-profile-tax-disabled-fields-test", () => {
      cy.retrieveBusinessProfileTest(globalState);
    });
  });

  context("Toggle tax flag - re-enable after disable", () => {
    it("re-enable-tax-after-disable-test", () => {
      const merchantConnectorId = globalState.get("merchantConnectorId");
      cy.updateBusinessProfileWithTaxConnector(
        fixtures.businessProfile.bpUpdate,
        true,
        merchantConnectorId,
        globalState
      );
    });

    it("toggle-tax-create-confirm-retrieve-payment-test", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
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

      cy.step("Confirm Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });
});
