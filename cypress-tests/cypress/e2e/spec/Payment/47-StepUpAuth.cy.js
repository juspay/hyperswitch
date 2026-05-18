import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, {
  CONNECTOR_LISTS,
  shouldIncludeConnector,
} from "../../configs/Payment/Utils";
import * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Step-Up Auth payment flow test", () => {
  before("seed global state", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);

        const baseUrl = globalState.get("baseUrl") || "";
        if (baseUrl.includes("localhost")) {
          skip = true;
          return;
        }

        if (
          shouldIncludeConnector(
            globalState.get("connectorId"),
            CONNECTOR_LISTS.INCLUDE.STEP_UP_AUTH
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

  after("flush global state", () => {
    if (globalState) {
      cy.task("setGlobalState", globalState.data);
    }
  });

  context(
    "Step-Up Auth setup - create auth processor and update profile",
    () => {
      let shouldContinue = true;

      before(function () {
        if (!globalState.get("profileId")) {
          shouldContinue = false;
        }
      });

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      afterEach("flush global state", () => {
        if (!globalState.get("authProcessorConnectorId")) {
          shouldContinue = false;
        }
        cy.task("setGlobalState", globalState.data);
      });

      it("create authentication processor connector", () => {
        const createConnectorBody = { ...fixtures.createConnectorBody };
        createConnectorBody.connector_name = "netcetera";

        cy.createAuthenticationProcessorConnectorTest(
          createConnectorBody,
          globalState
        );
      });

      it("update business profile with auth connector", () => {
        const bpUpdateAuthConnector = {
          authentication_connector_details: {
            authentication_connectors: ["netcetera"],
            three_ds_requestor_url: "https://example.com",
          },
        };
        cy.UpdateBusinessProfileTest(
          bpUpdateAuthConnector,
          false,
          false,
          false,
          false,
          false,
          globalState
        );
      });
    }
  );

  context(
    "Step-Up Auth happy path - create, confirm, authenticate and retrieve",
    () => {
      let shouldContinue = true;

      before(function () {
        if (!globalState.get("authProcessorConnectorId")) {
          shouldContinue = false;
        }
      });

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      afterEach("flush global state", () => {
        cy.task("setGlobalState", globalState.data);
      });

      it("create payment intent with three_ds", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "step_up_auth"
        ]["PaymentIntentOnly"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "three_ds",
          "automatic",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      it("confirm payment with three_ds card", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: confirm payment with three_ds card"
          );
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "step_up_auth"
        ]["ConfirmPayment"];

        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "three_ds",
          "automatic",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      it("call 3ds authentication endpoint", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: call 3ds authentication endpoint");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "step_up_auth"
        ]["ThreeDSAuthentication"];

        cy.threeDSAuthenticationCallTest(
          fixtures.threeDSAuthenticationBody,
          data,
          globalState
        );
      });

      it("retrieve payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: retrieve payment");
          return;
        }
        cy.retrievePaymentCallTest({ globalState });
      });
    }
  );

  context("Step-Up Auth negative - 3ds auth without confirmed payment", () => {
    let shouldContinue = true;

    before(function () {
      if (!globalState.get("authProcessorConnectorId")) {
        shouldContinue = false;
      }
    });

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    afterEach("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("create payment intent with three_ds only", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "step_up_auth"
      ]["PaymentIntentOnly"];

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );

      if (!utils.should_continue_further(data)) {
        shouldContinue = false;
      }
    });

    it("call 3ds authentication - should fail with IR_04", () => {
      if (!shouldContinue) {
        cy.task(
          "cli_log",
          "Skipping step: call 3ds authentication - should fail with IR_04"
        );
        return;
      }
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "step_up_auth"
      ]["ThreeDSAuthenticationUnconfirmed"];

      cy.threeDSAuthenticationCallTest(
        fixtures.threeDSAuthenticationBody,
        data,
        globalState
      );
    });
  });

  context("Step-Up Auth with merchant codes from business profile", () => {
    let shouldContinue = true;

    before(function () {
      if (!globalState.get("authProcessorConnectorId")) {
        shouldContinue = false;
      }

      if (shouldContinue) {
        const bpUpdateMerchantCodes = {
          merchant_country_code: "840",
          merchant_category_code: "5411",
        };
        cy.UpdateBusinessProfileTest(
          bpUpdateMerchantCodes,
          false,
          false,
          false,
          false,
          false,
          globalState
        );
      }
    });

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    afterEach("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("create payment intent with three_ds", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "step_up_auth"
      ]["PaymentIntentOnly"];

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );

      if (!utils.should_continue_further(data)) {
        shouldContinue = false;
      }
    });

    it("confirm payment with three_ds card", () => {
      if (!shouldContinue) {
        cy.task("cli_log", "Skipping step: confirm payment with three_ds card");
        return;
      }
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "step_up_auth"
      ]["StepUpAuthWithMerchantCodes"];

      cy.createConfirmPaymentTest(
        fixtures.createConfirmPaymentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );

      if (!utils.should_continue_further(data)) {
        shouldContinue = false;
      }
    });

    it("call 3ds authentication endpoint", () => {
      if (!shouldContinue) {
        cy.task("cli_log", "Skipping step: call 3ds authentication endpoint");
        return;
      }
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "step_up_auth"
      ]["ThreeDSAuthenticationWithMerchantCodes"];

      cy.threeDSAuthenticationCallTest(
        fixtures.threeDSAuthenticationBody,
        data,
        globalState
      );
    });

    it("retrieve payment", () => {
      if (!shouldContinue) {
        cy.task("cli_log", "Skipping step: retrieve payment");
        return;
      }
      cy.retrievePaymentCallTest({ globalState });
    });
  });

  context(
    "Step-Up Auth Visa frictionless - create, confirm, authenticate and retrieve",
    () => {
      let shouldContinue = true;

      before(function () {
        if (!globalState.get("authProcessorConnectorId")) {
          shouldContinue = false;
        }
      });

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      afterEach("flush global state", () => {
        cy.task("setGlobalState", globalState.data);
      });

      it("create payment intent with three_ds for visa frictionless", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "step_up_auth"
        ]["PaymentIntentOnly"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "three_ds",
          "automatic",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      it("confirm payment with visa frictionless card", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: confirm payment with visa frictionless card"
          );
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "step_up_auth"
        ]["ConfirmPaymentVisaFrictionless"];

        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "three_ds",
          "automatic",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      it("call 3ds authentication endpoint for visa frictionless", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: call 3ds authentication endpoint for visa frictionless"
          );
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "step_up_auth"
        ]["ThreeDSAuthentication"];

        cy.threeDSAuthenticationCallTest(
          fixtures.threeDSAuthenticationBody,
          data,
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      it("authorize payment after visa frictionless authentication", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: authorize payment after visa frictionless authentication"
          );
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "step_up_auth"
        ]["AuthorizeAfterFrictionlessAuth"];

        cy.authorizeViaThreeDsAuthorizeUrlTest(data, globalState);
      });

      it("retrieve payment for visa frictionless", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: retrieve payment for visa frictionless"
          );
          return;
        }
        cy.retrievePaymentCallTest({ globalState });
      });
    }
  );

  context(
    "Step-Up Auth Mastercard challenge - create, confirm, authenticate and retrieve",
    () => {
      let shouldContinue = true;

      before(function () {
        if (!globalState.get("authProcessorConnectorId")) {
          shouldContinue = false;
        }
      });

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      afterEach("flush global state", () => {
        cy.task("setGlobalState", globalState.data);
      });

      it("create payment intent with three_ds for mastercard challenge", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "step_up_auth"
        ]["PaymentIntentOnly"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "three_ds",
          "automatic",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      it("confirm payment with mastercard challenge card", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: confirm payment with mastercard challenge card"
          );
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "step_up_auth"
        ]["ConfirmPaymentMastercardChallenge"];

        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "three_ds",
          "automatic",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      it("call 3ds authentication endpoint for mastercard challenge", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: call 3ds authentication endpoint for mastercard challenge"
          );
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "step_up_auth"
        ]["ThreeDSAuthentication"];

        cy.threeDSAuthenticationCallTest(
          fixtures.threeDSAuthenticationBody,
          data,
          globalState
        );
      });

      it("retrieve payment for mastercard challenge", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: retrieve payment for mastercard challenge"
          );
          return;
        }
        cy.retrievePaymentCallTest({ globalState });
      });
    }
  );
});
