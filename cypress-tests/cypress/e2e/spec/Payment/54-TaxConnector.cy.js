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

const NULL_CARD_METADATA = {
  card_type: null,
  card_network: null,
  card_issuer: null,
  card_issuing_country: null,
};

function withNullCardMetadata(data) {
  if (data?.Response?.body?.payment_method_data?.card) {
    Object.assign(
      data.Response.body.payment_method_data.card,
      NULL_CARD_METADATA
    );
  }
  return data;
}

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
        "taxProfile"
      );
    });

    it("enable-tax-connector-on-profile-test", () => {
      const merchantConnectorId = globalState.get("merchantConnectorId");
      cy.updateBusinessProfileWithTaxConnector(
        fixtures.businessProfile.bpUpdate,
        true,
        merchantConnectorId,
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
          const data = {
            ...getConnectorDetails(globalState.get("connectorId"))["card_pm"][
              "PaymentIntent"
            ],
            ...TAX_PROFILE_CONFIG,
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
          const data = withNullCardMetadata({
            ...getConnectorDetails(globalState.get("connectorId"))["card_pm"][
              "No3DSAutoCapture"
            ],
            ...TAX_PROFILE_CONFIG,
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
          const data = withNullCardMetadata({
            ...getConnectorDetails(globalState.get("connectorId"))["card_pm"][
              "No3DSAutoCapture"
            ],
            ...TAX_PROFILE_CONFIG,
          });

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
        globalState,
        "taxProfile"
      );
    });

    it("tax-disabled-create-confirm-retrieve-payment-test", () => {
      let shouldProceed = true;

      cy.step("Create Payment Intent", () => {
        const data = {
          ...getConnectorDetails(globalState.get("connectorId"))["card_pm"][
            "PaymentIntent"
          ],
          ...TAX_PROFILE_CONFIG,
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
        const data = withNullCardMetadata({
          ...getConnectorDetails(globalState.get("connectorId"))["card_pm"][
            "No3DSAutoCapture"
          ],
          ...TAX_PROFILE_CONFIG,
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
        const data = withNullCardMetadata({
          ...getConnectorDetails(globalState.get("connectorId"))["card_pm"][
            "No3DSAutoCapture"
          ],
          ...TAX_PROFILE_CONFIG,
        });

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
          globalState,
          "taxProfile"
        );
      });

      it("skip-tax-create-confirm-retrieve-payment-test", () => {
        let shouldProceed = true;

        cy.step("Create Payment Intent with skip flag", () => {
          const data = {
            ...getConnectorDetails(globalState.get("connectorId"))["card_pm"][
              "PaymentIntent"
            ],
            ...TAX_PROFILE_CONFIG,
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
          const data = withNullCardMetadata({
            ...getConnectorDetails(globalState.get("connectorId"))["card_pm"][
              "No3DSAutoCapture"
            ],
            ...TAX_PROFILE_CONFIG,
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
          const data = withNullCardMetadata({
            ...getConnectorDetails(globalState.get("connectorId"))["card_pm"][
              "No3DSAutoCapture"
            ],
            ...TAX_PROFILE_CONFIG,
          });

          cy.retrievePaymentCallTest({ globalState, data });
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
      cy.retrieveBusinessProfileTest(globalState, "taxProfile", false);
    });
  });

  context("Toggle tax flag - re-enable after disable", () => {
    it("re-enable-tax-after-disable-test", () => {
      const merchantConnectorId = globalState.get("merchantConnectorId");
      cy.updateBusinessProfileWithTaxConnector(
        fixtures.businessProfile.bpUpdate,
        true,
        merchantConnectorId,
        globalState,
        "taxProfile"
      );
    });

    it("toggle-tax-create-confirm-retrieve-payment-test", () => {
      let shouldProceed = true;

      cy.step("Create Payment Intent", () => {
        const data = {
          ...getConnectorDetails(globalState.get("connectorId"))["card_pm"][
            "PaymentIntent"
          ],
          ...TAX_PROFILE_CONFIG,
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
        const data = withNullCardMetadata({
          ...getConnectorDetails(globalState.get("connectorId"))["card_pm"][
            "No3DSAutoCapture"
          ],
          ...TAX_PROFILE_CONFIG,
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
        const data = withNullCardMetadata({
          ...getConnectorDetails(globalState.get("connectorId"))["card_pm"][
            "No3DSAutoCapture"
          ],
          ...TAX_PROFILE_CONFIG,
        });

        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });
});
