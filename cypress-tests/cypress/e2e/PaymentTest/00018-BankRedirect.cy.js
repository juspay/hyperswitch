import * as fixtures from "../../fixtures/imports";
import State from "../../utils/State";
import getConnectorDetails, * as utils from "../PaymentUtils/Utils";

let globalState;

describe("Bank Redirect tests", () => {
  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Blik Create and Confirm flow test", () => {
    let should_continue = true; // variable that will be used to skip tests if a previous test fails

    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    beforeEach(function () {
      if (!should_continue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_redirect_pm"
      ]["BlikPaymentIntent"];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
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
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm bank redirect", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_redirect_pm"
      ]["Blik"];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.confirmBankRedirectCallTest(
        fixtures.confirmBody,
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

    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

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
        fixtures.createPaymentBody,
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
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm bank redirect", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_redirect_pm"
      ]["Eps"];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.confirmBankRedirectCallTest(
        fixtures.confirmBody,
        req_data,
        res_data,
        true,
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("Handle bank redirect redirection", () => {
      // return_url is a static url (https://hyperswitch.io) taken from confirm-body fixture and is not updated
      let expected_redirection = fixtures.confirmBody["return_url"];
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

    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

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
        fixtures.createPaymentBody,
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
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm bank redirect", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_redirect_pm"
      ]["Ideal"];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.confirmBankRedirectCallTest(
        fixtures.confirmBody,
        req_data,
        res_data,
        true,
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("Handle bank redirect redirection", () => {
      // return_url is a static url (https://hyperswitch.io) taken from confirm-body fixture and is not updated
      let expected_redirection = fixtures.confirmBody["return_url"];
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

    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

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
        fixtures.createPaymentBody,
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
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm bank redirect", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_redirect_pm"
      ]["Giropay"];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.confirmBankRedirectCallTest(
        fixtures.confirmBody,
        req_data,
        res_data,
        true,
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("Handle bank redirect redirection", () => {
      // return_url is a static url (https://hyperswitch.io) taken from confirm-body fixture and is not updated
      let expected_redirection = fixtures.confirmBody["return_url"];
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

    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

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
        fixtures.createPaymentBody,
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
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm bank redirect", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_redirect_pm"
      ]["Sofort"];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.confirmBankRedirectCallTest(
        fixtures.confirmBody,
        req_data,
        res_data,
        true,
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("Handle bank redirect redirection", () => {
      // return_url is a static url (https://hyperswitch.io) taken from confirm-body fixture and is not updated
      let expected_redirection = fixtures.confirmBody["return_url"];
      let payment_method_type = globalState.get("paymentMethodType");
      cy.handleBankRedirectRedirection(
        globalState,
        payment_method_type,
        expected_redirection
      );
    });
  });

  context("Przelewy24 Create and Confirm flow test", () => {
    let should_continue = true; // variable that will be used to skip tests if a previous test fails

    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

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
        fixtures.createPaymentBody,
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
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm bank redirect", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_redirect_pm"
      ]["Przelewy24"];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.confirmBankRedirectCallTest(
        fixtures.confirmBody,
        req_data,
        res_data,
        true,
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("Handle bank redirect redirection", () => {
      let expected_redirection = fixtures.confirmBody["return_url"];
      let payment_method_type = globalState.get("paymentMethodType");
      cy.handleBankRedirectRedirection(
        globalState,
        payment_method_type,
        expected_redirection
      );
    });
  });
});
