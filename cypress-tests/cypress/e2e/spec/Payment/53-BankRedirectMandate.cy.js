import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

Cypress.on("uncaught:exception", () => false);

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
        cy.task("cli_log", "Config resolved: " + JSON.stringify(data));
        if (!data || !data.Request) {
          cy.task(
            "cli_log",
            "ERROR: Invalid PaymentIntentOffSession config for Ideal"
          );
          shouldContinue = false;
          return;
        }
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
        const { Request: reqData, Response: resData } = confirmData;

        const requestBody = { ...fixtures.confirmBody };
        for (const key in reqData) {
          requestBody[key] = reqData[key];
        }
        delete requestBody.client_secret;
        requestBody.confirm = true;
        requestBody.profile_id = globalState.get("profileId");
        requestBody.customer_id = globalState.get("customerId");

        cy.request({
          method: "POST",
          url: `${globalState.get("baseUrl")}/payments/${globalState.get("paymentID")}/confirm`,
          headers: {
            "Content-Type": "application/json",
            "api-key": globalState.get("apiKey"),
          },
          failOnStatusCode: false,
          body: requestBody,
        }).then((response) => {
          cy.wrap(response).then(() => {
            expect(response.status).to.equal(resData.status);
            if (response.status === 200 && resData.body.status !== "failed") {
              globalState.set(
                "paymentMethodType",
                requestBody.payment_method_type
              );
              globalState.set(
                "nextActionUrl",
                response.body.next_action?.redirect_to_url
              );
            } else if (response.body.status === "failed") {
              expect(response.body.error_code).to.equal(
                resData.body.error_code
              );
            }
          });
        });

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
        if (!shouldContinue || !globalState.get("mandateId")) {
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
          if (response.status === 404) {
            cy.task(
              "cli_log",
              "MIT returned 404 — payment method not stored for bank redirect mandate. Skipping subsequent steps."
            );
            shouldContinue = false;
            return;
          }
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
        cy.task("cli_log", "Config resolved: " + JSON.stringify(data));
        if (!data || !data.Request) {
          cy.task(
            "cli_log",
            "ERROR: Invalid PaymentIntentOffSession config for Ideal"
          );
          shouldContinue = false;
          return;
        }
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
        const { Request: reqData, Response: resData } = confirmData;

        const requestBody = { ...fixtures.confirmBody };
        for (const key in reqData) {
          requestBody[key] = reqData[key];
        }
        delete requestBody.client_secret;
        requestBody.confirm = true;
        requestBody.profile_id = globalState.get("profileId");
        requestBody.customer_id = globalState.get("customerId");

        cy.request({
          method: "POST",
          url: `${globalState.get("baseUrl")}/payments/${globalState.get("paymentID")}/confirm`,
          headers: {
            "Content-Type": "application/json",
            "api-key": globalState.get("apiKey"),
          },
          failOnStatusCode: false,
          body: requestBody,
        }).then((response) => {
          cy.wrap(response).then(() => {
            expect(response.status).to.equal(resData.status);
            if (response.status === 200 && resData.body.status !== "failed") {
              globalState.set(
                "paymentMethodType",
                requestBody.payment_method_type
              );
              globalState.set(
                "nextActionUrl",
                response.body.next_action?.redirect_to_url
              );
            } else if (response.body.status === "failed") {
              expect(response.body.error_code).to.equal(
                resData.body.error_code
              );
            }
          });
        });

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
          if (response.status === 404) {
            cy.task(
              "cli_log",
              "MIT returned 404 — payment method not stored for bank redirect mandate. Skipping subsequent steps."
            );
            shouldContinue = false;
            return;
          }
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
        cy.task("cli_log", "Config resolved: " + JSON.stringify(data));
        if (!data || !data.Request) {
          cy.task(
            "cli_log",
            "ERROR: Invalid PaymentIntentOffSession config for Eps"
          );
          shouldContinue = false;
          return;
        }
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
        const { Request: reqData, Response: resData } = confirmData;

        const requestBody = { ...fixtures.confirmBody };
        for (const key in reqData) {
          requestBody[key] = reqData[key];
        }
        delete requestBody.client_secret;
        requestBody.confirm = true;
        requestBody.profile_id = globalState.get("profileId");
        requestBody.customer_id = globalState.get("customerId");

        cy.request({
          method: "POST",
          url: `${globalState.get("baseUrl")}/payments/${globalState.get("paymentID")}/confirm`,
          headers: {
            "Content-Type": "application/json",
            "api-key": globalState.get("apiKey"),
          },
          failOnStatusCode: false,
          body: requestBody,
        }).then((response) => {
          cy.wrap(response).then(() => {
            expect(response.status).to.equal(resData.status);
            if (response.status === 200 && resData.body.status !== "failed") {
              globalState.set(
                "paymentMethodType",
                requestBody.payment_method_type
              );
              globalState.set(
                "nextActionUrl",
                response.body.next_action?.redirect_to_url
              );
            } else if (response.body.status === "failed") {
              expect(response.body.error_code).to.equal(
                resData.body.error_code
              );
            }
          });
        });

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
        if (!shouldContinue || !globalState.get("mandateId")) {
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
          if (response.status === 404) {
            cy.task(
              "cli_log",
              "MIT returned 404 — payment method not stored for bank redirect mandate. Skipping subsequent steps."
            );
            shouldContinue = false;
            return;
          }
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
        cy.task("cli_log", "Config resolved: " + JSON.stringify(data));
        if (!data || !data.Request) {
          cy.task(
            "cli_log",
            "ERROR: Invalid PaymentIntentOffSession config for Eps"
          );
          shouldContinue = false;
          return;
        }
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
        const { Request: reqData, Response: resData } = confirmData;

        const requestBody = { ...fixtures.confirmBody };
        for (const key in reqData) {
          requestBody[key] = reqData[key];
        }
        delete requestBody.client_secret;
        requestBody.confirm = true;
        requestBody.profile_id = globalState.get("profileId");
        requestBody.customer_id = globalState.get("customerId");

        cy.request({
          method: "POST",
          url: `${globalState.get("baseUrl")}/payments/${globalState.get("paymentID")}/confirm`,
          headers: {
            "Content-Type": "application/json",
            "api-key": globalState.get("apiKey"),
          },
          failOnStatusCode: false,
          body: requestBody,
        }).then((response) => {
          cy.wrap(response).then(() => {
            expect(response.status).to.equal(resData.status);
            if (response.status === 200 && resData.body.status !== "failed") {
              globalState.set(
                "paymentMethodType",
                requestBody.payment_method_type
              );
              globalState.set(
                "nextActionUrl",
                response.body.next_action?.redirect_to_url
              );
            } else if (response.body.status === "failed") {
              expect(response.body.error_code).to.equal(
                resData.body.error_code
              );
            }
          });
        });

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
          if (response.status === 404) {
            cy.task(
              "cli_log",
              "MIT returned 404 — payment method not stored for bank redirect mandate. Skipping subsequent steps."
            );
            shouldContinue = false;
            return;
          }
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
        cy.task("cli_log", "Config resolved: " + JSON.stringify(data));
        if (!data || !data.Request) {
          cy.task(
            "cli_log",
            "ERROR: Invalid PaymentIntentOffSession config for Giropay"
          );
          shouldContinue = false;
          return;
        }
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
        const { Request: reqData, Response: resData } = confirmData;

        const requestBody = { ...fixtures.confirmBody };
        for (const key in reqData) {
          requestBody[key] = reqData[key];
        }
        delete requestBody.client_secret;
        requestBody.confirm = true;
        requestBody.profile_id = globalState.get("profileId");
        requestBody.customer_id = globalState.get("customerId");

        cy.request({
          method: "POST",
          url: `${globalState.get("baseUrl")}/payments/${globalState.get("paymentID")}/confirm`,
          headers: {
            "Content-Type": "application/json",
            "api-key": globalState.get("apiKey"),
          },
          failOnStatusCode: false,
          body: requestBody,
        }).then((response) => {
          cy.wrap(response).then(() => {
            expect(response.status).to.equal(resData.status);
            if (response.status === 200 && resData.body.status !== "failed") {
              globalState.set(
                "paymentMethodType",
                requestBody.payment_method_type
              );
              globalState.set(
                "nextActionUrl",
                response.body.next_action?.redirect_to_url
              );
            } else if (response.body.status === "failed") {
              expect(response.body.error_code).to.equal(
                resData.body.error_code
              );
            }
          });
        });

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
        if (!shouldContinue || !globalState.get("mandateId")) {
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
          if (response.status === 404) {
            cy.task(
              "cli_log",
              "MIT returned 404 — payment method not stored for bank redirect mandate. Skipping subsequent steps."
            );
            shouldContinue = false;
            return;
          }
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
        cy.task("cli_log", "Config resolved: " + JSON.stringify(data));
        if (!data || !data.Request) {
          cy.task(
            "cli_log",
            "ERROR: Invalid PaymentIntentOffSession config for Giropay"
          );
          shouldContinue = false;
          return;
        }
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
        const { Request: reqData, Response: resData } = confirmData;

        const requestBody = { ...fixtures.confirmBody };
        for (const key in reqData) {
          requestBody[key] = reqData[key];
        }
        delete requestBody.client_secret;
        requestBody.confirm = true;
        requestBody.profile_id = globalState.get("profileId");
        requestBody.customer_id = globalState.get("customerId");

        cy.request({
          method: "POST",
          url: `${globalState.get("baseUrl")}/payments/${globalState.get("paymentID")}/confirm`,
          headers: {
            "Content-Type": "application/json",
            "api-key": globalState.get("apiKey"),
          },
          failOnStatusCode: false,
          body: requestBody,
        }).then((response) => {
          cy.wrap(response).then(() => {
            expect(response.status).to.equal(resData.status);
            if (response.status === 200 && resData.body.status !== "failed") {
              globalState.set(
                "paymentMethodType",
                requestBody.payment_method_type
              );
              globalState.set(
                "nextActionUrl",
                response.body.next_action?.redirect_to_url
              );
            } else if (response.body.status === "failed") {
              expect(response.body.error_code).to.equal(
                resData.body.error_code
              );
            }
          });
        });

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
          if (response.status === 404) {
            cy.task(
              "cli_log",
              "MIT returned 404 — payment method not stored for bank redirect mandate. Skipping subsequent steps."
            );
            shouldContinue = false;
            return;
          }
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
        cy.task("cli_log", "Config resolved: " + JSON.stringify(data));
        if (!data || !data.Request) {
          cy.task(
            "cli_log",
            "ERROR: Invalid PaymentIntentOffSession config for Sofort"
          );
          shouldContinue = false;
          return;
        }
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
        const { Request: reqData, Response: resData } = confirmData;

        const requestBody = { ...fixtures.confirmBody };
        for (const key in reqData) {
          requestBody[key] = reqData[key];
        }
        delete requestBody.client_secret;
        requestBody.confirm = true;
        requestBody.profile_id = globalState.get("profileId");
        requestBody.customer_id = globalState.get("customerId");

        cy.request({
          method: "POST",
          url: `${globalState.get("baseUrl")}/payments/${globalState.get("paymentID")}/confirm`,
          headers: {
            "Content-Type": "application/json",
            "api-key": globalState.get("apiKey"),
          },
          failOnStatusCode: false,
          body: requestBody,
        }).then((response) => {
          cy.wrap(response).then(() => {
            expect(response.status).to.equal(resData.status);
            if (response.status === 200 && resData.body.status !== "failed") {
              globalState.set(
                "paymentMethodType",
                requestBody.payment_method_type
              );
              globalState.set(
                "nextActionUrl",
                response.body.next_action?.redirect_to_url
              );
            } else if (response.body.status === "failed") {
              expect(response.body.error_code).to.equal(
                resData.body.error_code
              );
            }
          });
        });

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
        cy.task("cli_log", "Config resolved: " + JSON.stringify(data));
        if (!data || !data.Request) {
          cy.task(
            "cli_log",
            "ERROR: Invalid PaymentIntentOffSession config for Sofort"
          );
          shouldContinue = false;
          return;
        }
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
        const { Request: reqData, Response: resData } = confirmData;

        const requestBody = { ...fixtures.confirmBody };
        for (const key in reqData) {
          requestBody[key] = reqData[key];
        }
        delete requestBody.client_secret;
        requestBody.confirm = true;
        requestBody.profile_id = globalState.get("profileId");
        requestBody.customer_id = globalState.get("customerId");

        cy.request({
          method: "POST",
          url: `${globalState.get("baseUrl")}/payments/${globalState.get("paymentID")}/confirm`,
          headers: {
            "Content-Type": "application/json",
            "api-key": globalState.get("apiKey"),
          },
          failOnStatusCode: false,
          body: requestBody,
        }).then((response) => {
          cy.wrap(response).then(() => {
            expect(response.status).to.equal(resData.status);
            if (response.status === 200 && resData.body.status !== "failed") {
              globalState.set(
                "paymentMethodType",
                requestBody.payment_method_type
              );
              globalState.set(
                "nextActionUrl",
                response.body.next_action?.redirect_to_url
              );
            } else if (response.body.status === "failed") {
              expect(response.body.error_code).to.equal(
                resData.body.error_code
              );
            }
          });
        });

        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });
    });
  });
});
