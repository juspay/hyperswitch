import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import * as utils from "../../configs/Payout/Utils";

let globalState;
let payoutBody;

describe("[Payout] Client Auth Confirm", () => {
  let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);

      if (!globalState.get("payoutsExecution")) {
        shouldContinue = false;
      }

      if (
        !utils.CONNECTOR_LISTS.INCLUDE.CLIENT_AUTH_CONFIRM.includes(
          globalState.get("connectorId")
        )
      ) {
        shouldContinue = false;
      }
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  beforeEach(function () {
    if (!shouldContinue) {
      this.skip();
    }
    payoutBody = Cypress._.cloneDeep(fixtures.createPayoutBody);
  });

  context("[Payout] Client Auth Confirm - Restricted Fields", () => {
    let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payout-for-client-auth-confirm-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["Create"];

      cy.createConfirmPayoutTest(payoutBody, data, false, true, globalState);
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("client-auth-confirm-with-restricted-field-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["ConfirmClientAuthRestrictedFields"];

      cy.confirmPayoutCallTest({}, data, true, globalState);
    });

    it("client-auth-confirm-with-multiple-restricted-fields-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["ConfirmClientAuthMultipleRestrictedFields"];

      cy.confirmPayoutCallTest({}, data, true, globalState);
    });

    it("client-auth-confirm-without-client-secret-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["ConfirmClientAuthMissingClientSecret"];

      cy.confirmPayoutCallTest({}, data, true, globalState);
    });

    it("retrieve-payout-after-client-auth-errors-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["RetrieveAfterClientAuthErrors"];

      cy.retrievePayoutCallTest(globalState, data);
    });

    it("merchant-auth-confirm-with-restricted-fields-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["ConfirmMerchantAuth"];

      cy.confirmPayoutCallTest({}, data, false, globalState);
    });

    it("retrieve-payout-after-merchant-auth-confirm-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["RetrieveAfterConfirm"];

      cy.retrievePayoutCallTest(globalState, data);
    });
  });

  context("[Payout] Client Auth Confirm - Valid Request", () => {
    let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payout-for-client-auth-confirm-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["Create"];

      cy.createConfirmPayoutTest(payoutBody, data, false, true, globalState);
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("client-auth-confirm-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["ConfirmClientAuth"];

      cy.confirmPayoutCallTest({}, data, true, globalState);
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-payout-after-client-auth-confirm-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["RetrieveAfterConfirm"];

      cy.retrievePayoutCallTest(globalState, data);
    });
  });
});
