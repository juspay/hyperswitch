import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";
import * as routingUtils from "../../configs/Routing/Utils";

let globalState;

describe("Surcharge payment flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Setup surcharge DSL config", () => {
    it("create-surcharge-dsl-config", () => {
      const dslData =
        routingUtils.getConnectorDetails("common")["SurchargeDecisionManager"][
          "Create"
        ];
      cy.createSurchargeDSLConfig(dslData.Request, dslData, globalState);
    });
  });

  context("Surcharge payment flow test Create and confirm", () => {
    let shouldContinue = true;

    before("check connector inclusion", function () {
      let skip = false;

      cy.task("getGlobalState")
        .then((state) => {
          globalState = new State(state);
          if (
            utils.shouldIncludeConnector(
              globalState.get("connectorId"),
              utils.CONNECTOR_LISTS.INCLUDE.SURCHARGE
            )
          ) {
            skip = true;
            return;
          }
        })
        .then(() => {
          if (skip) {
            this.skip();
          }
        });
    });

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("Create Payment Intent -> Payment Methods Call -> Confirm Payment -> Retrieve Payment", () => {
      let continueSteps = true;

      cy.step("Create Payment Intent", () => {
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
          continueSteps = false;
        }
      });

      cy.step("Payment Methods Call", () => {
        if (!continueSteps) {
          cy.task("cli_log", "Skipping step: Payment Methods Call");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment", () => {
        if (!continueSteps) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SurchargeDSLConfirm"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

        if (!utils.should_continue_further(data)) {
          continueSteps = false;
        }
      });

      cy.step("Retrieve Payment", () => {
        if (!continueSteps) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SurchargeDSLConfirm"];

        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });
});
