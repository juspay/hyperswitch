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
        const maxAttempts = 60;
        const intervalMs = 5000;
        const poll = (attempt) => {
          if (attempt >= maxAttempts) {
            throw new Error(
              `Superposition config did not propagate within ${(maxAttempts * intervalMs) / 1000}s`
            );
          }
          cy.request({
            method: "POST",
            url: `${globalState.get("baseUrl")}/payments`,
            headers: {
              "api-key": globalState.get("apiKey"),
              "Content-Type": "application/json",
            },
            body: {
              ...fixtures.createPaymentBody,
              currency: "USD",
              amount: 100,
              customer_id: `poll_block_${Date.now()}_${attempt}`,
              authentication_type: "no_three_ds",
              capture_method: "automatic",
              profile_id: globalState.get("profileId"),
            },
            failOnStatusCode: false,
          }).then((response) => {
            if (response.status === 404) {
              cy.task(
                "cli_log",
                `Config propagated after ${attempt + 1} poll attempt(s)`
              );
            } else {
              cy.task(
                "cli_log",
                `Poll attempt ${attempt + 1}: got ${response.status}, waiting ${intervalMs / 1000}s...`
              );
              cy.wait(intervalMs).then(() => poll(attempt + 1));
            }
          });
        };
        poll(0);
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
          const maxAttempts = 60;
          const intervalMs = 5000;
          const poll = (attempt) => {
            if (attempt >= maxAttempts) {
              throw new Error(
                `Superposition config did not propagate within ${(maxAttempts * intervalMs) / 1000}s`
              );
            }
            cy.request({
              method: "POST",
              url: `${globalState.get("baseUrl")}/payments`,
              headers: {
                "api-key": globalState.get("apiKey"),
                "Content-Type": "application/json",
              },
              body: {
                ...fixtures.createPaymentBody,
                currency: "USD",
                amount: 100,
                customer_id: `poll_allow_${Date.now()}_${attempt}`,
                authentication_type: "no_three_ds",
                capture_method: "automatic",
                profile_id: globalState.get("profileId"),
              },
              failOnStatusCode: false,
            }).then((response) => {
              if (response.status === 200) {
                cy.task(
                  "cli_log",
                  `Config propagated after ${attempt + 1} poll attempt(s)`
                );
              } else {
                cy.task(
                  "cli_log",
                  `Poll attempt ${attempt + 1}: got ${response.status}, waiting ${intervalMs / 1000}s...`
                );
                cy.wait(intervalMs).then(() => poll(attempt + 1));
              }
            });
          };
          poll(0);
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
