import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Client Session Validation", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Valid Client Session - Confirm with SDK Authorization", () => {
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

      cy.confirmWithSdkAuthTest(
        fixtures.confirmBody,
        confirmData,
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
      const paymentIntentID = globalState.get("paymentID");

      cy.request({
        method: "POST",
        url: `${globalState.get("baseUrl")}/payments/${paymentIntentID}`,
        headers: {
          "Content-Type": "application/json",
          "api-key": globalState.get("publishableKey"),
        },
        failOnStatusCode: false,
        body: {
          amount: 7000,
          client_secret: globalState.get("clientSecret"),
        },
      }).then((response) => {
        expect(response.status).to.equal(200);
        expect(response.body.status).to.equal("requires_payment_method");
        if (response.body.sdk_authorization) {
          globalState.set("sdkAuthorization", response.body.sdk_authorization);
        }
        if (response.body.client_secret) {
          globalState.set("clientSecret", response.body.client_secret);
        }
      });
    });

    it("Confirm with old CSI - expect 401", () => {
      const replayData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["ClientSessionReplayConfirm"];

      cy.confirmWithSdkAuthTest(
        fixtures.confirmBody,
        replayData,
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
});
