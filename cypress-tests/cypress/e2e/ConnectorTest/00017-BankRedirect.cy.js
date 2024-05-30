import confirmBody from "../../fixtures/confirm-body.json";
import createPaymentBody from "../../fixtures/create-payment-body.json";
import State from "../../utils/State";
import getConnectorDetails, * as utils from "../ConnectorUtils/utils";

let globalState;

describe("Bank Redirect tests", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      console.log("seeding globalState -> " + JSON.stringify(globalState));
    });
  });

  after("flush global state", () => {
    console.log("flushing globalState -> " + JSON.stringify(globalState));
    cy.task("setGlobalState", globalState.data);
  });

  afterEach("flush global state", () => {
    console.log("flushing globalState -> " + JSON.stringify(globalState));
    cy.task("setGlobalState", globalState.data);
    cy.task(
      "cli_log",
      " FLUSHING GLOBAL STATE -> " + JSON.stringify(globalState)
    );
  });

  context("Blik Create and Confirm flow test", () => {
    let should_continue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!should_continue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_redirect_pm"
      ]["blikPaymentIntent"];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.createPaymentIntentTest(
        createPaymentBody,
        req_data,
        res_data,
        "three_ds",
        "automatic",
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("payment_methods-call-test", () => {
      cy.task("cli_log", "PM CALL ");
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm bank redirect", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_redirect_pm"
      ]["blik"];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.task("cli_log", "GLOBAL STATE -> " + JSON.stringify(globalState.data));
      cy.confirmBankRedirectCallTest(
        confirmBody,
        req_data,
        res_data,
        true,
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });
  });

  context("EPS Create and Confirm flow test", () => {
    let should_continue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!should_continue) {
        this.skip();
      }
    });
    it("create-payment-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_redirect_pm"
      ]["PaymentIntent"];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.createPaymentIntentTest(
        createPaymentBody,
        req_data,
        res_data,
        "three_ds",
        "automatic",
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("payment_methods-call-test", () => {
      cy.task("cli_log", "PM CALL ");
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm bank redirect", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_redirect_pm"
      ]["eps"];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.task("cli_log", "GLOBAL STATE -> " + JSON.stringify(globalState.data));
      cy.confirmBankRedirectCallTest(
        confirmBody,
        req_data,
        res_data,
        true,
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("Handle bank redirect redirection", () => {
      let expected_redirection = confirmBody["return_url"];
      let payment_method_type = globalState.get("paymentMethodType");
      cy.handleBankRedirectRedirection(
        globalState,
        payment_method_type,
        expected_redirection
      );
    });
  });

  context("iDEAL Create and Confirm flow test", () => {
    let should_continue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!should_continue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_redirect_pm"
      ]["PaymentIntent"];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.createPaymentIntentTest(
        createPaymentBody,
        req_data,
        res_data,
        "three_ds",
        "automatic",
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("payment_methods-call-test", () => {
      cy.task("cli_log", "PM CALL ");
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm bank redirect", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_redirect_pm"
      ]["ideal"];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.task("cli_log", "GLOBAL STATE -> " + JSON.stringify(globalState.data));
      cy.confirmBankRedirectCallTest(
        confirmBody,
        req_data,
        res_data,
        true,
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("Handle bank redirect redirection", () => {
      let expected_redirection = confirmBody["return_url"];
      let payment_method_type = globalState.get("paymentMethodType");
      cy.handleBankRedirectRedirection(
        globalState,
        payment_method_type,
        expected_redirection
      );
    });
  });

  context("Giropay Create and Confirm flow test", () => {
    let should_continue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!should_continue) {
        this.skip();
      }
    });
    it("create-payment-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_redirect_pm"
      ]["PaymentIntent"];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.createPaymentIntentTest(
        createPaymentBody,
        req_data,
        res_data,
        "three_ds",
        "automatic",
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("payment_methods-call-test", () => {
      cy.task("cli_log", "PM CALL ");
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm bank redirect", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_redirect_pm"
      ]["giropay"];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.task("cli_log", "GLOBAL STATE -> " + JSON.stringify(globalState.data));
      cy.confirmBankRedirectCallTest(
        confirmBody,
        req_data,
        res_data,
        true,
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("Handle bank redirect redirection", () => {
      let expected_redirection = confirmBody["return_url"];
      let payment_method_type = globalState.get("paymentMethodType");
      cy.handleBankRedirectRedirection(
        globalState,
        payment_method_type,
        expected_redirection
      );
    });
  });

  context("Sofort Create and Confirm flow test", () => {
    let should_continue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!should_continue) {
        this.skip();
      }
    });
    it("create-payment-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_redirect_pm"
      ]["PaymentIntent"];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.createPaymentIntentTest(
        createPaymentBody,
        req_data,
        res_data,
        "three_ds",
        "automatic",
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("payment_methods-call-test", () => {
      cy.task("cli_log", "PM CALL ");
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm bank redirect", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_redirect_pm"
      ]["sofort"];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.task("cli_log", "GLOBAL STATE -> " + JSON.stringify(globalState.data));
      cy.confirmBankRedirectCallTest(
        confirmBody,
        req_data,
        res_data,
        true,
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("Handle bank redirect redirection", () => {
      let expected_redirection = confirmBody["return_url"];
      let payment_method_type = globalState.get("paymentMethodType");
      cy.handleBankRedirectRedirection(
        globalState,
        payment_method_type,
        expected_redirection
      );
    });
  });
});
