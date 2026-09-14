import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Block Implicit Customer Creation", () => {
  let specShouldSkip = false;

  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      const connectorId = globalState.get("connectorId");
      specShouldSkip = utils.shouldIncludeConnector(
        connectorId,
        utils.CONNECTOR_LISTS.INCLUDE.BLOCK_IMPLICIT_CUSTOMER_CREATION
      );
      if (
        !globalState.get("superpositionBaseUrl") ||
        !globalState.get("superpositionSecret")
      ) {
        cy.task(
          "cli_log",
          "Superposition credentials not set — skipping BlockImplicitCustomerCreation spec"
        );
        specShouldSkip = true;
      }
    });
  });

  beforeEach(function () {
    if (specShouldSkip) {
      this.skip();
    }
  });

  after("cleanup superposition config + flush global state", () => {
    cy.setSuperpositionConfig(
      globalState,
      "block_implicit_customer_creation",
      false,
      {
        organization_id: globalState.get("organizationId"),
      }
    );
    cy.task("setGlobalState", globalState.data);
  });

  context("Implicit customer creation allowed (default behavior)", () => {
    it("create payment with non-existent customer_id — verify implicit creation", () => {
      let shouldContinue = true;

      cy.step("Set non-existent customer_id in globalState", () => {
        globalState.set("customerId", `non_existent_customer_${Date.now()}`);
      });

      cy.step(
        "Create payment intent (expect 200, requires_payment_method)",
        () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Create payment intent");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["BlockImplicitCustomerCreationAllowed"];
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
        }
      );

      cy.step("Retrieve customer — verify created (expect 200)", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve customer");
          return;
        }
        cy.customerRetrieveCall(globalState, 200);
      });
    });
  });

  context("Block implicit customer creation via superposition config", () => {
    it("set block config, create payment with non-existent customer (expect 404), verify customer not created", () => {
      const shouldContinue = true;

      cy.step(
        "Set block_implicit_customer_creation=true via superposition",
        () => {
          cy.setSuperpositionConfig(
            globalState,
            "block_implicit_customer_creation",
            true,
            {
              organization_id: globalState.get("organizationId"),
            }
          );
        }
      );

      cy.step("Wait for config propagation (poll until 404)", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Wait for config propagation");
          return;
        }
        cy.waitForConfigPropagation(globalState, 404, "block");
      });

      cy.step(
        "Create payment with non-existent customer_id (expect 404 HE_02)",
        () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Create payment");
            return;
          }
          globalState.set("customerId", `non_existent_customer_${Date.now()}`);
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["BlockImplicitCustomerCreationBlocked"];
          cy.createPaymentIntentTest(
            fixtures.createPaymentBody,
            data,
            "no_three_ds",
            "automatic",
            globalState
          );
          // Do NOT call should_continue_further — 404 is expected,
          // retrieve must still run to verify customer was not created
        }
      );

      cy.step("Retrieve customer — verify not created (expect 404)", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve customer");
          return;
        }
        cy.customerRetrieveCall(globalState, 404);
      });
    });
  });

  context(
    "Restore default behavior after resetting superposition config",
    () => {
      it("reset block config, create payment with non-existent customer (expect 200), verify customer created", () => {
        let shouldContinue = true;

        cy.step(
          "Reset block_implicit_customer_creation to false via superposition",
          () => {
            cy.setSuperpositionConfig(
              globalState,
              "block_implicit_customer_creation",
              false,
              {
                organization_id: globalState.get("organizationId"),
              }
            );
          }
        );

        cy.step("Wait for config propagation (poll until 200)", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Wait for config propagation");
            return;
          }
          cy.waitForConfigPropagation(globalState, 200, "allow");
        });

        cy.step(
          "Create payment with non-existent customer_id (expect 200, requires_payment_method)",
          () => {
            if (!shouldContinue) {
              cy.task("cli_log", "Skipping step: Create payment");
              return;
            }
            globalState.set(
              "customerId",
              `non_existent_customer_${Date.now()}`
            );
            const data = getConnectorDetails(globalState.get("connectorId"))[
              "card_pm"
            ]["BlockImplicitCustomerCreationAllowed"];
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
          }
        );

        cy.step("Retrieve customer — verify created (expect 200)", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Retrieve customer");
            return;
          }
          cy.customerRetrieveCall(globalState, 200);
        });
      });
    }
  );
});
