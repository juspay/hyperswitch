import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import { payment_methods_enabled } from "../../configs/Payment/Commons";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;
let shouldRun = false;

describe("Tax Connector flow test", () => {
  before("seed global state and check tax connector credentials", function () {
    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        return cy.readFile(globalState.get("connectorAuthFilePath"));
      })
      .then((content) => {
        if (content && content.taxjar) {
          shouldRun = true;
        } else {
          cy.task(
            "cli_log",
            "TaxJar credentials not found in creds.json — skipping Tax Connector spec"
          );
        }
      });
  });

  beforeEach(function () {
    if (!shouldRun) {
      this.skip();
    }
  });

  after("cleanup and flush state", () => {
    if (shouldRun && globalState) {
      cy.deleteBusinessProfileTest(globalState);
      cy.task("setGlobalState", globalState.data);
    }
  });

  context(
    "Setup - Create profile, connectors, and calculate tax with US shipping",
    () => {
      it("setup-and-calculate-tax-us-test", () => {
        let shouldContinue = true;

        cy.step("Create Business Profile", () => {
          cy.createBusinessProfileTest(
            fixtures.businessProfile.bpCreate,
            globalState
          );
        });

        cy.step("Create Payment Processor Connector", () => {
          cy.createNamedConnectorCallTest(
            "payment_processor",
            fixtures.createConnectorBody,
            payment_methods_enabled,
            globalState,
            "stripe",
            "stripe_payment"
          );
        });

        cy.step("Create Tax Connector", () => {
          cy.createConnectorCallTest(
            "tax_calculation_provider",
            fixtures.createConnectorBody,
            [],
            globalState,
            "profile",
            "taxConnector"
          );
        });

        cy.step("Enable Tax Connector on Profile", () => {
          const taxConnectorId = globalState.get("taxConnectorId");
          cy.updateBusinessProfileWithTaxConnector(
            fixtures.businessProfile.bpUpdate,
            true,
            taxConnectorId,
            globalState
          );
        });

        cy.step("Create Payment Intent", () => {
          const data =
            getConnectorDetails("taxjar")["card_pm"]["PaymentIntent"];
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
          const data =
            getConnectorDetails("taxjar")["card_pm"]["No3DSAutoCapture"];
          cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Calculate Tax with US shipping address", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Calculate Tax with US shipping");
            return;
          }
          const data =
            getConnectorDetails("taxjar")["tax_connector"]["CalculateTax"];
          cy.calculateTaxCallTest(data, globalState);
        });
      });
    }
  );

  context("Tax enabled - calculate tax with EU shipping and debit PMT", () => {
    it("calculate-tax-eu-shipping-test", () => {
      let shouldContinue = true;

      cy.step("Ensure Tax Connector is enabled", () => {
        const taxConnectorId = globalState.get("taxConnectorId");
        cy.updateBusinessProfileWithTaxConnector(
          fixtures.businessProfile.bpUpdate,
          true,
          taxConnectorId,
          globalState
        );
      });

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails("taxjar")["card_pm"]["PaymentIntent"];
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
        const data =
          getConnectorDetails("taxjar")["card_pm"]["No3DSAutoCapture"];
        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Calculate Tax with EU shipping address and debit PMT", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Calculate Tax with EU shipping");
          return;
        }
        const data =
          getConnectorDetails("taxjar")["tax_connector"]["CalculateTaxEU"];
        cy.calculateTaxCallTest(data, globalState);
      });
    });
  });

  context("Tax enabled - skip_external_tax_calculation bypasses tax", () => {
    it("calculate-tax-with-skip-flag-test", () => {
      let shouldContinue = true;

      cy.step("Ensure Tax Connector is enabled", () => {
        const taxConnectorId = globalState.get("taxConnectorId");
        cy.updateBusinessProfileWithTaxConnector(
          fixtures.businessProfile.bpUpdate,
          true,
          taxConnectorId,
          globalState
        );
      });

      cy.step(
        "Create Payment Intent with skip_external_tax_calculation",
        () => {
          const data =
            getConnectorDetails("taxjar")["card_pm"]["PaymentIntent"];
          const createPaymentBody = { ...fixtures.createPaymentBody };
          createPaymentBody.skip_external_tax_calculation = "skip";
          cy.createPaymentIntentTest(
            createPaymentBody,
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

      cy.step("Confirm Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const data =
          getConnectorDetails("taxjar")["card_pm"]["No3DSAutoCapture"];
        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Calculate Tax should return order_tax_amount null", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Calculate Tax with skip flag");
          return;
        }
        const data =
          getConnectorDetails("taxjar")["tax_connector"]["CalculateTaxSkip"];
        cy.calculateTaxCallTest(data, globalState);
      });
    });
  });

  context("Tax connector disabled on profile", () => {
    it("tax-disabled-calculate-tax-fails-test", () => {
      let shouldContinue = true;

      cy.step("Disable Tax Connector on Profile", () => {
        cy.updateBusinessProfileWithTaxConnector(
          fixtures.businessProfile.bpUpdate,
          false,
          null,
          globalState
        );
      });

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails("taxjar")["card_pm"]["PaymentIntent"];
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
        const data =
          getConnectorDetails("taxjar")["card_pm"]["No3DSAutoCapture"];
        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Calculate Tax should fail when disabled", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Calculate Tax disabled");
          return;
        }
        const data =
          getConnectorDetails("taxjar")["tax_connector"][
            "CalculateTaxDisabled"
          ];
        cy.calculateTaxCallTest(data, globalState);
      });
    });
  });

  context("Edge case - calculate tax on succeeded payment", () => {
    it("calculate-tax-on-succeeded-payment-ir16-test", () => {
      let shouldContinue = true;

      cy.step("Re-enable Tax Connector on Profile", () => {
        const taxConnectorId = globalState.get("taxConnectorId");
        cy.updateBusinessProfileWithTaxConnector(
          fixtures.businessProfile.bpUpdate,
          true,
          taxConnectorId,
          globalState
        );
      });

      cy.step("Create and Confirm Payment", () => {
        const data =
          getConnectorDetails("taxjar")["card_pm"]["No3DSAutoCapture"];
        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
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
        "Calculate Tax on succeeded payment should fail with IR_16",
        () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Calculate Tax on succeeded payment"
            );
            return;
          }
          const data =
            getConnectorDetails("taxjar")["tax_connector"][
              "CalculateTaxSucceededPayment"
            ];
          cy.calculateTaxCallTest(data, globalState);
        }
      );
    });
  });

  context("Edge case - calculate tax with wrong auth", () => {
    it("calculate-tax-with-merchant-api-key-ir01-test", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails("taxjar")["card_pm"]["PaymentIntent"];
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
        const data =
          getConnectorDetails("taxjar")["card_pm"]["No3DSAutoCapture"];
        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step(
        "Calculate Tax with merchant API key should fail with IR_01",
        () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Calculate Tax wrong auth");
            return;
          }
          const data =
            getConnectorDetails("taxjar")["tax_connector"][
              "CalculateTaxWrongAuth"
            ];
          cy.calculateTaxCallTest(data, globalState, true);
        }
      );
    });
  });

  context("Edge case - missing client_secret", () => {
    it("calculate-tax-missing-client-secret-ir04-test", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails("taxjar")["card_pm"]["PaymentIntent"];
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
        const data =
          getConnectorDetails("taxjar")["card_pm"]["No3DSAutoCapture"];
        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step(
        "Calculate Tax without client_secret should fail with IR_04",
        () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Calculate Tax missing client_secret"
            );
            return;
          }
          const data =
            getConnectorDetails("taxjar")["tax_connector"][
              "CalculateTaxMissingClientSecret"
            ];
          cy.calculateTaxCallTest(data, globalState, false, true);
        }
      );
    });
  });

  context("Edge case - calculate tax on unconfirmed payment", () => {
    it("calculate-tax-on-unconfirmed-payment-ir39-test", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent (not confirmed)", () => {
        const data = getConnectorDetails("taxjar")["card_pm"]["PaymentIntent"];
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

      cy.step(
        "Calculate Tax on unconfirmed payment should fail with IR_39",
        () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Calculate Tax on unconfirmed");
            return;
          }
          const data =
            getConnectorDetails("taxjar")["tax_connector"][
              "CalculateTaxUnconfirmedPayment"
            ];
          cy.calculateTaxCallTest(data, globalState);
        }
      );
    });
  });
});
