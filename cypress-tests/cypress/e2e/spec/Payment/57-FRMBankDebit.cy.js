import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails from "../../configs/Payment/Utils";
import {
  shouldIncludeConnector,
  CONNECTOR_LISTS,
} from "../../configs/Payment/Utils";

let globalState;

// Coverage for pre-FRM support on bank_debit payments (PR #13595): a payment
// routed through absa_sanlam (payment_method_type: eft_debit_order) is gated
// by a pre-FRM call to the sanlam_payshield FRM connector before the payment
// connector is invoked. Extends the existing card+signifyd FRM coverage
// (52-FRM.cy.js) with the newer fail_open/fail_closed failure-mode behavior,
// toggled via the same Superposition commands used for the payout pre-FRM
// coverage (frm.pre_frm_failure_mode).
//
// Uses raw cy.request for the payment create+verify calls rather than
// cy.createConfirmPaymentTest: that command hard-asserts
// response.body.connector === the payment connector under test, which does
// not hold once FRM blocks a payment before connector selection (Fraud /
// TransactionFailure+FailClosed cases).
describe("[Payment] [FRM - Pre-FRM Bank Debit with Payshield]", () => {
  let shouldContinue = true;

  before("seed global state", function () {
    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        const connector = globalState.get("connectorId");

        if (
          shouldIncludeConnector(
            connector,
            CONNECTOR_LISTS.INCLUDE.FRM_BANK_DEBIT
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

  context("[Payment FRM] Setup", () => {
    it("create-frm-connector-sanlam-payshield", () => {
      cy.createNamedConnectorCallTest(
        "payment_vas",
        fixtures.createConnectorBody,
        {},
        globalState,
        "sanlam_payshield",
        "sanlam_payshield_bank_debit_frm",
        "profile",
        "frmConnector",
        "bank_debit"
      );
    });

    it("set-frm-routing-algorithm", () => {
      cy.setFrmRoutingAlgorithm(
        { frm_routing_algorithm: { type: "single", data: "sanlam_payshield" } },
        globalState
      );
    });
  });

  context("[Payment FRM] Legit — FRM passes, payment proceeds", () => {
    it("payment-create-confirm-legit", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_debit_pm"
      ]["FRMLegit"];

      cy.request({
        method: "POST",
        url: `${globalState.get("baseUrl")}/payments`,
        headers: {
          "Content-Type": "application/json",
          "api-key": globalState.get("apiKey"),
        },
        body: {
          ...fixtures.createConfirmPaymentBody,
          ...data.Request,
          profile_id: globalState.get("profileId"),
        },
      }).then((response) => {
        expect(response.status).to.equal(200);
        globalState.set("paymentID", response.body.payment_id);
        expect(response.body.frm_message.frm_name).to.equal("sanlam_payshield");
        expect(response.body.frm_message.frm_status).to.equal("legit");
        expect(response.body.connector).to.equal("absa_sanlam");
      });
    });
  });

  context(
    "[Payment FRM] Fraud — FRM blocks before the payment connector",
    () => {
      it("payment-create-confirm-fraud", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_debit_pm"
        ]["FRMFraud"];

        cy.request({
          method: "POST",
          url: `${globalState.get("baseUrl")}/payments`,
          headers: {
            "Content-Type": "application/json",
            "api-key": globalState.get("apiKey"),
          },
          body: {
            ...fixtures.createConfirmPaymentBody,
            ...data.Request,
            profile_id: globalState.get("profileId"),
          },
        }).then((response) => {
          expect(response.status).to.equal(200);
          expect(response.body.status).to.equal("failed");
          expect(response.body.error_code).to.equal("fraud");
          expect(response.body.frm_message.frm_status).to.equal("fraud");
          expect(response.body.frm_message.frm_score).to.be.greaterThan(0);
        });
      });
    }
  );

  context(
    "[Payment FRM] Transaction Failure (4xx from Payshield) — FailClosed blocks",
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

      it("payment-create-confirm-fail-closed", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_debit_pm"
        ]["FRMTransactionFailureFailClosed"];

        cy.request({
          method: "POST",
          url: `${globalState.get("baseUrl")}/payments`,
          headers: {
            "Content-Type": "application/json",
            "api-key": globalState.get("apiKey"),
          },
          body: {
            ...fixtures.createConfirmPaymentBody,
            ...data.Request,
            profile_id: globalState.get("profileId"),
          },
        }).then((response) => {
          expect(response.status).to.equal(200);
          expect(response.body.status).to.equal("failed");
          expect(response.body.frm_message.frm_status).to.equal(
            "transaction_failure"
          );
        });
      });
    }
  );

  context(
    "[Payment FRM] Transaction Failure (4xx from Payshield) — FailOpen proceeds",
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

      it("payment-create-confirm-fail-open", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_debit_pm"
        ]["FRMTransactionFailureFailOpen"];

        cy.request({
          method: "POST",
          url: `${globalState.get("baseUrl")}/payments`,
          headers: {
            "Content-Type": "application/json",
            "api-key": globalState.get("apiKey"),
          },
          body: {
            ...fixtures.createConfirmPaymentBody,
            ...data.Request,
            profile_id: globalState.get("profileId"),
          },
        }).then((response) => {
          expect(response.status).to.equal(200);
          expect(response.body.status).to.not.equal("failed");
          expect(response.body.frm_message.frm_status).to.equal(
            "transaction_failure"
          );
          expect(response.body.connector).to.equal("absa_sanlam");
        });
      });
    }
  );
});
