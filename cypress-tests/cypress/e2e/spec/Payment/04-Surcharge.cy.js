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

  context("Surcharge payment flow test Create and confirm", () => {
    let shouldContinue = true;

    before("setup surcharge DSL", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
        if (
          utils.shouldIncludeConnector(
            globalState.get("connectorId"),
            utils.CONNECTOR_LISTS.INCLUDE.SURCHARGE
          )
        ) {
          shouldContinue = false;
          return;
        }
        const dslData =
          routingUtils.getConnectorDetails("common")[
            "SurchargeDecisionManager"
          ]["Create"];

        cy.request({
          method: "PUT",
          url: `${globalState.get("baseUrl")}/routing/decision/surcharge`,
          headers: {
            "api-key":
              globalState.get("apiKey") || globalState.get("adminApiKey"),
            "Content-Type": "application/json",
          },
          body: dslData.Request,
          failOnStatusCode: false,
        }).then((response) => {
          if (response.status === 200) {
            globalState.set("surchargeDSLConfig", response.body);
            for (const key in dslData.Response.body) {
              expect(dslData.Response.body[key]).to.deep.equal(
                response.body[key]
              );
            }
          } else {
            const errorCode = response.body?.error?.code;
            if (errorCode === "IR_04" || errorCode === "IR_17") {
              // Surcharge DSL endpoint requires JWT auth in release-mode servers.
              // These tests target the non-release CI build where api-key is accepted.
              cy.task(
                "cli_log",
                `[Surcharge] Skipping: /routing/decision/surcharge requires JWT auth (${errorCode}). Test targets non-release CI builds.`
              );
              shouldContinue = false;
            } else {
              throw new Error(
                `PUT /routing/decision/surcharge failed (${response.status}): ${JSON.stringify(response.body)}`
              );
            }
          }
        });
      });
    });

    after("cleanup surcharge DSL", () => {
      if (shouldContinue) {
        const dslData =
          routingUtils.getConnectorDetails("common")[
            "SurchargeDecisionManager"
          ]["Delete"];
        cy.deleteSurchargeDSLConfig(dslData, globalState);
      }
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
