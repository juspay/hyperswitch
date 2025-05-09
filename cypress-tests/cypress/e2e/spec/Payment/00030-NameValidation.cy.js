import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails from "../../configs/Payment/Utils";

let globalState;

describe("Name Validation Test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Happy Case: [Valid Name]", () => {
    it("create-intent-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "name_validation"
      ]["HappyCase"];

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
    });
  });

  context("Variant Case: [Invalid Name]", () => {
    it("create-intent-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "name_validation"
      ]["InvalidCase"];

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
    });
  });
});
