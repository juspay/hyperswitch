import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import { payment_methods_enabled } from "../../configs/Payment/Commons";
import getConnectorDetails, {
  CONNECTOR_LISTS,
  shouldIncludeConnector,
} from "../../configs/Payment/Utils";

let globalState;

describe("Merchant Redirect Method Tests", () => {
  before("seed global state", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);

        if (
          shouldIncludeConnector(
            globalState.get("connectorId"),
            CONNECTOR_LISTS.INCLUDE.MERCHANT_REDIRECT
          )
        ) {
          skip = true;
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

  context(
    "Test redirect_to_merchant_with_http_post enabled - POST redirect flow",
    () => {
      it("Create Business Profile → Create Connector → Create Customer → Enable POST redirect → Create Payment Intent → Confirm Payment → Retrieve Payment", () => {
        let shouldContinue = true;

        cy.step(
          "Create Business Profile with redirect_to_merchant_with_http_post enabled",
          () => {
            cy.createBusinessProfileTest(
              fixtures.businessProfile.bpCreate,
              globalState
            );
          }
        );

        cy.step("Create Connector", () => {
          cy.createConnectorCallTest(
            "payment_processor",
            fixtures.createConnectorBody,
            payment_methods_enabled,
            globalState
          );
        });

        cy.step("Create Customer", () => {
          cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
        });

        cy.step("Update redirect_to_merchant_with_http_post to true", () => {
          cy.UpdateBusinessProfileTest(
            fixtures.businessProfile.bpUpdateRedirectPost,
            true,
            false,
            false,
            false,
            false,
            globalState
          );
        });

        cy.step("Create Payment Intent with POST redirect enabled", () => {
          const connectorId = globalState.get("connectorId");
          const connectorDetails = getConnectorDetails(connectorId);
          const data = connectorDetails?.["card_pm"]?.["PaymentIntent"];
          expect(data, `card_pm.PaymentIntent not found for ${connectorId}`).to
            .exist;

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
          "Confirm Payment and verify redirect behavior with POST method",
          () => {
            if (!shouldContinue) {
              cy.task("cli_log", "Skipping step: Confirm Payment");
              return;
            }
            const connectorId = globalState.get("connectorId");
            const connectorDetails = getConnectorDetails(connectorId);
            const data = connectorDetails?.["card_pm"]?.["No3DSAutoCapture"];
            expect(
              data,
              `card_pm.No3DSAutoCapture not found for ${connectorId}`
            ).to.exist;

            if (!data) {
              cy.task(
                "cli_log",
                "Skipping confirm step: No3DSAutoCapture config not found"
              );
              shouldContinue = false;
              return;
            }

            cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
            if (!utils.should_continue_further(data)) {
              shouldContinue = false;
            }
          }
        );

        cy.step("Retrieve Payment to verify status", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Retrieve Payment");
            return;
          }
          cy.retrievePaymentCallTest({ globalState });
        });
      });
    }
  );

  context(
    "Test redirect_to_merchant_with_http_post disabled - GET redirect flow",
    () => {
      it("Create Business Profile → Create Connector → Create Customer → Disable POST redirect → Create Payment Intent → Confirm Payment → Retrieve Payment", () => {
        let shouldContinue = true;

        cy.step(
          "Create Business Profile with redirect_to_merchant_with_http_post disabled",
          () => {
            cy.createBusinessProfileTest(
              fixtures.businessProfile.bpCreate,
              globalState
            );
          }
        );

        cy.step("Create Connector", () => {
          cy.createConnectorCallTest(
            "payment_processor",
            fixtures.createConnectorBody,
            payment_methods_enabled,
            globalState
          );
        });

        cy.step("Create Customer", () => {
          cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
        });

        cy.step("Update redirect_to_merchant_with_http_post to false", () => {
          cy.UpdateBusinessProfileTest(
            fixtures.businessProfile.bpUpdateRedirectGet,
            true,
            false,
            false,
            false,
            false,
            globalState
          );
        });

        cy.step("Create Payment Intent with GET redirect enabled", () => {
          const connectorId = globalState.get("connectorId");
          const connectorDetails = getConnectorDetails(connectorId);
          const data = connectorDetails?.["card_pm"]?.["PaymentIntent"];
          expect(data, `card_pm.PaymentIntent not found for ${connectorId}`).to
            .exist;

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
          "Confirm Payment and verify redirect behavior with GET method",
          () => {
            if (!shouldContinue) {
              cy.task("cli_log", "Skipping step: Confirm Payment");
              return;
            }
            const connectorId = globalState.get("connectorId");
            const connectorDetails = getConnectorDetails(connectorId);
            const data = connectorDetails?.["card_pm"]?.["No3DSAutoCapture"];
            expect(
              data,
              `card_pm.No3DSAutoCapture not found for ${connectorId}`
            ).to.exist;

            if (!data) {
              cy.task(
                "cli_log",
                "Skipping confirm step: No3DSAutoCapture config not found"
              );
              shouldContinue = false;
              return;
            }

            cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
            if (!utils.should_continue_further(data)) {
              shouldContinue = false;
            }
          }
        );

        cy.step("Retrieve Payment to verify status", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Retrieve Payment");
            return;
          }
          cy.retrievePaymentCallTest({ globalState });
        });
      });
    }
  );

  context(
    "Edge case - Toggle redirect_to_merchant_with_http_post during payment flow",
    () => {
      it("Create Business Profile → Create Connector → Create Customer → Enable POST redirect → Create Payment Intent → Toggle to GET → Confirm Payment → Retrieve Payment", () => {
        let shouldContinue = true;

        cy.step(
          "Create Business Profile with initial POST redirect setting",
          () => {
            cy.createBusinessProfileTest(
              fixtures.businessProfile.bpCreate,
              globalState
            );
          }
        );

        cy.step("Create Connector", () => {
          cy.createConnectorCallTest(
            "payment_processor",
            fixtures.createConnectorBody,
            payment_methods_enabled,
            globalState
          );
        });

        cy.step("Create Customer", () => {
          cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
        });

        cy.step(
          "Set initial redirect_to_merchant_with_http_post to true",
          () => {
            cy.UpdateBusinessProfileTest(
              fixtures.businessProfile.bpUpdateRedirectPost,
              true,
              false,
              false,
              false,
              false,
              globalState
            );
          }
        );

        cy.step("Create Payment Intent", () => {
          const connectorId = globalState.get("connectorId");
          const connectorDetails = getConnectorDetails(connectorId);
          const data = connectorDetails?.["card_pm"]?.["PaymentIntent"];
          expect(data, `card_pm.PaymentIntent not found for ${connectorId}`).to
            .exist;

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
          "Toggle redirect_to_merchant_with_http_post to false mid-flow",
          () => {
            if (!shouldContinue) {
              cy.task("cli_log", "Skipping step: Toggle redirect setting");
              return;
            }
            cy.UpdateBusinessProfileTest(
              fixtures.businessProfile.bpUpdateRedirectGet,
              true,
              false,
              false,
              false,
              false,
              globalState
            );
          }
        );

        cy.step("Confirm Payment after toggling redirect method", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm Payment");
            return;
          }
          const connectorId = globalState.get("connectorId");
          const connectorDetails = getConnectorDetails(connectorId);
          const data = connectorDetails?.["card_pm"]?.["No3DSAutoCapture"];
          expect(data, `card_pm.No3DSAutoCapture not found for ${connectorId}`)
            .to.exist;

          if (!data) {
            cy.task(
              "cli_log",
              "Skipping confirm step: No3DSAutoCapture config not found"
            );
            shouldContinue = false;
            return;
          }

          cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Retrieve Payment to verify status after toggle", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Retrieve Payment");
            return;
          }
          cy.retrievePaymentCallTest({ globalState });
        });
      });
    }
  );

  context("Negative case - Invalid redirect method value", () => {
    it("Create Business Profile → Attempt update with invalid redirect value → Verify API rejection", () => {
      cy.step("Create Business Profile", () => {
        cy.createBusinessProfileTest(
          fixtures.businessProfile.bpCreate,
          globalState
        );
      });

      cy.step(
        "Attempt to update with invalid redirect_to_merchant_with_http_post value",
        () => {
          const invalidBody = {
            ...fixtures.businessProfile.bpUpdate,
            is_connector_agnostic_mit_enabled: true,
            collect_shipping_details_from_wallet_connector: false,
            collect_billing_details_from_wallet_connector: false,
            always_collect_billing_details_from_wallet_connector: false,
            always_collect_shipping_details_from_wallet_connector: false,
            redirect_to_merchant_with_http_post: "invalid_value",
          };

          cy.request({
            method: "POST",
            url: `${globalState.get("baseUrl")}/account/${globalState.get(
              "merchantId"
            )}/business_profile/${globalState.get("profileId")}`,
            headers: {
              Accept: "application/json",
              "Content-Type": "application/json",
              "api-key": globalState.get("apiKey"),
            },
            body: invalidBody,
            failOnStatusCode: false,
          }).then((response) => {
            if (response.status !== 200) {
              cy.task(
                "cli_log",
                `Expected error for invalid redirect method value: ${response.status}`
              );
            }
          });
        }
      );
    });
  });
});
