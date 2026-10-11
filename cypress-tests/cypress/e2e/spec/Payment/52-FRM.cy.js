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
        cy.setFrmRoutingAlgorithm(
          fixtures.frmRoutingAlgorithmBody,
          globalState
        );
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
        cy.screenshot("frm-connector-creation");
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
        cy.screenshot("frm-approve-payment");
      });

      cy.step("Retrieve Payment to Verify Status", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }

        cy.retrievePaymentCallTest({
          globalState,
          data: {
            Response: { body: { frm_message: { frm_name: "signifyd" } } },
          },
        });
        cy.screenshot("frm-approve-payment-status");
      });
    });

    afterEach("Delete FRM connector", () => {
      cy.deleteFrmConnector(globalState);
    });
  });

  context("FRM routing - profile precedence", () => {
    it("skip-frm-when-profile-routes-to-connector-without-mca", () => {
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
      });

      cy.step("Set business profile FRM routing to riskified (no MCA)", () => {
        cy.setProfileFrmRoutingAlgorithm(
          { type: "single", data: "riskified" },
          globalState
        );
      });

      cy.step("Create and Confirm Payment - FRM is not invoked", () => {
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

      cy.step("Retrieve Payment - no FRM message", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }

        cy.retrievePaymentCallTest({
          globalState,
          data: { Response: { body: { frm_message: null } } },
        });
      });
    });

    afterEach("Delete FRM connector and restore profile FRM routing", () => {
      cy.deleteFrmConnector(globalState);
      // The business profile endpoint's update is a diesel AsChangeset over
      // Option<T> fields with no `treat_none_as_null`, so sending `null` here
      // would be silently skipped rather than clearing the override set above
      // — it must be reset to an explicit, known-good value instead. Reset to
      // signifyd (the merchant account's value) so later specs in this phase,
      // which share this same merchant/profile via globalState, don't inherit
      // a profile-level override pointing at a connector with no MCA.
      cy.setProfileFrmRoutingAlgorithm(
        { type: "single", data: "signifyd" },
        globalState
      );
    });
  });
});
