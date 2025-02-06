import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;
let connector;

describe("Card - Mandates using Network Transaction Id flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      connector = globalState.get("connectorId");
    });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context(
    "Card - NoThreeDS Create and Confirm Automatic MIT payment flow test",
    () => {
      beforeEach(function () {
        if (connector !== "cybersource") {
          this.skip();
        }
      });

      it("Confirm No 3DS MIT", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["MITAutoCapture"];

        cy.mitUsingNTID(
          fixtures.ntidConfirmBody,
          data,
          7000,
          true,
          "automatic",
          globalState
        );
      });
    }
  );

  context(
    "Card - NoThreeDS Create and Confirm Manual MIT payment flow test",
    () => {
      beforeEach(function () {
        if (connector !== "cybersource") {
          this.skip();
        }
      });

      it("Confirm No 3DS MIT", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["MITManualCapture"];

        cy.mitUsingNTID(
          fixtures.ntidConfirmBody,
          data,
          7000,
          true,
          "manual",
          globalState
        );
      });
    }
  );

  context(
    "Card - NoThreeDS Create and Confirm Automatic multiple MITs payment flow test",
    () => {
      beforeEach(function () {
        if (connector !== "cybersource") {
          this.skip();
        }
      });

      it("Confirm No 3DS MIT", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["MITAutoCapture"];

        cy.mitUsingNTID(
          fixtures.ntidConfirmBody,
          data,
          7000,
          true,
          "automatic",
          globalState
        );
      });
      it("Confirm No 3DS MIT", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["MITAutoCapture"];

        cy.mitUsingNTID(
          fixtures.ntidConfirmBody,
          data,
          7000,
          true,
          "automatic",
          globalState
        );
      });
    }
  );

  context(
    "Card - NoThreeDS Create and Confirm Manual multiple MITs payment flow test",
    () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (connector !== "cybersource") {
          this.skip();
        }
      });

      it("Confirm No 3DS MIT 1", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["MITManualCapture"];

        cy.mitUsingNTID(
          fixtures.ntidConfirmBody,
          data,
          6500,
          true,
          "manual",
          globalState
        );
      });

      it("mit-capture-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];

        cy.captureCallTest(fixtures.captureBody, data, 6500, globalState);

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("Confirm No 3DS MIT 2", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["MITManualCapture"];

        cy.mitUsingNTID(
          fixtures.ntidConfirmBody,
          data,
          6500,
          true,
          "manual",
          globalState
        );
      });

      it("mit-capture-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];

        cy.captureCallTest(fixtures.captureBody, data, 6500, globalState);

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });
    }
  );

  context(
    "Card - ThreeDS Create and Confirm Automatic multiple MITs payment flow test",
    () => {
      beforeEach(function () {
        if (connector !== "cybersource") {
          this.skip();
        }
      });

      it("Confirm No 3DS MIT", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["MITAutoCapture"];

        cy.mitUsingNTID(
          fixtures.ntidConfirmBody,
          data,
          7000,
          true,
          "automatic",
          globalState
        );
      });
      it("Confirm No 3DS MIT", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["MITAutoCapture"];

        cy.mitUsingNTID(
          fixtures.ntidConfirmBody,
          data,
          7000,
          true,
          "automatic",
          globalState
        );
      });
    }
  );

  context(
    "Card - ThreeDS Create and Confirm Manual multiple MITs payment flow",
    () => {
      beforeEach(function () {
        if (connector !== "cybersource") {
          this.skip();
        }
      });

      it("Confirm No 3DS MIT", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["MITAutoCapture"];

        cy.mitUsingNTID(
          fixtures.ntidConfirmBody,
          data,
          7000,
          true,
          "automatic",
          globalState
        );
      });
    }
  );
});
