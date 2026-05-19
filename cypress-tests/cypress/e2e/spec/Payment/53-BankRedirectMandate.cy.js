import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Bank Redirect Mandate tests", () => {
  before(function () {
    let skip = false;
    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        if (
          !utils.CONNECTOR_LISTS.INCLUDE.BANK_REDIRECT_MANDATE.includes(
            globalState.get("connectorId")
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

  context("iDEAL - Single-use Mandate CIT and MIT", () => {
    it("Create Intent -> Confirm CIT -> Handle Redirect -> Retrieve CIT -> MIT -> Retrieve MIT", () => {
      let shouldContinue = true;
      let mandateCreated = false;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["PaymentIntentOffSession"]("Ideal");
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

      cy.step("Confirm CIT for single-use mandate", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Confirm CIT for single-use mandate"
          );
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["IdealMandateSingleUseNo3DSAutoCapture"];
        cy.confirmBankRedirectCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle Bank Redirect Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle Bank Redirect Redirection");
          return;
        }
        const expected_redirection = fixtures.confirmBody["return_url"];
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handleBankRedirectRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      });

      cy.step("Retrieve CIT and extract mandate_id", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Retrieve CIT and extract mandate_id"
          );
          return;
        }
        cy.request({
          method: "GET",
          url: `${globalState.get("baseUrl")}/payments/${globalState.get("paymentID")}?force_sync=true`,
          headers: {
            "Content-Type": "application/json",
            "api-key": globalState.get("apiKey"),
          },
          failOnStatusCode: false,
        }).then((response) => {
          expect(response.status).to.equal(200);
          if (response.body.mandate_id) {
            globalState.set("mandateId", response.body.mandate_id);
            mandateCreated = true;
          } else {
            cy.task(
              "cli_log",
              "No mandate_id returned — skipping MIT and subsequent steps"
            );
            shouldContinue = false;
          }
        });
      });

      cy.step("MIT for single-use mandate", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: MIT for single-use mandate");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["IdealMITAutoCapture"];
        const { Request: reqData, Response: resData } = data;

        const requestBody = { ...fixtures.mitConfirmBody };
        for (const key in reqData) {
          requestBody[key] = reqData[key];
        }
        requestBody.amount = 6500;
        requestBody.confirm = true;
        requestBody.capture_method = "automatic";
        requestBody.customer_id = globalState.get("customerId");
        requestBody.mandate_id = globalState.get("mandateId");

        cy.request({
          method: "POST",
          url: `${globalState.get("baseUrl")}/payments`,
          headers: {
            "Content-Type": "application/json",
            "api-key": globalState.get("apiKey"),
          },
          failOnStatusCode: false,
          body: requestBody,
        }).then((response) => {
          globalState.set("paymentID", response.body.payment_id);
          expect(response.status).to.equal(resData.status);
          expect(response.body.status).to.equal(resData.body.status);
        });

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve MIT Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve MIT Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["IdealMITAutoCapture"];
        cy.retrievePaymentCallTest({ globalState, data });
      });

      cy.step("List Mandates", () => {
        if (!mandateCreated) {
          cy.task(
            "cli_log",
            "Skipping step: List Mandates - no mandate created"
          );
          return;
        }
        cy.listMandateCallTest(globalState);
      });

      cy.step("Revoke Mandate", () => {
        if (!mandateCreated) {
          cy.task(
            "cli_log",
            "Skipping step: Revoke Mandate - no mandate created"
          );
          return;
        }
        cy.revokeMandateCallTest(globalState);
      });
    });
  });

  context("iDEAL - Multi-use Mandate CIT and MIT", () => {
    it("Create Intent -> Confirm CIT -> Handle Redirect -> Retrieve CIT -> MIT -> Retrieve MIT", () => {
      let shouldContinue = true;
      let mandateCreated = false;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["PaymentIntentOffSession"]("Ideal");
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

      cy.step("Confirm CIT for multi-use mandate", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Confirm CIT for multi-use mandate"
          );
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["IdealMandateMultiUseNo3DSAutoCapture"];
        cy.confirmBankRedirectCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle Bank Redirect Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle Bank Redirect Redirection");
          return;
        }
        const expected_redirection = fixtures.confirmBody["return_url"];
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handleBankRedirectRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      });

      cy.step("Retrieve CIT and extract mandate_id", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Retrieve CIT and extract mandate_id"
          );
          return;
        }
        cy.request({
          method: "GET",
          url: `${globalState.get("baseUrl")}/payments/${globalState.get("paymentID")}?force_sync=true`,
          headers: {
            "Content-Type": "application/json",
            "api-key": globalState.get("apiKey"),
          },
          failOnStatusCode: false,
        }).then((response) => {
          expect(response.status).to.equal(200);
          if (response.body.mandate_id) {
            globalState.set("mandateId", response.body.mandate_id);
            mandateCreated = true;
          } else {
            cy.task(
              "cli_log",
              "No mandate_id returned — skipping MIT and subsequent steps"
            );
            shouldContinue = false;
          }
        });
      });

      cy.step("MIT for multi-use mandate", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: MIT for multi-use mandate");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["IdealMITAutoCapture"];
        const { Request: reqData, Response: resData } = data;

        const requestBody = { ...fixtures.mitConfirmBody };
        for (const key in reqData) {
          requestBody[key] = reqData[key];
        }
        requestBody.amount = 6500;
        requestBody.confirm = true;
        requestBody.capture_method = "automatic";
        requestBody.customer_id = globalState.get("customerId");
        requestBody.mandate_id = globalState.get("mandateId");

        cy.request({
          method: "POST",
          url: `${globalState.get("baseUrl")}/payments`,
          headers: {
            "Content-Type": "application/json",
            "api-key": globalState.get("apiKey"),
          },
          failOnStatusCode: false,
          body: requestBody,
        }).then((response) => {
          globalState.set("paymentID", response.body.payment_id);
          expect(response.status).to.equal(resData.status);
          expect(response.body.status).to.equal(resData.body.status);
        });

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve MIT Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve MIT Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["IdealMITAutoCapture"];
        cy.retrievePaymentCallTest({ globalState, data });
      });

      cy.step("List Mandates", () => {
        if (!mandateCreated) {
          cy.task(
            "cli_log",
            "Skipping step: List Mandates - no mandate created"
          );
          return;
        }
        cy.listMandateCallTest(globalState);
      });

      cy.step("Revoke Mandate", () => {
        if (!mandateCreated) {
          cy.task(
            "cli_log",
            "Skipping step: Revoke Mandate - no mandate created"
          );
          return;
        }
        cy.revokeMandateCallTest(globalState);
      });
    });
  });

  context("EPS - Single-use Mandate CIT and MIT", () => {
    it("Create Intent -> Confirm CIT -> Handle Redirect -> Retrieve CIT -> MIT -> Retrieve MIT", () => {
      let shouldContinue = true;
      let mandateCreated = false;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["PaymentIntentOffSession"]("Eps");
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

      cy.step("Confirm CIT for single-use mandate", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Confirm CIT for single-use mandate"
          );
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["EpsMandateSingleUseNo3DSAutoCapture"];
        cy.confirmBankRedirectCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle Bank Redirect Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle Bank Redirect Redirection");
          return;
        }
        const expected_redirection = fixtures.confirmBody["return_url"];
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handleBankRedirectRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      });

      cy.step("Retrieve CIT and extract mandate_id", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Retrieve CIT and extract mandate_id"
          );
          return;
        }
        cy.request({
          method: "GET",
          url: `${globalState.get("baseUrl")}/payments/${globalState.get("paymentID")}?force_sync=true`,
          headers: {
            "Content-Type": "application/json",
            "api-key": globalState.get("apiKey"),
          },
          failOnStatusCode: false,
        }).then((response) => {
          expect(response.status).to.equal(200);
          if (response.body.mandate_id) {
            globalState.set("mandateId", response.body.mandate_id);
            mandateCreated = true;
          } else {
            cy.task(
              "cli_log",
              "No mandate_id returned — skipping MIT and subsequent steps"
            );
            shouldContinue = false;
          }
        });
      });

      cy.step("MIT for single-use mandate", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: MIT for single-use mandate");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["EpsMITAutoCapture"];
        const { Request: reqData, Response: resData } = data;

        const requestBody = { ...fixtures.mitConfirmBody };
        for (const key in reqData) {
          requestBody[key] = reqData[key];
        }
        requestBody.amount = 6500;
        requestBody.confirm = true;
        requestBody.capture_method = "automatic";
        requestBody.customer_id = globalState.get("customerId");
        requestBody.mandate_id = globalState.get("mandateId");

        cy.request({
          method: "POST",
          url: `${globalState.get("baseUrl")}/payments`,
          headers: {
            "Content-Type": "application/json",
            "api-key": globalState.get("apiKey"),
          },
          failOnStatusCode: false,
          body: requestBody,
        }).then((response) => {
          globalState.set("paymentID", response.body.payment_id);
          expect(response.status).to.equal(resData.status);
          expect(response.body.status).to.equal(resData.body.status);
        });

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve MIT Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve MIT Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["EpsMITAutoCapture"];
        cy.retrievePaymentCallTest({ globalState, data });
      });

      cy.step("List Mandates", () => {
        if (!mandateCreated) {
          cy.task(
            "cli_log",
            "Skipping step: List Mandates - no mandate created"
          );
          return;
        }
        cy.listMandateCallTest(globalState);
      });

      cy.step("Revoke Mandate", () => {
        if (!mandateCreated) {
          cy.task(
            "cli_log",
            "Skipping step: Revoke Mandate - no mandate created"
          );
          return;
        }
        cy.revokeMandateCallTest(globalState);
      });
    });
  });

  context("EPS - Multi-use Mandate CIT and MIT", () => {
    it("Create Intent -> Confirm CIT -> Handle Redirect -> Retrieve CIT -> MIT -> Retrieve MIT", () => {
      let shouldContinue = true;
      let mandateCreated = false;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["PaymentIntentOffSession"]("Eps");
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

      cy.step("Confirm CIT for multi-use mandate", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Confirm CIT for multi-use mandate"
          );
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["EpsMandateMultiUseNo3DSAutoCapture"];
        cy.confirmBankRedirectCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle Bank Redirect Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle Bank Redirect Redirection");
          return;
        }
        const expected_redirection = fixtures.confirmBody["return_url"];
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handleBankRedirectRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      });

      cy.step("Retrieve CIT and extract mandate_id", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Retrieve CIT and extract mandate_id"
          );
          return;
        }
        cy.request({
          method: "GET",
          url: `${globalState.get("baseUrl")}/payments/${globalState.get("paymentID")}?force_sync=true`,
          headers: {
            "Content-Type": "application/json",
            "api-key": globalState.get("apiKey"),
          },
          failOnStatusCode: false,
        }).then((response) => {
          expect(response.status).to.equal(200);
          if (response.body.mandate_id) {
            globalState.set("mandateId", response.body.mandate_id);
            mandateCreated = true;
          } else {
            cy.task(
              "cli_log",
              "No mandate_id returned — skipping MIT and subsequent steps"
            );
            shouldContinue = false;
          }
        });
      });

      cy.step("MIT for multi-use mandate", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: MIT for multi-use mandate");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["EpsMITAutoCapture"];
        const { Request: reqData, Response: resData } = data;

        const requestBody = { ...fixtures.mitConfirmBody };
        for (const key in reqData) {
          requestBody[key] = reqData[key];
        }
        requestBody.amount = 6500;
        requestBody.confirm = true;
        requestBody.capture_method = "automatic";
        requestBody.customer_id = globalState.get("customerId");
        requestBody.mandate_id = globalState.get("mandateId");

        cy.request({
          method: "POST",
          url: `${globalState.get("baseUrl")}/payments`,
          headers: {
            "Content-Type": "application/json",
            "api-key": globalState.get("apiKey"),
          },
          failOnStatusCode: false,
          body: requestBody,
        }).then((response) => {
          globalState.set("paymentID", response.body.payment_id);
          expect(response.status).to.equal(resData.status);
          expect(response.body.status).to.equal(resData.body.status);
        });

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve MIT Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve MIT Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["EpsMITAutoCapture"];
        cy.retrievePaymentCallTest({ globalState, data });
      });

      cy.step("List Mandates", () => {
        if (!mandateCreated) {
          cy.task(
            "cli_log",
            "Skipping step: List Mandates - no mandate created"
          );
          return;
        }
        cy.listMandateCallTest(globalState);
      });

      cy.step("Revoke Mandate", () => {
        if (!mandateCreated) {
          cy.task(
            "cli_log",
            "Skipping step: Revoke Mandate - no mandate created"
          );
          return;
        }
        cy.revokeMandateCallTest(globalState);
      });
    });
  });

  context("Giropay - Single-use Mandate CIT and MIT", () => {
    it("Create Intent -> Confirm CIT -> Handle Redirect -> Retrieve CIT -> MIT -> Retrieve MIT", () => {
      let shouldContinue = true;
      let mandateCreated = false;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["PaymentIntentOffSession"]("Giropay");
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

      cy.step("Confirm CIT for single-use mandate", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Confirm CIT for single-use mandate"
          );
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["GiropayMandateSingleUseNo3DSAutoCapture"];
        cy.confirmBankRedirectCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle Bank Redirect Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle Bank Redirect Redirection");
          return;
        }
        const expected_redirection = fixtures.confirmBody["return_url"];
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handleBankRedirectRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      });

      cy.step("Retrieve CIT and extract mandate_id", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Retrieve CIT and extract mandate_id"
          );
          return;
        }
        cy.request({
          method: "GET",
          url: `${globalState.get("baseUrl")}/payments/${globalState.get("paymentID")}?force_sync=true`,
          headers: {
            "Content-Type": "application/json",
            "api-key": globalState.get("apiKey"),
          },
          failOnStatusCode: false,
        }).then((response) => {
          expect(response.status).to.equal(200);
          if (response.body.mandate_id) {
            globalState.set("mandateId", response.body.mandate_id);
            mandateCreated = true;
          } else {
            cy.task(
              "cli_log",
              "No mandate_id returned — skipping MIT and subsequent steps"
            );
            shouldContinue = false;
          }
        });
      });

      cy.step("MIT for single-use mandate", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: MIT for single-use mandate");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["GiropayMITAutoCapture"];
        const { Request: reqData, Response: resData } = data;

        const requestBody = { ...fixtures.mitConfirmBody };
        for (const key in reqData) {
          requestBody[key] = reqData[key];
        }
        requestBody.amount = 6500;
        requestBody.confirm = true;
        requestBody.capture_method = "automatic";
        requestBody.customer_id = globalState.get("customerId");
        requestBody.mandate_id = globalState.get("mandateId");

        cy.request({
          method: "POST",
          url: `${globalState.get("baseUrl")}/payments`,
          headers: {
            "Content-Type": "application/json",
            "api-key": globalState.get("apiKey"),
          },
          failOnStatusCode: false,
          body: requestBody,
        }).then((response) => {
          globalState.set("paymentID", response.body.payment_id);
          expect(response.status).to.equal(resData.status);
          expect(response.body.status).to.equal(resData.body.status);
        });

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve MIT Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve MIT Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["GiropayMITAutoCapture"];
        cy.retrievePaymentCallTest({ globalState, data });
      });

      cy.step("List Mandates", () => {
        if (!mandateCreated) {
          cy.task(
            "cli_log",
            "Skipping step: List Mandates - no mandate created"
          );
          return;
        }
        cy.listMandateCallTest(globalState);
      });

      cy.step("Revoke Mandate", () => {
        if (!mandateCreated) {
          cy.task(
            "cli_log",
            "Skipping step: Revoke Mandate - no mandate created"
          );
          return;
        }
        cy.revokeMandateCallTest(globalState);
      });
    });
  });

  context("Giropay - Multi-use Mandate CIT and MIT", () => {
    it("Create Intent -> Confirm CIT -> Handle Redirect -> Retrieve CIT -> MIT -> Retrieve MIT", () => {
      let shouldContinue = true;
      let mandateCreated = false;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["PaymentIntentOffSession"]("Giropay");
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

      cy.step("Confirm CIT for multi-use mandate", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Confirm CIT for multi-use mandate"
          );
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["GiropayMandateMultiUseNo3DSAutoCapture"];
        cy.confirmBankRedirectCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle Bank Redirect Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle Bank Redirect Redirection");
          return;
        }
        const expected_redirection = fixtures.confirmBody["return_url"];
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handleBankRedirectRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      });

      cy.step("Retrieve CIT and extract mandate_id", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Retrieve CIT and extract mandate_id"
          );
          return;
        }
        cy.request({
          method: "GET",
          url: `${globalState.get("baseUrl")}/payments/${globalState.get("paymentID")}?force_sync=true`,
          headers: {
            "Content-Type": "application/json",
            "api-key": globalState.get("apiKey"),
          },
          failOnStatusCode: false,
        }).then((response) => {
          expect(response.status).to.equal(200);
          if (response.body.mandate_id) {
            globalState.set("mandateId", response.body.mandate_id);
            mandateCreated = true;
          } else {
            cy.task(
              "cli_log",
              "No mandate_id returned — skipping MIT and subsequent steps"
            );
            shouldContinue = false;
          }
        });
      });

      cy.step("MIT for multi-use mandate", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: MIT for multi-use mandate");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["GiropayMITAutoCapture"];
        const { Request: reqData, Response: resData } = data;

        const requestBody = { ...fixtures.mitConfirmBody };
        for (const key in reqData) {
          requestBody[key] = reqData[key];
        }
        requestBody.amount = 6500;
        requestBody.confirm = true;
        requestBody.capture_method = "automatic";
        requestBody.customer_id = globalState.get("customerId");
        requestBody.mandate_id = globalState.get("mandateId");

        cy.request({
          method: "POST",
          url: `${globalState.get("baseUrl")}/payments`,
          headers: {
            "Content-Type": "application/json",
            "api-key": globalState.get("apiKey"),
          },
          failOnStatusCode: false,
          body: requestBody,
        }).then((response) => {
          globalState.set("paymentID", response.body.payment_id);
          expect(response.status).to.equal(resData.status);
          expect(response.body.status).to.equal(resData.body.status);
        });

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve MIT Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve MIT Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["GiropayMITAutoCapture"];
        cy.retrievePaymentCallTest({ globalState, data });
      });

      cy.step("List Mandates", () => {
        if (!mandateCreated) {
          cy.task(
            "cli_log",
            "Skipping step: List Mandates - no mandate created"
          );
          return;
        }
        cy.listMandateCallTest(globalState);
      });

      cy.step("Revoke Mandate", () => {
        if (!mandateCreated) {
          cy.task(
            "cli_log",
            "Skipping step: Revoke Mandate - no mandate created"
          );
          return;
        }
        cy.revokeMandateCallTest(globalState);
      });
    });
  });

  context("Sofort - Single-use Mandate CIT (expected failure)", () => {
    it("Create Intent -> Confirm CIT (Sofort mandate fails at sandbox)", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["PaymentIntentOffSession"]("Sofort");
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

      cy.step("Confirm CIT for single-use mandate", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Confirm CIT for single-use mandate"
          );
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["SofortMandateSingleUseNo3DSAutoCapture"];
        cy.confirmBankRedirectCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });
    });
  });

  context("Sofort - Multi-use Mandate CIT (expected failure)", () => {
    it("Create Intent -> Confirm CIT (Sofort mandate fails at sandbox)", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["PaymentIntentOffSession"]("Sofort");
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

      cy.step("Confirm CIT for multi-use mandate", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Confirm CIT for multi-use mandate"
          );
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "bank_redirect_pm"
        ]["SofortMandateMultiUseNo3DSAutoCapture"];
        cy.confirmBankRedirectCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });
    });
  });
});
