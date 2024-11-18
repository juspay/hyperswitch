import * as fixtures from "../../fixtures/imports";
import { validateConfig } from "../../utils/featureFlags";
import State from "../../utils/State";
import getConnectorDetails, * as utils from "../PaymentUtils/Utils";

let globalState;

describe("Card - Mandates using Payment Method Id flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context(
    "Card - NoThreeDS Create and Confirm Automatic CIT and MIT payment flow test",
    () => {
      let should_continue = true;

      beforeEach(function () {
        if (!should_continue) {
          this.skip();
        }
      });

      it("Create No 3DS Payment Intent", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];

        let configs = validateConfig(data["Configs"]);
        let req_data = data["Request"];
        let res_data = data["Response"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          req_data,
          res_data,
          "no_three_ds",
          "automatic",
          globalState,
          configs
        );

        if (should_continue)
          should_continue = utils.should_continue_further(res_data, configs);
      });

      it("Confirm No 3DS CIT", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentMethodIdMandateNo3DSAutoCapture"];

        let configs = validateConfig(data["Configs"]);
        let req_data = data["Request"];
        let res_data = data["Response"];

        cy.citForMandatesCallTest(
          fixtures.citConfirmBody,
          req_data,
          res_data,
          7000,
          true,
          "automatic",
          "new_mandate",
          globalState,
          configs
        );

        if (should_continue)
          should_continue = utils.should_continue_further(res_data, configs);
      });

      it("Confirm No 3DS MIT", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentMethodIdMandateNo3DSAutoCapture"];

        let configs = validateConfig(data["Configs"]);

        cy.mitUsingPMId(
          fixtures.pmIdConfirmBody,
          7000,
          true,
          "automatic",
          globalState,
          configs
        );
      });
    }
  );

  context(
    "Card - NoThreeDS Create and Confirm Manual CIT and MIT payment flow test",
    () => {
      let should_continue = true;

      beforeEach(function () {
        if (!should_continue) {
          this.skip();
        }
      });

      it("Create No 3DS Payment Intent", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];

        let configs = validateConfig(data["Configs"]);
        let req_data = data["Request"];
        let res_data = data["Response"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          req_data,
          res_data,
          "no_three_ds",
          "manual",
          globalState,
          configs
        );

        if (should_continue)
          should_continue = utils.should_continue_further(res_data, configs);
      });

      it("Confirm No 3DS CIT", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentMethodIdMandateNo3DSManualCapture"];

        let configs = validateConfig(data["Configs"]);
        let req_data = data["Request"];
        let res_data = data["Response"];

        cy.citForMandatesCallTest(
          fixtures.citConfirmBody,
          req_data,
          res_data,
          6500,
          true,
          "manual",
          "new_mandate",
          globalState,
          configs
        );

        if (should_continue)
          should_continue = utils.should_continue_further(res_data, configs);
      });

      it("cit-capture-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];

        let configs = validateConfig(data["Configs"]);
        let req_data = data["Request"];
        let res_data = data["Response"];

        cy.captureCallTest(
          fixtures.captureBody,
          req_data,
          res_data,
          6500,
          globalState,
          configs
        );

        if (should_continue)
          should_continue = utils.should_continue_further(res_data, configs);
      });

      it("Confirm No 3DS MIT", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];

        let configs = validateConfig(data["Configs"]);

        cy.mitUsingPMId(
          fixtures.pmIdConfirmBody,
          7000,
          true,
          "automatic",
          globalState,
          configs
        );
      });
    }
  );

  context(
    "Card - NoThreeDS Create + Confirm Automatic CIT and MIT payment flow test",
    () => {
      let should_continue = true;

      beforeEach(function () {
        if (!should_continue) {
          this.skip();
        }
      });

      it("Confirm No 3DS CIT", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentMethodIdMandateNo3DSAutoCapture"];

        let configs = validateConfig(data["Configs"]);
        let req_data = data["Request"];
        let res_data = data["Response"];

        cy.citForMandatesCallTest(
          fixtures.citConfirmBody,
          req_data,
          res_data,
          7000,
          true,
          "automatic",
          "new_mandate",
          globalState,
          configs
        );

        if (should_continue)
          should_continue = utils.should_continue_further(res_data, configs);
      });

      it("Confirm No 3DS MIT", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentMethodIdMandateNo3DSAutoCapture"];

        let configs = validateConfig(data["Configs"]);

        cy.mitUsingPMId(
          fixtures.pmIdConfirmBody,
          7000,
          true,
          "automatic",
          globalState,
          configs
        );
      });
      it("Confirm No 3DS MIT", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentMethodIdMandateNo3DSAutoCapture"];

        let configs = validateConfig(data["Configs"]);

        cy.mitUsingPMId(
          fixtures.pmIdConfirmBody,
          7000,
          true,
          "automatic",
          globalState,
          configs
        );
      });
    }
  );

  context(
    "Card - NoThreeDS Create + Confirm Manual CIT and MIT payment flow test",
    () => {
      let should_continue = true;

      beforeEach(function () {
        if (!should_continue) {
          this.skip();
        }
      });

      it("Confirm No 3DS CIT", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentMethodIdMandateNo3DSManualCapture"];

        let configs = validateConfig(data["Configs"]);
        let req_data = data["Request"];
        let res_data = data["Response"];

        cy.citForMandatesCallTest(
          fixtures.citConfirmBody,
          req_data,
          res_data,
          6500,
          true,
          "manual",
          "new_mandate",
          globalState,
          configs
        );

        if (should_continue)
          should_continue = utils.should_continue_further(res_data, configs);
      });

      it("cit-capture-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];

        let configs = validateConfig(data["Configs"]);
        let req_data = data["Request"];
        let res_data = data["Response"];

        cy.captureCallTest(
          fixtures.captureBody,
          req_data,
          res_data,
          6500,
          globalState,
          configs
        );

        if (should_continue)
          should_continue = utils.should_continue_further(res_data, configs);
      });

      it("Confirm No 3DS MIT 1", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];

        let configs = validateConfig(data["Configs"]);

        cy.mitUsingPMId(
          fixtures.pmIdConfirmBody,
          6500,
          true,
          "manual",
          globalState,
          configs
        );
      });

      it("mit-capture-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];

        let configs = validateConfig(data["Configs"]);
        let req_data = data["Request"];
        let res_data = data["Response"];

        cy.captureCallTest(
          fixtures.captureBody,
          req_data,
          res_data,
          6500,
          globalState,
          configs
        );

        if (should_continue)
          should_continue = utils.should_continue_further(res_data, configs);
      });

      it("Confirm No 3DS MIT 2", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];

        let configs = validateConfig(data["Configs"]);

        cy.mitUsingPMId(
          fixtures.pmIdConfirmBody,
          6500,
          true,
          "manual",
          globalState,
          configs
        );
      });

      it("mit-capture-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];

        let configs = validateConfig(data["Configs"]);
        let req_data = data["Request"];
        let res_data = data["Response"];

        cy.captureCallTest(
          fixtures.captureBody,
          req_data,
          res_data,
          6500,
          globalState,
          configs
        );

        if (should_continue)
          should_continue = utils.should_continue_further(res_data, configs);
      });
    }
  );

  context(
    "Card - ThreeDS Create + Confirm Automatic CIT and MIT payment flow test",
    () => {
      let should_continue = true;

      beforeEach(function () {
        if (!should_continue) {
          this.skip();
        }
      });

      it("Confirm 3DS CIT", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentMethodIdMandate3DSAutoCapture"];

        let configs = validateConfig(data["Configs"]);
        let req_data = data["Request"];
        let res_data = data["Response"];

        cy.citForMandatesCallTest(
          fixtures.citConfirmBody,
          req_data,
          res_data,
          7000,
          true,
          "automatic",
          "new_mandate",
          globalState,
          configs
        );

        if (should_continue)
          should_continue = utils.should_continue_further(res_data, configs);
      });

      it("Handle redirection", () => {
        let expected_redirection = fixtures.citConfirmBody["return_url"];
        cy.handleRedirection(globalState, expected_redirection);
      });

      it("Confirm No 3DS MIT", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentMethodIdMandate3DSAutoCapture"];

        let configs = validateConfig(data["Configs"]);

        cy.mitUsingPMId(
          fixtures.pmIdConfirmBody,
          7000,
          true,
          "automatic",
          globalState,
          configs
        );
      });
      it("Confirm No 3DS MIT", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentMethodIdMandate3DSAutoCapture"];

        let configs = validateConfig(data["Configs"]);

        cy.mitUsingPMId(
          fixtures.pmIdConfirmBody,
          7000,
          true,
          "automatic",
          globalState,
          configs
        );
      });
    }
  );

  context(
    "Card - ThreeDS Create + Confirm Manual CIT and MIT payment flow",
    () => {
      let should_continue = true;

      beforeEach(function () {
        if (!should_continue) {
          this.skip();
        }
      });

      it("Confirm 3DS CIT", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentMethodIdMandate3DSManualCapture"];

        let configs = validateConfig(data["Configs"]);
        let req_data = data["Request"];
        let res_data = data["Response"];

        cy.citForMandatesCallTest(
          fixtures.citConfirmBody,
          req_data,
          res_data,
          6500,
          true,
          "manual",
          "new_mandate",
          globalState,
          configs
        );

        if (should_continue)
          should_continue = utils.should_continue_further(res_data, configs);
      });

      it("Handle redirection", () => {
        let expected_redirection = fixtures.citConfirmBody["return_url"];
        cy.handleRedirection(globalState, expected_redirection);
      });

      it("cit-capture-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];

        let configs = validateConfig(data["Configs"]);
        let req_data = data["Request"];
        let res_data = data["Response"];

        cy.captureCallTest(
          fixtures.captureBody,
          req_data,
          res_data,
          6500,
          globalState,
          configs
        );

        if (should_continue)
          should_continue = utils.should_continue_further(res_data, configs);
      });

      it("Confirm No 3DS MIT", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];

        let configs = validateConfig(data["Configs"]);

        cy.mitUsingPMId(
          fixtures.pmIdConfirmBody,
          7000,
          true,
          "automatic",
          globalState,
          configs
        );
      });
    }
  );
});
