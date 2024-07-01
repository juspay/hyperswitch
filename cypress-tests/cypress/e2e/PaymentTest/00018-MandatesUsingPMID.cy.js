import captureBody from "../../fixtures/capture-flow-body.json";
import citConfirmBody from "../../fixtures/create-mandate-cit.json";
import createPaymentBody from "../../fixtures/create-payment-body";
import pmIdConfirmBody from "../../fixtures/create-pm-id-mit.json";
import State from "../../utils/State";
import getConnectorDetails, * as utils from "../PaymentUtils/utils";

let globalState;

describe("Card - Mandates using Payment Method Id flow test", () => {
  let should_continue = true; // variable that will be used to skip tests if a previous test fails

  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
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
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.createPaymentIntentTest(
          createPaymentBody,
          req_data,
          res_data,
          "no_three_ds",
          "automatic",
          globalState
        );
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("Confirm No 3DS CIT", () => {
        console.log("confirm -> " + globalState.get("connectorId"));
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentMethodIdMandateNo3DSAutoCapture"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        console.log("det -> " + data.card);
        cy.citForMandatesCallTest(
          citConfirmBody,
          req_data,
          res_data,
          7000,
          true,
          "automatic",
          "new_mandate",
          globalState
        );
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("Confirm No 3DS MIT", () => {
        cy.mitUsingPMId(pmIdConfirmBody, 7000, true, "automatic", globalState);
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
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.createPaymentIntentTest(
          createPaymentBody,
          req_data,
          res_data,
          "no_three_ds",
          "manual",
          globalState
        );
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("Confirm No 3DS CIT", () => {
        console.log("confirm -> " + globalState.get("connectorId"));
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentMethodIdMandateNo3DSManualCapture"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        console.log("det -> " + data.card);
        cy.citForMandatesCallTest(
          citConfirmBody,
          req_data,
          res_data,
          6500,
          true,
          "manual",
          "new_mandate",
          globalState
        );
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("cit-capture-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        console.log("det -> " + data.card);
        cy.captureCallTest(captureBody, req_data, res_data, 6500, globalState);
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("Confirm No 3DS MIT", () => {
        cy.mitUsingPMId(pmIdConfirmBody, 7000, true, "automatic", globalState);
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
        console.log("confirm -> " + globalState.get("connectorId"));
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentMethodIdMandateNo3DSAutoCapture"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        console.log("det -> " + data.card);
        cy.citForMandatesCallTest(
          citConfirmBody,
          req_data,
          res_data,
          7000,
          true,
          "automatic",
          "new_mandate",
          globalState
        );
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("Confirm No 3DS MIT", () => {
        cy.mitUsingPMId(pmIdConfirmBody, 7000, true, "automatic", globalState);
      });
      it("Confirm No 3DS MIT", () => {
        cy.mitUsingPMId(pmIdConfirmBody, 7000, true, "automatic", globalState);
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
        console.log("confirm -> " + globalState.get("connectorId"));
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentMethodIdMandateNo3DSManualCapture"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        console.log("det -> " + data.card);
        cy.citForMandatesCallTest(
          citConfirmBody,
          req_data,
          res_data,
          6500,
          true,
          "manual",
          "new_mandate",
          globalState
        );
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("cit-capture-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        console.log("det -> " + data.card);
        cy.captureCallTest(captureBody, req_data, res_data, 6500, globalState);
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("Confirm No 3DS MIT 1", () => {
        cy.mitUsingPMId(pmIdConfirmBody, 6500, true, "manual", globalState);
      });

      it("mit-capture-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        console.log("det -> " + data.card);
        cy.captureCallTest(captureBody, req_data, res_data, 6500, globalState);
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("Confirm No 3DS MIT 2", () => {
        cy.mitUsingPMId(pmIdConfirmBody, 6500, true, "manual", globalState);
      });

      it("mit-capture-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        console.log("det -> " + data.card);
        cy.captureCallTest(captureBody, req_data, res_data, 6500, globalState);
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
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
        console.log("confirm -> " + globalState.get("connectorId"));
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentMethodIdMandate3DSAutoCapture"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        console.log("det -> " + data.card);
        cy.citForMandatesCallTest(
          citConfirmBody,
          req_data,
          res_data,
          7000,
          true,
          "automatic",
          "new_mandate",
          globalState
        );
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("Handle redirection", () => {
        let expected_redirection = citConfirmBody["return_url"];
        cy.handleRedirection(globalState, expected_redirection);
      });

      it("Confirm No 3DS MIT", () => {
        cy.mitUsingPMId(pmIdConfirmBody, 7000, true, "automatic", globalState);
      });
      it("Confirm No 3DS MIT", () => {
        cy.mitUsingPMId(pmIdConfirmBody, 7000, true, "automatic", globalState);
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
        console.log("confirm -> " + globalState.get("connectorId"));
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentMethodIdMandate3DSManualCapture"];
        let req_data = data["Request"];
        let res_data = data["Response"];

        cy.citForMandatesCallTest(
          citConfirmBody,
          req_data,
          res_data,
          6500,
          true,
          "manual",
          "new_mandate",
          globalState
        );
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("Handle redirection", () => {
        let expected_redirection = citConfirmBody["return_url"];
        cy.handleRedirection(globalState, expected_redirection);
      });

      it("cit-capture-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        console.log("det -> " + data.card);
        cy.captureCallTest(captureBody, req_data, res_data, 6500, globalState);
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("Confirm No 3DS MIT", () => {
        cy.mitUsingPMId(pmIdConfirmBody, 7000, true, "automatic", globalState);
      });
    }
  );
});
