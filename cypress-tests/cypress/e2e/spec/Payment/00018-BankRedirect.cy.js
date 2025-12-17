import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Bank Redirect tests", () => {
  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Blik Create and Confirm flow test", () => {
    let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_redirect_pm"
      ]["PaymentIntent"]("Blik");

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm bank redirect", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_redirect_pm"
      ]["Blik"];

      cy.confirmBankRedirectCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
  });

  context("EPS Create and Confirm flow test", () => {
    let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_redirect_pm"
      ]["PaymentIntent"]("Eps");

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm bank redirect", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_redirect_pm"
      ]["Eps"];

      cy.confirmBankRedirectCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Handle bank redirect redirection", () => {
      // return_url is a static url (https://example.com) taken from confirm-body fixture and is not updated
      const expected_redirection = fixtures.confirmBody["return_url"];
      const payment_method_type = globalState.get("paymentMethodType");

      cy.handleBankRedirectRedirection(
        globalState,
        payment_method_type,
        expected_redirection
      );
    });
  });

  context("iDEAL Create and Confirm flow test", () => {
    let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_redirect_pm"
      ]["PaymentIntent"]("Ideal");

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm bank redirect", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_redirect_pm"
      ]["Ideal"];

      cy.confirmBankRedirectCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Handle bank redirect redirection", () => {
      // return_url is a static url (https://example.com) taken from confirm-body fixture and is not updated
      const expected_redirection = fixtures.confirmBody["return_url"];
      const payment_method_type = globalState.get("paymentMethodType");
      cy.handleBankRedirectRedirection(
        globalState,
        payment_method_type,
        expected_redirection
      );
    });
  });

  context("Sofort Create and Confirm flow test", () => {
    let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });
    it("create-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_redirect_pm"
      ]["PaymentIntent"]("Sofort");

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm bank redirect", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_redirect_pm"
      ]["Sofort"];

      cy.confirmBankRedirectCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Handle bank redirect redirection", () => {
      // return_url is a static url (https://example.com) taken from confirm-body fixture and is not updated
      const expected_redirection = fixtures.confirmBody["return_url"];
      const payment_method_type = globalState.get("paymentMethodType");
      cy.handleBankRedirectRedirection(
        globalState,
        payment_method_type,
        expected_redirection
      );
    });
  });

  context("Przelewy24 Create and Confirm flow test", () => {
    let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });
    it("create-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_redirect_pm"
      ]["PaymentIntent"]("Przelewy24");

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm bank redirect", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_redirect_pm"
      ]["Przelewy24"];

      cy.confirmBankRedirectCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Handle bank redirect redirection", () => {
      const expected_redirection = fixtures.confirmBody["return_url"];
      const payment_method_type = globalState.get("paymentMethodType");
      cy.handleBankRedirectRedirection(
        globalState,
        payment_method_type,
        expected_redirection
      );
    });
  });
  
  context("OpenBankingUk Create and Confirm flow test", () => {
    let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    before("ensure connector supports OpenBankingUk", () => {
      const connectorId = (globalState.get("connectorId") || "").toLowerCase();
      if (connectorId !== "volt") {
        return;
      }

      cy.enableVoltBankRedirectPaymentMethods(globalState);
    });

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_redirect_pm"
      ]["PaymentIntent"]("OpenBankingUk");

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm bank redirect", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_redirect_pm"
      ]["OpenBankingUk"];

      cy.confirmBankRedirectCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
    it("Handle bank redirect redirection", () => {
      const expected_redirection = fixtures.confirmBody["return_url"];
      const payment_method_type = globalState.get("paymentMethodType");
      cy.handleBankRedirectRedirection(
        globalState,
        payment_method_type,
        expected_redirection
      );
    });

    it("Sync payment status", () => {
      const paymentIntentId = globalState.get("paymentID");
      const syncUrl = `${globalState.get("baseUrl")}/payments/${paymentIntentId}?force_sync=true`;

      cy.request({
        method: "GET",
        url: syncUrl,
        headers: {
          "Content-Type": "application/json",
          "api-key": globalState.get("apiKey"),
        },
        failOnStatusCode: false,
      }).then((response) => {
        expect(response.status).to.equal(200);
        expect(response.body.payment_id).to.equal(paymentIntentId);
        expect(response.body.connector).to.equal(globalState.get("connectorId"));
        expect(response.body.payment_method_type).to.equal("open_banking_uk");
        expect(response.body.status).to.equal("requires_customer_action");
      });
    });
  });
 });