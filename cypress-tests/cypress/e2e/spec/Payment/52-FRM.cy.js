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

        if (shouldIncludeConnector(connector, CONNECTOR_LISTS.INCLUDE.FRM)) {
          skip = true;
          return;
        }
      })
      .then(() => {
        if (skip) {
          this.skip();
        }
      })
      .then(() => {
        if (skip) return;
        // Set frm_routing_algorithm on merchant account so Hyperswitch knows to use Signifyd for FRM
        cy.request({
          method: "POST",
          url: `${globalState.get("baseUrl")}/accounts/${globalState.get("merchantId")}`,
          headers: {
            "Content-Type": "application/json",
            Accept: "application/json",
            "api-key": globalState.get("adminApiKey"),
          },
          body: {
            merchant_id: globalState.get("merchantId"),
            frm_routing_algorithm: {
              data: "signifyd",
              type: "single",
            },
          },
          failOnStatusCode: false,
        }).then((response) => {
          cy.task(
            "cli_log",
            "Set frm_routing_algorithm status: " +
              response.status +
              " body: " +
              JSON.stringify(response.body)
          );
          expect(
            response.status,
            "frm_routing_algorithm update should return 200"
          ).to.equal(200);
        });
      });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("FRM with Signifyd - Create FRM Connector + Payment", () => {
    it("create-frm-connector-signifyd-and-approve-payment", () => {
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

      cy.step("Create and Confirm Payment with FRM (Approve)", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Create and Confirm Payment with FRM"
          );
          return;
        }

        const data = getConnectorDetails("signifyd")["card_pm"]["FRMApprove"];

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

        cy.retrievePaymentCallTest({ globalState });
      });
    });

    it("create-frm-connector-signifyd-and-decline-payment", () => {
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

      cy.step("Create and Confirm Payment with FRM (Decline)", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Create and Confirm Payment with FRM"
          );
          return;
        }

        const data = getConnectorDetails("signifyd")["card_pm"]["FRMDecline"];

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

        cy.retrievePaymentCallTest({ globalState });
      });
    });

    it("create-frm-connector-signifyd-and-hold-payment", () => {
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

      cy.step("Create and Confirm Payment with FRM (Hold)", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Create and Confirm Payment with FRM"
          );
          return;
        }

        const data = getConnectorDetails("signifyd")["card_pm"]["FRMHold"];

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

        cy.retrievePaymentCallTest({ globalState });
      });
    });

    afterEach("Delete FRM connector", () => {
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
});
