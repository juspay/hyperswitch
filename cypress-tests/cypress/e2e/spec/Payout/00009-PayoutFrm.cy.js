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
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["frm_legit"]["Create"];

      cy.createConfirmPayoutTest(
        fixtures.createPayoutBody,
        data,
        true,
        true,
        globalState
      );
      if (contextShouldContinue)
        contextShouldContinue = utils.should_continue_further(data);
    });

    it("verify frm_status is legit", () => {
      cy.getPayoutDetails(globalState).then((response) => {
        expect(response.body.frm_message.frm_name).to.equal("sanlam_payshield");
        expect(response.body.frm_message.frm_status).to.equal("legit");
        // Proves the payout actually reached the connector, not just that
        // FRM flagged it as legit.
        expect(response.body.status).to.equal("initiated");
        expect(response.body.connector).to.equal("gotyme_sanlam");
      });
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
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["frm_fraud"]["Create"];

      cy.createConfirmPayoutTest(
        fixtures.createPayoutBody,
        data,
        true,
        true,
        globalState
      );
    });

    it("verify frm_status is fraud and connector was never called", () => {
      cy.getPayoutDetails(globalState).then((response) => {
        expect(response.body.status).to.equal("failed");
        expect(response.body.error_code).to.equal("fraud");
        expect(response.body.connector).to.be.null;
        expect(response.body.frm_message.frm_status).to.equal("fraud");
        expect(response.body.frm_message.frm_score).to.be.greaterThan(0);
      });
    });
  });

  context(
    "[Payout FRM] Transaction Failure (4xx from Payshield) — FailClosed blocks",
    () => {
      it("break sanlam_payshield credentials", () => {
        cy.request({
          method: "POST",
          url: `${globalState.get("baseUrl")}/account/${globalState.get("merchantId")}/connectors/${globalState.get("frmConnectorId")}`,
          headers: {
            "Content-Type": "application/json",
            "api-key": globalState.get("apiKey"),
          },
          body: {
            connector_type: "payment_vas",
            connector_account_details: {
              auth_type: "HeaderKey",
              api_key: "invalid_key_to_force_frm_failure",
            },
          },
        }).then((response) => {
          expect(response.status).to.equal(200);
        });
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
        const data = utils.getConnectorDetails(globalState.get("connectorId"))[
          "bank_transfer_pm"
        ]["frm_transaction_failure_fail_closed"]["Create"];

        cy.createConfirmPayoutTest(
          fixtures.createPayoutBody,
          data,
          true,
          true,
          globalState
        );
      });

      it("verify payout blocked with transaction_failure", () => {
        cy.getPayoutDetails(globalState).then((response) => {
          expect(response.body.status).to.equal("failed");
          expect(response.body.error_code).to.equal("transaction_failure");
          expect(response.body.connector).to.be.null;
          expect(response.body.frm_message.frm_status).to.equal(
            "transaction_failure"
          );
        });
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
        const data = utils.getConnectorDetails(globalState.get("connectorId"))[
          "bank_transfer_pm"
        ]["frm_transaction_failure_fail_open"]["Create"];

        cy.createConfirmPayoutTest(
          fixtures.createPayoutBody,
          data,
          true,
          true,
          globalState
        );
      });

      it("verify FRM failure did not block the payout", () => {
        cy.getPayoutDetails(globalState).then((response) => {
          expect(response.body.error_code).to.not.equal("transaction_failure");
          expect(response.body.frm_message.frm_status).to.equal(
            "transaction_failure"
          );
          // Proves the payout actually reached the connector despite the
          // FRM failure, not just that it avoided this one error code.
          expect(response.body.status).to.equal("initiated");
          expect(response.body.connector).to.equal("gotyme_sanlam");
        });
      });
    }
  );
});
