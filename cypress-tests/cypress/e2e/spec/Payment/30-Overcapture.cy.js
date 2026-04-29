import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let connector;
let globalState;

describe("[Payment] Overcapture", () => {
  before(function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        connector = globalState.get("connectorId");

        // Skip the test if the connector is not in the inclusion list
        if (
          utils.shouldIncludeConnector(
            connector,
            utils.CONNECTOR_LISTS.INCLUDE.OVERCAPTURE
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

  context("[Payment] Overcapture Happy Path - Amount Exceeds Authorization", () => {
    let shouldContinue = true;

    it("create-call-test-with-overcapture-enabled", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PaymentIntent"];

      const newData = {
        ...data,
        Request: {
          ...data.Request,
          enable_overcapture: true,
        },
      };

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        newData,
        "no_three_ds",
        "manual",
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("confirm-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSManualCapture"];

      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("overcapture-call-test-amount-exceeds-authorization", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["Overcapture"];

      cy.captureCallTest(fixtures.captureBody, data, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-payment-validate-overcapture-fields", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["Overcapture"];

      cy.retrievePaymentCallTest({ globalState, data }).then(() => {
        const paymentId = globalState.get("paymentID");

        cy.request({
          method: "GET",
          url: `${globalState.get("baseUrl")}/payments/${paymentId}`,
          headers: {
            "Content-Type": "application/json",
            Accept: "application/json",
            "api-key": globalState.get("apiKey"),
          },
        }).then((response) => {
          expect(response.status).to.eq(200);
          expect(response.body.amount).to.eq(6000);
          expect(response.body.amount_captured).to.eq(7000);
          expect(response.body.status).to.eq("succeeded");

          // Validate overcapture specific fields
          if (response.body.overcapture) {
            expect(response.body.overcapture).to.have.property("status");
            expect(response.body.overcapture.status).to.be.oneOf(["Available", "Unavailable"]);
            expect(response.body.overcapture).to.have.property("maximum_amount_capturable");
          }
        });
      });
    });
  });

  context("[Payment] Overcapture Edge Case - Exact Authorized Amount", () => {
    let shouldContinue = true;

    it("create-call-test-with-overcapture-enabled", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PaymentIntent"];

      const newData = {
        ...data,
        Request: {
          ...data.Request,
          enable_overcapture: true,
        },
      };

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        newData,
        "no_three_ds",
        "manual",
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("confirm-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSManualCapture"];

      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("capture-call-test-exact-authorized-amount", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["Capture"];

      cy.captureCallTest(fixtures.captureBody, data, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
  });

  context("[Payment] Overcapture Edge Case - Partial Capture with Overcapture Enabled", () => {
    let shouldContinue = true;

    it("create-call-test-with-overcapture-enabled", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PaymentIntent"];

      const newData = {
        ...data,
        Request: {
          ...data.Request,
          enable_overcapture: true,
        },
      };

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        newData,
        "no_three_ds",
        "manual",
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("confirm-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSManualCapture"];

      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("capture-call-test-partial-capture", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PartialCapture"];

      cy.captureCallTest(fixtures.captureBody, data, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-payment-partial-captured-status", () => {
      const paymentId = globalState.get("paymentID");

      cy.request({
        method: "GET",
        url: `${globalState.get("baseUrl")}/payments/${paymentId}`,
        headers: {
          "Content-Type": "application/json",
          Accept: "application/json",
          "api-key": globalState.get("apiKey"),
        },
      }).then((response) => {
        expect(response.status).to.eq(200);
        expect(response.body.status).to.eq("partially_captured");
        expect(response.body.amount).to.eq(6000);
        expect(response.body.amount_captured).to.eq(2000);
      });
    });
  });

  context("[Payment] Overcapture - Standard Capture Flow (without overcapture flag)", () => {
    let shouldContinue = true;

    it("create-call-test-without-overcapture-flag", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PaymentIntent"];

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "manual",
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("confirm-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSManualCapture"];

      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("capture-call-test-standard-flow", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["Capture"];

      cy.captureCallTest(fixtures.captureBody, data, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-payment-verification", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["Capture"];

      cy.retrievePaymentCallTest({ globalState, data });
    });
  });
});
