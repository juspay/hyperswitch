import * as fixtures from "../../fixtures/imports";
import State from "../../utils/State";
import getConnectorDetails, * as utils from "../PaymentUtils/Utils";

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
      let should_continue = true;

      beforeEach(function () {
        if (!should_continue || connector !== "cybersource") {
          this.skip();
        }
      });

      it("Confirm No 3DS MIT", () => {
        cy.mitUsingNTID(
          fixtures.ntidConfirmBody,
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
      let should_continue = true;

      beforeEach(function () {
        if (!should_continue || connector !== "cybersource") {
          this.skip();
        }
      });

      it("Confirm No 3DS MIT", () => {
        cy.mitUsingNTID(
          fixtures.ntidConfirmBody,
          7000,
          true,
          "automatic",
          globalState
        );
      });
    }
  );

  context(
    "Card - NoThreeDS Create and Confirm Automatic multiple MITs payment flow test",
    () => {
      let should_continue = true;

      beforeEach(function () {
        if (!should_continue || connector !== "cybersource") {
          this.skip();
        }
      });

      it("Confirm No 3DS MIT", () => {
        cy.mitUsingNTID(
          fixtures.ntidConfirmBody,
          7000,
          true,
          "automatic",
          globalState
        );
      });
      it("Confirm No 3DS MIT", () => {
        cy.mitUsingNTID(
          fixtures.ntidConfirmBody,
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
      let should_continue = true;

      beforeEach(function () {
        if (!should_continue || connector !== "cybersource") {
          this.skip();
        }
      });

      it("Confirm No 3DS MIT 1", () => {
        cy.mitUsingNTID(
          fixtures.ntidConfirmBody,
          6500,
          true,
          "manual",
          globalState
        );
      });

      it("mit-capture-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];
        let req_data = data["Request"];
        let res_data = data["Response"];

        cy.captureCallTest(
          fixtures.captureBody,
          req_data,
          res_data,
          6500,
          globalState
        );
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("Confirm No 3DS MIT 2", () => {
        cy.mitUsingNTID(
          fixtures.ntidConfirmBody,
          6500,
          true,
          "manual",
          globalState
        );
      });

      it("mit-capture-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];
        let req_data = data["Request"];
        let res_data = data["Response"];

        cy.captureCallTest(
          fixtures.captureBody,
          req_data,
          res_data,
          6500,
          globalState
        );
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });
    }
  );

  context(
    "Card - ThreeDS Create and Confirm Automatic multiple MITs payment flow test",
    () => {
      let should_continue = true;

      beforeEach(function () {
        if (!should_continue || connector !== "cybersource") {
          this.skip();
        }
      });

      it("Confirm No 3DS MIT", () => {
        cy.mitUsingNTID(
          fixtures.ntidConfirmBody,
          7000,
          true,
          "automatic",
          globalState
        );
      });
      it("Confirm No 3DS MIT", () => {
        cy.mitUsingNTID(
          fixtures.ntidConfirmBody,
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
      let should_continue = true;

      beforeEach(function () {
        if (!should_continue || connector !== "cybersource") {
          this.skip();
        }
      });

      it("Confirm No 3DS MIT", () => {
        cy.mitUsingNTID(
          fixtures.ntidConfirmBody,
          7000,
          true,
          "automatic",
          globalState
        );
      });
    }
  );
});
