import * as fixtures from "../../fixtures/imports";
import State from "../../utils/State";
import { validateConfig } from "../../utils/featureFlags";
import getConnectorDetails, * as utils from "../PaymentUtils/Utils";

let globalState;

describe("UPI Payments - Hyperswitch", () => {
  let should_continue = true; // variable that will be used to skip tests if a previous test fails

  context("[Payment] [UPI - UPI Collect] Create & Confirm + Refund", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    afterEach("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    beforeEach(function () {
      if (!should_continue) {
        this.skip();
      }
    });

    it("Create payment intent", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["upi_pm"][
        "PaymentIntent"
      ];

      let configs = validateConfig(data["Configs"]);
      let req_data = data["Request"];
      let res_data = data["Response"];

      cy.createPaymentIntentTest(
        configs,
        fixtures.createPaymentBody,
        req_data,
        res_data,
        "three_ds",
        "automatic",
        globalState
      );

      if (should_continue)
        should_continue = utils.should_continue_further(res_data, configs);
    });

    it("List Merchant payment methods", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm payment", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["upi_pm"][
        "UpiCollect"
      ];

      let configs = validateConfig(data["Configs"]);
      let req_data = data["Request"];
      let res_data = data["Response"];

      cy.confirmUpiCall(
        configs,
        fixtures.confirmBody,
        req_data,
        res_data,
        true,
        globalState
      );

      if (should_continue)
        should_continue = utils.should_continue_further(res_data, configs);
    });

    it("Handle UPI Redirection", () => {
      let expected_redirection = fixtures.confirmBody["return_url"];
      let payment_method_type = globalState.get("paymentMethodType");
      cy.handleUpiRedirection(
        globalState,
        payment_method_type,
        expected_redirection
      );
    });

    it("Retrieve payment", () => {
      cy.retrievePaymentCallTest(globalState);
    });

    it("Refund payment", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["upi_pm"][
        "Refund"
      ];

      let configs = validateConfig(data["Configs"]);
      let req_data = data["Request"];
      let res_data = data["Response"];

      cy.refundCallTest(
        configs,
        fixtures.refundBody,
        req_data,
        res_data,
        6500,
        globalState
      );

      if (should_continue)
        should_continue = utils.should_continue_further(res_data, configs);
    });
  });

  // Skipping UPI Intent intentionally as connector is throwing 5xx during redirection
  context.skip("[Payment] [UPI - UPI Intent] Create & Confirm", () => {
    should_continue = true; // variable that will be used to skip tests if a previous test fails

    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    beforeEach(function () {
      if (!should_continue) {
        this.skip();
      }
    });

    it("Create payment intent", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["upi_pm"][
        "PaymentIntent"
      ];

      let configs = validateConfig(data["Configs"]);
      let req_data = data["Request"];
      let res_data = data["Response"];

      cy.createPaymentIntentTest(
        configs,
        fixtures.createPaymentBody,
        req_data,
        res_data,
        "three_ds",
        "automatic",
        globalState
      );

      if (should_continue)
        should_continue = utils.should_continue_further(res_data, configs);
    });

    it("List Merchant payment methods", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm payment", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["upi_pm"][
        "UpiIntent"
      ];

      let configs = validateConfig(data["Configs"]);
      let req_data = data["Request"];
      let res_data = data["Response"];

      cy.confirmUpiCall(
        configs,
        fixtures.confirmBody,
        req_data,
        res_data,
        true,
        globalState
      );

      if (should_continue)
        should_continue = utils.should_continue_further(res_data, configs);
    });

    it("Handle UPI Redirection", () => {
      let expected_redirection = fixtures.confirmBody["return_url"];
      let payment_method_type = globalState.get("paymentMethodType");

      cy.handleUpiRedirection(
        globalState,
        payment_method_type,
        expected_redirection
      );
    });

    it("Retrieve payment", () => {
      cy.retrievePaymentCallTest(globalState);
    });
  });
});

// TODO: This test is incomplete. Above has to be replicated here with changes to support SCL
describe.skip("UPI Payments -- Hyperswitch Stripe Compatibility Layer", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });
});
