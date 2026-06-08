import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";
import {
  shouldIncludeConnector,
  CONNECTOR_LISTS,
} from "../../configs/Payment/Utils";

let globalState;

describe("FRM - Fraud Risk Management Tests", () => {
  before("seed global state", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        const connector = globalState.get("connectorId");

        if (
          shouldIncludeConnector(
            connector,
            CONNECTOR_LISTS.INCLUDE.FRM
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

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("FRM with Signifyd - Create FRM Connector + Payment", () => {
    it("create-frm-connector-signifyd-and-confirm-payment-test", () => {
      let shouldContinue = true;

      cy.step("Create FRM Connector (Signifyd)", () => {
        cy.createNamedConnectorCallTest(
          "payment_vas",
          fixtures.createConnectorBody,
          {},
          globalState,
          "signifyd",
          "signifyd_frm",
          "profile",
          "frmConnector"
        );

        if (
          !utils.should_continue_further({
            Response: { status: 200, body: {} },
          })
        ) {
          shouldContinue = false;
        }
      });

      cy.step("Create and Confirm Payment with FRM", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Create and Confirm Payment with FRM"
          );
          return;
        }

        const data = getConnectorDetails("signifyd")[
          "card_pm"
        ]["FRM"];

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

      cy.step("Retrieve Payment to Verify Status", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }

        cy.retrievePaymentCallTest(globalState);
      });
    });

    after("Delete FRM connector", () => {
      const frmMcaId = globalState.get("frmConnectorId");
      if (frmMcaId) {
        cy.request({
          method: "DELETE",
          url: `${globalState.get("baseUrl")}/account/${globalState.get(
            "merchantId"
          )}/connectors/${frmMcaId}`,
          headers: {
            Accept: "application/json",
            "api-key": globalState.get("adminApiKey"),
          },
          failOnStatusCode: false,
        }).then((response) => {
          cy.task(
            "cli_log",
            "FRM Signifyd connector delete status: " + response.status
          );
        });
      }
    });
  });

  context("FRM with Riskified - Create FRM Connector + Payment", () => {
    it("create-frm-connector-riskified-and-confirm-payment-test", () => {
      let shouldContinue = true;

      cy.step("Create FRM Connector (Riskified)", () => {
        cy.createNamedConnectorCallTest(
          "payment_vas",
          fixtures.createConnectorBody,
          {},
          globalState,
          "riskified",
          "riskified_frm",
          "profile",
          "frmConnector"
        );

        if (
          !utils.should_continue_further({
            Response: { status: 200, body: {} },
          })
        ) {
          shouldContinue = false;
        }
      });

      cy.step("Create and Confirm Payment with FRM", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Create and Confirm Payment with FRM"
          );
          return;
        }

        const data = getConnectorDetails("riskified")[
          "card_pm"
        ]["FRM"];

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

      cy.step("Retrieve Payment to Verify Status", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }

        cy.retrievePaymentCallTest(globalState);
      });
    });

    after("Delete FRM connector", () => {
      const frmMcaId = globalState.get("frmConnectorId");
      if (frmMcaId) {
        cy.request({
          method: "DELETE",
          url: `${globalState.get("baseUrl")}/account/${globalState.get(
            "merchantId"
          )}/connectors/${frmMcaId}`,
          headers: {
            Accept: "application/json",
            "api-key": globalState.get("adminApiKey"),
          },
          failOnStatusCode: false,
        }).then((response) => {
          cy.task(
            "cli_log",
            "FRM Riskified connector delete status: " + response.status
          );
        });
      }
    });
  });

  context(
    "FRM with CyberSource Decision Manager - Create FRM Connector + Payment",
    () => {
      it("create-frm-connector-cybersourcedm-and-confirm-payment-test", () => {
        let shouldContinue = true;

        cy.step("Create FRM Connector (CyberSource DM)", () => {
          cy.createNamedConnectorCallTest(
            "payment_vas",
            fixtures.createConnectorBody,
            {},
            globalState,
            "cybersourcedecisionmanager",
            "cybersourcedm_frm",
            "profile",
            "frmConnector"
          );

          if (
            !utils.should_continue_further({
              Response: { status: 200, body: {} },
            })
          ) {
            shouldContinue = false;
          }
        });

        cy.step("Create and Confirm Payment with FRM", () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Create and Confirm Payment with FRM"
            );
            return;
          }

          const data = getConnectorDetails("cybersourcedecisionmanager")[
            "card_pm"
          ]["FRM"];

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

        cy.step("Retrieve Payment to Verify Status", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Retrieve Payment");
            return;
          }

          cy.retrievePaymentCallTest(globalState);
        });
      });

      after("Delete FRM connector", () => {
        const frmMcaId = globalState.get("frmConnectorId");
        if (frmMcaId) {
          cy.request({
            method: "DELETE",
            url: `${globalState.get("baseUrl")}/account/${globalState.get(
              "merchantId"
            )}/connectors/${frmMcaId}`,
            headers: {
              Accept: "application/json",
              "api-key": globalState.get("adminApiKey"),
            },
            failOnStatusCode: false,
          }).then((response) => {
            cy.task(
              "cli_log",
              "FRM CyberSourceDM connector delete status: " + response.status
            );
          });
        }
      });
    }
  );
});
