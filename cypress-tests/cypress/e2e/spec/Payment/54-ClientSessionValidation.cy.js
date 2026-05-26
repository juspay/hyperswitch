import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import { cardCreditEnabled } from "../../configs/Payment/Commons";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Client Session Validation", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  before("enable client session validation", () => {
    cy.setupConfigs(globalState, "client_session_validation_enabled", "true");
  });

  after("cleanup and flush global state", () => {
    cy.setupConfigs(globalState, "client_session_validation_enabled", "false");
    cy.task("setGlobalState", globalState.data);
  });

  context("Valid Client Session - Confirm with SDK Authorization", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("Create Business Profile", () => {
      cy.createBusinessProfileTest(
        fixtures.businessProfile.bpCreate,
        globalState
      );
    });

    it("connector-create-call-test", () => {
      cy.createConnectorCallTest(
        "payment_processor",
        fixtures.createConnectorBody,
        cardCreditEnabled,
        globalState
      );
    });

    it("Create Payment Intent", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PaymentIntent"];

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Confirm with valid SDK Authorization", () => {
      const confirmData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["ClientSessionValidConfirm"];

      cy.confirmWithSdkAuthTest(
        fixtures.confirmBody,
        confirmData,
        true,
        globalState
      );

      if (shouldContinue)
        shouldContinue = utils.should_continue_further(confirmData);
    });

    it("Retrieve Payment", () => {
      const confirmData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["ClientSessionValidConfirm"];

      cy.retrievePaymentCallTest({ globalState, data: confirmData });
    });
  });

  context("Invalid Client Session - Confirm with tampered CSI", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("Create Payment Intent", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PaymentIntent"];

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Confirm with invalid client_session_id - expect 401", () => {
      const confirmData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["ClientSessionInvalidConfirm"];

      const invalidConfirmData = {
        ...confirmData,
        ResponseCustom: confirmData.ResponseCustom || confirmData.Response,
      };

      cy.confirmWithSdkAuthTest(
        fixtures.confirmBody,
        invalidConfirmData,
        true,
        globalState,
        "invalid_session"
      );
    });
  });

  context(
    "Missing Client Session - Confirm without CSI (legacy fallback)",
    () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("Confirm without client_session_id - legacy fallback", () => {
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ClientSessionValidConfirm"];

        cy.confirmWithSdkAuthTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState,
          "missing_session"
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(confirmData);
      });

      it("Retrieve Payment", () => {
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ClientSessionValidConfirm"];

        cy.retrievePaymentCallTest({ globalState, data: confirmData });
      });
    }
  );

  context("Replay Client Session - Confirm with old CSI after update", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("Create Payment Intent", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PaymentIntent"];

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Save old SDK Authorization", () => {
      cy.wrap(null).then(() => {
        const oldSdkAuth = globalState.get("sdkAuthorization");
        globalState.set("oldSdkAuthorization", oldSdkAuth);
      });
    });

    it("Update Payment Intent - triggers session recreation", () => {
      const updateData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["ClientSessionUpdatePayment"];

      cy.updatePaymentIntentTest({}, updateData, globalState);

      if (shouldContinue)
        shouldContinue = utils.should_continue_further(updateData);
    });

    it("Confirm with old CSI - expect 401", () => {
      const replayData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["ClientSessionReplayConfirm"];

      const replayConfirmData = {
        ...replayData,
        ResponseCustom: replayData.ResponseCustom || replayData.Response,
      };

      cy.confirmWithSdkAuthTest(
        fixtures.confirmBody,
        replayConfirmData,
        true,
        globalState,
        globalState.get("oldSdkAuthorization")
      );
    });

    it("Confirm with new CSI - expect 200", () => {
      const confirmData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["ClientSessionValidConfirm"];

      cy.confirmWithSdkAuthTest(
        fixtures.confirmBody,
        confirmData,
        true,
        globalState
      );

      if (shouldContinue)
        shouldContinue = utils.should_continue_further(confirmData);
    });

    it("Retrieve Payment", () => {
      const confirmData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["ClientSessionValidConfirm"];

      cy.retrievePaymentCallTest({ globalState, data: confirmData });
    });
  });

  context(
    "Toggle client_session_validation_enabled - disabled allows invalid CSI",
    () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("Disable client_session_validation_enabled", () => {
        cy.setupConfigs(
          globalState,
          "client_session_validation_enabled",
          "false"
        );
      });

      it("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("Confirm with invalid CSI - should succeed when validation disabled", () => {
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ClientSessionValidConfirm"];

        cy.confirmWithSdkAuthTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState,
          "invalid_session"
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(confirmData);
      });

      it("Re-enable client_session_validation_enabled", () => {
        cy.setupConfigs(
          globalState,
          "client_session_validation_enabled",
          "true"
        );
      });

      it("Create another Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("Confirm with invalid CSI - should fail when validation enabled", () => {
        const invalidData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ClientSessionInvalidConfirm"];

        const invalidConfirmData = {
          ...invalidData,
          ResponseCustom: invalidData.ResponseCustom || invalidData.Response,
        };

        cy.confirmWithSdkAuthTest(
          fixtures.confirmBody,
          invalidConfirmData,
          true,
          globalState,
          "invalid_session"
        );
      });
    }
  );
});
// CI trigger v3
