import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;
let connector;

describe("Card - Mandates using Network Transaction Id flow test", () => {
  before(function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        connector = globalState.get("connectorId");
        // Skip the test if the connector is not in the inclusion list
        // This is done because only cybersource is known to support at present
        if (
          utils.shouldIncludeConnector(
            connector,
            utils.CONNECTOR_LISTS.INCLUDE.MANDATES_USING_NTID_PROXY
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

  it("Card - NoThreeDS Create and Confirm Automatic MIT payment flow test", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
      "MITAutoCapture"
    ];

    cy.mitUsingNTID(
      fixtures.ntidConfirmBody,
      data,
      6000,
      true,
      "automatic",
      globalState
    );
  });

  it("Card - NoThreeDS Create and Confirm Manual MIT payment flow test", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
      "MITManualCapture"
    ];

    cy.mitUsingNTID(
      fixtures.ntidConfirmBody,
      data,
      6000,
      true,
      "manual",
      globalState
    );
  });

  it("Card - NoThreeDS Create and Confirm Automatic multiple MITs payment flow test", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
      "MITAutoCapture"
    ];

    cy.mitUsingNTID(
      fixtures.ntidConfirmBody,
      data,
      6000,
      true,
      "automatic",
      globalState
    );

    cy.mitUsingNTID(
      fixtures.ntidConfirmBody,
      data,
      6000,
      true,
      "automatic",
      globalState
    );
  });

  it("Card - NoThreeDS Create and Confirm Manual multiple MITs payment flow test", () => {
    const mitData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["MITManualCapture"];

    cy.mitUsingNTID(
      fixtures.ntidConfirmBody,
      mitData,
      6000,
      true,
      "manual",
      globalState
    );

    const captureData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["Capture"];

    cy.captureCallTest(fixtures.captureBody, captureData, globalState);

    if (!utils.should_continue_further(captureData)) return;

    cy.mitUsingNTID(
      fixtures.ntidConfirmBody,
      mitData,
      6000,
      true,
      "manual",
      globalState
    );

    cy.captureCallTest(fixtures.captureBody, captureData, globalState);
  });

  it("Card - ThreeDS Create and Confirm Automatic multiple MITs payment flow test", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
      "MITAutoCapture"
    ];

    cy.mitUsingNTID(
      fixtures.ntidConfirmBody,
      data,
      6000,
      true,
      "automatic",
      globalState
    );

    cy.mitUsingNTID(
      fixtures.ntidConfirmBody,
      data,
      6000,
      true,
      "automatic",
      globalState
    );
  });

  it("Card - ThreeDS Create and Confirm Manual multiple MITs payment flow", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
      "MITAutoCapture"
    ];

    cy.mitUsingNTID(
      fixtures.ntidConfirmBody,
      data,
      6000,
      true,
      "automatic",
      globalState
    );
  });
});
