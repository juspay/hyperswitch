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

  it("should complete NoThreeDS automatic MIT payment flow", () => {
    const mitData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["MITAutoCapture"];
    cy.mitUsingNTID(fixtures.ntidConfirmBody, mitData, 6000, true, "automatic", globalState);
  });

  it("should complete NoThreeDS manual MIT payment flow", () => {
    const mitData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["MITManualCapture"];
    cy.mitUsingNTID(fixtures.ntidConfirmBody, mitData, 6000, true, "manual", globalState);
  });

  it("should complete NoThreeDS automatic multiple MITs payment flow", () => {
    const mitData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["MITAutoCapture"];

    // First MIT
    cy.mitUsingNTID(fixtures.ntidConfirmBody, mitData, 6000, true, "automatic", globalState);

    // Second MIT
    cy.mitUsingNTID(fixtures.ntidConfirmBody, mitData, 6000, true, "automatic", globalState);
  });

  it("should complete NoThreeDS manual multiple MITs payment flow", () => {
    const mitData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["MITManualCapture"];
    const captureData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Capture"];

    // First MIT
    cy.mitUsingNTID(fixtures.ntidConfirmBody, mitData, 6000, true, "manual", globalState);

    // Capture first MIT
    cy.captureCallTest(fixtures.captureBody, captureData, globalState);

    // Second MIT
    cy.mitUsingNTID(fixtures.ntidConfirmBody, mitData, 6000, true, "manual", globalState);

    // Capture second MIT
    cy.captureCallTest(fixtures.captureBody, captureData, globalState);
  });

  it("should complete ThreeDS automatic multiple MITs payment flow", () => {
    const mitData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["MITAutoCapture"];

    // First MIT
    cy.mitUsingNTID(fixtures.ntidConfirmBody, mitData, 6000, true, "automatic", globalState);

    // Second MIT
    cy.mitUsingNTID(fixtures.ntidConfirmBody, mitData, 6000, true, "automatic", globalState);
  });

  it("should complete ThreeDS manual multiple MITs payment flow", () => {
    const mitData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["MITAutoCapture"];
    cy.mitUsingNTID(fixtures.ntidConfirmBody, mitData, 6000, true, "automatic", globalState);
  });
});