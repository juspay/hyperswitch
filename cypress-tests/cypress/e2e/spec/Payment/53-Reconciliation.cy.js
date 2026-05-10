// Reconciliation field verification test
import State from "../../../utils/State";
import * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Merchant Reconciliation fields test", () => {
  before("seed global state", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        const connectorId = globalState.get("connectorId");

        if (
          utils.shouldIncludeConnector(
            connectorId,
            utils.CONNECTOR_LISTS.INCLUDE.RECON
          )
        ) {
          skip = true;
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

  context("Merchant retrieve - reconciliation fields", () => {
    it("Retrieve merchant and assert reconciliation fields", () => {
      // Use cy.merchantRetrieveCall to retrieve merchant data
      cy.merchantRetrieveCall(globalState);
      // Use cy.assertReconFields for recon-specific assertions
      cy.assertReconFields(globalState);
      // NOTE: This test only covers default reconciliation field values
      // (is_recon_enabled: false, recon_status: "not_requested")
      // Full recon flow coverage should be a follow-up ticket.
    });
  });
});
