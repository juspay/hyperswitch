import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let connector;
let globalState;

describe("[Payment] Extend Authorization", () => {
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
            utils.CONNECTOR_LISTS.INCLUDE.EXTEND_AUTHORIZATION
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

  context("[Payment] Extend Authorization - Happy Path - Adyen (Async)", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue || connector !== "adyen") {
        this.skip();
      }
    });

    it("[Payment] Create Payment Intent", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PaymentIntent"];

      const newData = {
        ...data,
        Request: {
          ...data.Request,
          request_extended_authorization: true,
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

    it("[Payment] Confirm Payment Intent", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSManualCapture"];

      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("[Payment] Extend Authorization (Async - becomes processing)", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["ExtendAuthorizationNo3DSManual"];

      cy.extendAuthorizationCallTest(
        fixtures.extendAuthBody,
        data,
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    // Skip capture for Adyen - Extend Authorization is async and becomes processing
    // Cannot capture while payment is in processing state
    it("[Payment] Retrieve Payment Intent (Adyen - Skip Capture)", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["ExtendAuthorizationNo3DSManual"];

      cy.retrievePaymentCallTest({ globalState, data });
    });
  });

  context("[Payment] Extend Authorization - Happy Path - PayPal (Sync)", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue || connector !== "paypal") {
        this.skip();
      }
    });

    it("[Payment] Create Payment Intent", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PaymentIntent"];

      const newData = {
        ...data,
        Request: {
          ...data.Request,
          request_extended_authorization: true,
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

    it("[Payment] Confirm Payment Intent", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSManualCapture"];

      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("[Payment] Extend Authorization (Sync - stays requires_capture)", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["ExtendAuthorizationNo3DSManual"];

      cy.extendAuthorizationCallTest(
        fixtures.extendAuthBody,
        data,
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    // PayPal supports capture after sync extend authorization
    it("[Payment] Capture Payment Intent (PayPal - Can Capture)", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["Capture"];

      cy.captureCallTest(fixtures.captureBody, data, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
  });

  context(
    "[Payment] Extend Authorization - Negative Case - Invalid Status",
    () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("[Payment] Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];

        const newData = {
          ...data,
          Request: {
            ...data.Request,
            request_extended_authorization: true,
          },
        };

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          newData,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("[Payment] Confirm Payment Intent (Auto Capture)", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("[Payment] Extend Authorization - Should Fail (Payment Succeeded)", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ExtendAuthorizationInvalidStatus"];

        cy.extendAuthorizationCallTest(
          fixtures.extendAuthBody,
          data,
          globalState
        );
      });
    }
  );
});
