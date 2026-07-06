import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";
import * as routingUtils from "../../configs/Routing/Utils";

let globalState;

describe("Surcharge via Decision Manager", () => {
  let shouldContinue = true;

  before("seed state + setup surcharge DSL", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      if (
        !utils.CONNECTOR_LISTS.INCLUDE.SURCHARGE.includes(
          globalState.get("connectorId")
        )
      ) {
        shouldContinue = false;
        return;
      }
      const dslData =
        routingUtils.getConnectorDetails("common")["SurchargeDecisionManager"][
          "Create"
        ];
      cy.createSurchargeDSLConfig(dslData.Request, dslData, globalState);
    });
  });

  after("flush state + cleanup surcharge DSL", () => {
    if (shouldContinue) {
      const dslData =
        routingUtils.getConnectorDetails("common")["SurchargeDecisionManager"][
          "Delete"
        ];
      cy.deleteSurchargeDSLConfig(dslData, globalState);
    }
    cy.task("setGlobalState", globalState.data);
  });

  beforeEach(function () {
    if (!shouldContinue) {
      this.skip();
    }
  });

  context("Surcharge via DSL Decision Manager", () => {
    it("Create Payment (DSL auto-applies surcharge) -> Confirm -> Retrieve and verify surcharge", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent (no explicit surcharge_details)", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SurchargeDSL"];

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
        ]["SurchargeDSLConfirm"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step(
        "Retrieve Payment and verify DSL surcharge was applied",
        () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Retrieve Payment and verify DSL surcharge was applied"
            );
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["SurchargeDSLConfirm"];

          cy.retrievePaymentCallTest({ globalState, data });
        }
      );
    });
  });
});
