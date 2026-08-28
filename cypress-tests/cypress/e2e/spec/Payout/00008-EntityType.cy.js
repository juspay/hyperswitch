import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import * as utils from "../../configs/Payout/Utils";

let globalState;
let payoutBody;

describe("[Payout] Entity Type", () => {
  let shouldContinue = true;

  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);

      if (!globalState.get("payoutsExecution")) {
        shouldContinue = false;
      }

      if (
        !utils.CONNECTOR_LISTS.INCLUDE.ENTITY_TYPE.includes(
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

  utils.ENTITY_TYPE_LIST.forEach(({ key, name }) => {
    context(`[Payout] Entity Type - ${name}`, () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it(`create-payout-with-entity-type-${name.toLowerCase()}-test`, () => {
        const data = utils.getConnectorDetails(globalState.get("connectorId"))[
          "bank_transfer_pm"
        ]["sepa_bank_transfer"][key];

        cy.createConfirmPayoutTest(payoutBody, data, true, true, globalState);
        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("retrieve-payout-call-test", () => {
        cy.retrievePayoutCallTest(globalState);
      });
    });
  });

  context("[Payout] Entity Type - Default (Individual)", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payout-with-default-entity-type-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["sepa_bank_transfer"]["EntityTypeDefault"];

      delete payoutBody.entity_type;

      cy.createConfirmPayoutTest(payoutBody, data, true, true, globalState);
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-payout-call-test", () => {
      cy.retrievePayoutCallTest(globalState);
    });
  });

  context("[Payout] Entity Type - Invalid", () => {
    const shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payout-with-invalid-entity-type-test", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["sepa_bank_transfer"]["EntityTypeInvalid"];

      cy.createConfirmPayoutTest(payoutBody, data, true, true, globalState);
    });
  });
});
