import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import * as utils from "../../configs/Payout/Utils";

let globalState;

// Coverage for pre-FRM support on payouts (PR #14189): a payout routed
// through gotyme_sanlam is gated by a pre-FRM call to the sanlam_payshield
// FRM connector before the payout connector is ever invoked. Mirrors the
// scenario matrix from the PR's own manual verification:
// Legit / Fraud / TransactionFailure(4xx) x FailOpen / FailClosed.
//
// payouts.payout_frm_call is disabled by default and gated behind
// Superposition, same as frm.pre_frm_failure_mode, so both are toggled here
// via the existing cy.setSuperpositionConfig infrastructure.
describe("[Payout] [FRM - Pre-FRM with Payshield]", () => {
  let shouldContinue = true;

  before("seed global state", function () {
    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);

        if (!globalState.get("payoutsExecution")) {
          shouldContinue = false;
        }

        if (
          !utils.CONNECTOR_LISTS.INCLUDE.PAYOUT_FRM.includes(
            globalState.get("connectorId")
          )
        ) {
          shouldContinue = false;
        }
      })
      .then(() => {
        if (!shouldContinue) {
          this.skip();
        }
      });
  });

  after("cleanup configs + flush global state", () => {
    cy.deleteSuperpositionConfig(globalState, {
      provider_merchant_id: globalState.get("merchantId"),
      processor_merchant_id: globalState.get("merchantId"),
      profile_id: globalState.get("profileId"),
    });
    cy.deleteFrmConnector(globalState);
    cy.task("setGlobalState", globalState.data);
  });

  beforeEach(function () {
    if (!shouldContinue) {
      this.skip();
    }
  });

  context("[Payout FRM] Setup", () => {
    it("create-frm-connector-sanlam-payshield", () => {
      cy.createNamedConnectorCallTest(
        "payment_vas",
        fixtures.createConnectorBody,
        {},
        globalState,
        "sanlam_payshield",
        "sanlam_payshield_frm",
        "profile",
        "frmConnector",
        "bank_transfer"
      );
    });

    it("set-frm-routing-algorithm", () => {
      cy.setFrmRoutingAlgorithm(
        { frm_routing_algorithm: { type: "single", data: "sanlam_payshield" } },
        globalState
      );
    });

    it("enable-payout-frm-call-via-superposition", () => {
      cy.setSuperpositionConfig(globalState, "payouts.payout_frm_call", true, {
        provider_merchant_id: globalState.get("merchantId"),
        processor_merchant_id: globalState.get("merchantId"),
        profile_id: globalState.get("profileId"),
      });
    });
  });

  context("[Payout FRM] Legit — FRM passes, payout proceeds", () => {
    let contextShouldContinue = true;

    beforeEach(function () {
      if (!contextShouldContinue) {
        this.skip();
      }
    });

    it("create customer", () => {
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
    });

    it("payout-create-confirm-auto-fulfill-legit", () => {
      const scenario = utils.getConnectorDetails(
        globalState.get("connectorId")
      )["bank_transfer_pm"]["frm_legit"];

      cy.createConfirmPayoutTest(
        fixtures.createPayoutBody,
        scenario["Create"],
        true,
        true,
        globalState
      );
      if (contextShouldContinue)
        contextShouldContinue = utils.should_continue_further(
          scenario["Create"]
        );
    });

    it("verify frm_status is legit", () => {
      const scenario = utils.getConnectorDetails(
        globalState.get("connectorId")
      )["bank_transfer_pm"]["frm_legit"];

      cy.verifyPayoutFrmDetails(globalState, scenario["FrmDetails"]);
    });
  });

  context("[Payout FRM] Fraud — FRM blocks before the payout connector", () => {
    const contextShouldContinue = true;

    beforeEach(function () {
      if (!contextShouldContinue) {
        this.skip();
      }
    });

    it("create customer", () => {
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
    });

    it("payout-create-confirm-auto-fulfill-fraud", () => {
      const scenario = utils.getConnectorDetails(
        globalState.get("connectorId")
      )["bank_transfer_pm"]["frm_fraud"];

      cy.createConfirmPayoutTest(
        fixtures.createPayoutBody,
        scenario["Create"],
        true,
        true,
        globalState
      );
    });

    it("verify frm_status is fraud and connector was never called", () => {
      const scenario = utils.getConnectorDetails(
        globalState.get("connectorId")
      )["bank_transfer_pm"]["frm_fraud"];

      cy.verifyPayoutFrmDetails(globalState, scenario["FrmDetails"]);
    });
  });

  context(
    "[Payout FRM] Transaction Failure (4xx from Payshield) — FailClosed blocks",
    () => {
      it("break sanlam_payshield credentials", () => {
        cy.breakFrmConnectorCredentials(globalState);
      });

      it("set pre_frm_failure_mode=fail_closed via superposition", () => {
        cy.setSuperpositionConfig(
          globalState,
          "frm.pre_frm_failure_mode",
          "fail_closed",
          {
            provider_merchant_id: globalState.get("merchantId"),
            processor_merchant_id: globalState.get("merchantId"),
            profile_id: globalState.get("profileId"),
          }
        );
      });

      it("create customer", () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      it("payout-create-confirm-auto-fulfill-fail-closed", () => {
        const scenario = utils.getConnectorDetails(
          globalState.get("connectorId")
        )["bank_transfer_pm"]["frm_transaction_failure_fail_closed"];

        cy.createConfirmPayoutTest(
          fixtures.createPayoutBody,
          scenario["Create"],
          true,
          true,
          globalState
        );
      });

      it("verify payout blocked with transaction_failure", () => {
        const scenario = utils.getConnectorDetails(
          globalState.get("connectorId")
        )["bank_transfer_pm"]["frm_transaction_failure_fail_closed"];

        cy.verifyPayoutFrmDetails(globalState, scenario["FrmDetails"]);
      });
    }
  );

  context(
    "[Payout FRM] Transaction Failure (4xx from Payshield) — FailOpen proceeds",
    () => {
      // sanlam_payshield credentials are already broken from the FailClosed
      // context above; only the failure mode changes here.
      it("set pre_frm_failure_mode=fail_open via superposition", () => {
        cy.setSuperpositionConfig(
          globalState,
          "frm.pre_frm_failure_mode",
          "fail_open",
          {
            provider_merchant_id: globalState.get("merchantId"),
            processor_merchant_id: globalState.get("merchantId"),
            profile_id: globalState.get("profileId"),
          }
        );
      });

      it("create customer", () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      it("payout-create-confirm-auto-fulfill-fail-open", () => {
        const scenario = utils.getConnectorDetails(
          globalState.get("connectorId")
        )["bank_transfer_pm"]["frm_transaction_failure_fail_open"];

        cy.createConfirmPayoutTest(
          fixtures.createPayoutBody,
          scenario["Create"],
          true,
          true,
          globalState
        );
      });

      it("verify FRM failure did not block the payout", () => {
        const scenario = utils.getConnectorDetails(
          globalState.get("connectorId")
        )["bank_transfer_pm"]["frm_transaction_failure_fail_open"];

        cy.verifyPayoutFrmDetails(globalState, scenario["FrmDetails"]);
      });
    }
  );
});
