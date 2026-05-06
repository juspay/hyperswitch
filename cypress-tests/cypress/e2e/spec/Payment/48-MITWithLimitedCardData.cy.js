import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;
let connector;

describe("MIT with Limited Card Data", () => {
  before(function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        connector = globalState.get("connectorId");
        if (
          utils.shouldIncludeConnector(
            connector,
            utils.CONNECTOR_LISTS.INCLUDE.MIT_WITH_LIMITED_CARD_DATA
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

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("MIT with Limited Card Data - Auto Capture", () => {
    it("mit-card-limited-data-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["MITWithLimitedCardData"];
      cy.mitUsingCardWithLimitedData(
        fixtures.cardLimitedDataMITBody,
        data,
        globalState
      );
    });
  });
});
