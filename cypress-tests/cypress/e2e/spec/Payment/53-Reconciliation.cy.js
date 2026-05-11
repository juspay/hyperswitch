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
      cy.request({
        method: "GET",
        url: `${globalState.get("baseUrl")}/accounts/${globalState.get("merchantId")}`,
        headers: {
          Accept: "application/json",
          "Content-Type": "application/json",
          "api-key": globalState.get("adminApiKey"),
        },
        failOnStatusCode: false,
      }).then((response) => {
        cy.assertReconFields(response);
      });
    });
  });
});
