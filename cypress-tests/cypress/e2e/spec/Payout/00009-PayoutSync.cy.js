import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import * as utils from "../../configs/Payout/Utils";

let globalState;

const getPayoutBody = () => Cypress._.cloneDeep(fixtures.createPayoutBody);

// UCS rollout flows that must be enabled (primary) for sepa_bank_transfer
// payouts to route through UCS.
const UCS_ROLLOUT_FLOWS = [
  "PoEligibility",
  "PoCreate",
  "PoFulfill",
  "PoSync",
  "PoRecipient",
  "PoRecipientAccount",
];

const UCS_ROLLOUT_VALUE =
  '{"rollout_percent": 1.0, "execution_mode": "primary"}';

const getUcsRolloutKey = (merchantId, connector, flow) =>
  `ucs_rollout_config_${merchantId}_${connector}_sepa_bank_transfer_${flow}`;

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
      const merchantId = globalState.get("merchantId");
      const connector = globalState.get("connectorId");

      cy.setConfigs(globalState, "ucs_enabled", "true", "CREATE");

      UCS_ROLLOUT_FLOWS.forEach((flow) => {
        cy.setConfigs(
          globalState,
          getUcsRolloutKey(merchantId, connector, flow),
          UCS_ROLLOUT_VALUE,
          "CREATE"
        );
      });
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

      cy.createConfirmPayoutTest(
        getPayoutBody(),
        data,
        true,
        true,
        globalState
      ).then((response) => {
        cy.assertUcsPayoutCreateResponse(globalState, response);
        cy.assertPayoutBankDetailsMasked(
          response,
          data.Request.payout_method_data.bank_transfer
        );
      });
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

      cy.retrievePayoutForceSyncCallTest(globalState, data).then((response) => {
        cy.assertUcsPayoutSyncResponse(globalState, response);
      });
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("force-sync-payout-idempotency-test", () => {
      const data = Cypress._.cloneDeep(
        utils.getConnectorDetails(globalState.get("connectorId"))[
          "bank_transfer_pm"
        ]["sepa_bank_transfer"]["SyncIdempotent"]
      );

      cy.retrievePayoutForceSyncCallTest(globalState, data).then((response) => {
        cy.assertUcsPayoutSyncResponse(globalState, response);
      });
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

      cy.retrievePayoutForceSyncCallTest(
        globalState,
        data,
        "payout_unknown123"
      );
    });
  });

  context("UCS cleanup", () => {
    it("cleanup-ucs-configs", () => {
      const merchantId = globalState.get("merchantId");
      const connector = globalState.get("connectorId");

      UCS_ROLLOUT_FLOWS.forEach((flow) => {
        cy.setConfigs(
          globalState,
          getUcsRolloutKey(merchantId, connector, flow),
          UCS_ROLLOUT_VALUE,
          "DELETE"
        );
      });
    });
  });
});
