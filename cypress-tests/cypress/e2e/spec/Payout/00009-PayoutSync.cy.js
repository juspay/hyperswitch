import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import * as utils from "../../configs/Payout/Utils";

let globalState;

const getPayoutBody = () => Cypress._.cloneDeep(fixtures.createPayoutBody);

// UCS rollout flows that must be enabled (primary) for sepa_bank_transfer
// payouts to route through UCS. Shared by setup/cleanup via
// cy.createRolloutConfig / cy.deleteRolloutConfig (see commands.js), which
// also handle connector resolution and key generation.
const SEPA_BANK_TRANSFER_UCS_FLOWS = [
  "sepa_bank_transfer_PoEligibility",
  "sepa_bank_transfer_PoCreate",
  "sepa_bank_transfer_PoFulfill",
  "sepa_bank_transfer_PoSync",
  "sepa_bank_transfer_PoRecipient",
  "sepa_bank_transfer_PoRecipientAccount",
].join(",");

describe("[Payout] Sync", () => {
  let shouldContinue = true;

  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);

      if (
        !globalState.get("payoutsExecution") ||
        !utils.CONNECTOR_LISTS?.INCLUDE?.PAYOUT_SYNC?.includes(
          globalState.get("connectorId")
        )
      ) {
        shouldContinue = false;
      }
    });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  beforeEach(function () {
    if (!shouldContinue) {
      this.skip();
    }
  });

  it("create customer", () => {
    cy.createCustomerCallTest(
      Cypress._.cloneDeep(fixtures.customerCreateBody),
      globalState
    );
  });

  context("UCS setup for sepa_bank_transfer payouts", () => {
    it("setup-ucs-configs", () => {
      // createRolloutConfig no-ops (without failing) unless this is set —
      // it's a client-side gate, separate from the "ucs_enabled" merchant
      // config below.
      if (!globalState.get("ucsEnabled")) {
        globalState.set("ucsEnabled", true);
      }

      cy.setupConfigs(globalState, "ucs_enabled", "true");

      cy.createRolloutConfig(globalState, SEPA_BANK_TRANSFER_UCS_FLOWS, {
        rollout_percent: 1.0,
        execution_mode: "primary",
      });
    });

    // createRolloutConfig only logs on an upsert failure, it never fails the
    // test — so a partially-failed setup (e.g. one of the 6 flows silently
    // not persisted) would otherwise surface downstream as an unexplained
    // fall-through to the legacy path instead of here, at the actual cause.
    it("verify-ucs-configs", () => {
      cy.verifyRolloutConfig(globalState, SEPA_BANK_TRANSFER_UCS_FLOWS, {
        rollout_percent: 1.0,
        execution_mode: "primary",
      });
    });

    // The router doesn't pick up a freshly-written rollout config instantly.
    // A plain wait (no real payout side effects) rather than polling with
    // throwaway payout creates — repeated non-success payout attempts can
    // trip the router's per-scope kill switch, which force-diverts the flow
    // to the Direct integration; deutschebank has no Direct payout
    // implementation, so a tripped switch fails everything, not just this
    // step.
    it("wait-for-ucs-config-propagation", () => {
      // eslint-disable-next-line cypress/no-unnecessary-waiting
      cy.wait(15000);
    });
  });

  context("Payout create with auto fulfill then force sync", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("confirm-payout-call-with-auto-fulfill-test", () => {
      const data = Cypress._.cloneDeep(
        utils.getConnectorDetails(globalState.get("connectorId"))[
          "bank_transfer_pm"
        ]["sepa_bank_transfer"]["Fulfill"]
      );
      if (!utils.should_continue_further(data)) {
        shouldContinue = false;
        return;
      }

      cy.createConfirmUcsPayoutTest(
        getPayoutBody(),
        data,
        true,
        true,
        globalState
      );
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("force-sync-payout-call-test", () => {
      const data = Cypress._.cloneDeep(
        utils.getConnectorDetails(globalState.get("connectorId"))[
          "bank_transfer_pm"
        ]["sepa_bank_transfer"]["Sync"]
      );
      if (!utils.should_continue_further(data)) {
        shouldContinue = false;
        return;
      }

      cy.retrievePayoutUcsForceSyncCallTest(globalState, data);
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("force-sync-payout-idempotency-test", () => {
      const data = Cypress._.cloneDeep(
        utils.getConnectorDetails(globalState.get("connectorId"))[
          "bank_transfer_pm"
        ]["sepa_bank_transfer"]["SyncIdempotent"]
      );

      cy.retrievePayoutUcsForceSyncCallTest(globalState, data);
    });
  });

  context("Negative: payout create without source account holder name", () => {
    it("create-payout-without-source-account-holder-name-test", () => {
      const data = Cypress._.cloneDeep(
        utils.getConnectorDetails(globalState.get("connectorId"))[
          "bank_transfer_pm"
        ]["sepa_bank_transfer"]["CreateWithoutSourceAccountHolderName"]
      );

      cy.createConfirmPayoutTest(
        getPayoutBody(),
        data,
        true,
        true,
        globalState
      );
    });
  });

  context("Negative: force sync unknown payout", () => {
    it("force-sync-unknown-payout-test", () => {
      const data = Cypress._.cloneDeep(
        utils.getConnectorDetails(globalState.get("connectorId"))[
          "bank_transfer_pm"
        ]["sepa_bank_transfer"]["SyncNonExistentPayout"]
      );

      cy.retrievePayoutCallTest(globalState, data, {
        forceSync: true,
        payoutId: "payout_unknown123",
      });
    });
  });

  context("UCS cleanup", () => {
    it("cleanup-ucs-configs", () => {
      cy.deleteRolloutConfig(globalState, SEPA_BANK_TRANSFER_UCS_FLOWS, {
        rollout_percent: 1.0,
        execution_mode: "primary",
      });
      cy.setConfigs(globalState, "ucs_enabled", "true", "DELETE");
    });
  });
});
