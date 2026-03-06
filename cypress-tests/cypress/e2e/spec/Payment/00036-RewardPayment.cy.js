import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";
import step from "../../../utils/customStep";

let globalState;

describe("Reward Payment - Cashtocode", () => {
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

  context("Evoucher payment method flow", () => {
    it("Create Payment Intent for Evoucher -> Payment Methods Call -> Confirm Evoucher Payment -> Handle Redirection -> Retrieve Payment", () => {
      let shouldContinue = true;

      step("Create Payment Intent for Evoucher", shouldContinue, () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "reward_pm"
        ]["PaymentIntentUSD"];
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      step("Payment Methods Call", shouldContinue, () => {
        cy.paymentMethodsCallTest(globalState);
      });

      step("Confirm Evoucher Payment", shouldContinue, () => {
        const confirmData = getConnectorDetails(
          globalState.get("connectorId")
        )["reward_pm"]["Evoucher"];
        cy.confirmRewardCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      step("Handle Redirection", shouldContinue, () => {
        const expected_redirection = fixtures.confirmBody["return_url"];
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handleRewardRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      });

      step("Retrieve Payment", shouldContinue, () => {
        const confirmData = getConnectorDetails(
          globalState.get("connectorId")
        )["reward_pm"]["Evoucher"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
      });
    });
  });

  context("Classic payment method flow", () => {
    it("Create Payment Intent for Classic -> Payment Methods Call -> Confirm Classic Payment -> Handle Redirection for Classic -> Retrieve Payment", () => {
      let shouldContinue = true;

      step("Create Payment Intent for Classic", shouldContinue, () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "reward_pm"
        ]["PaymentIntentEUR"];
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      step("Payment Methods Call", shouldContinue, () => {
        cy.paymentMethodsCallTest(globalState);
      });

      step("Confirm Classic Payment", shouldContinue, () => {
        const confirmData = getConnectorDetails(
          globalState.get("connectorId")
        )["reward_pm"]["Classic"];
        cy.confirmRewardCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      step("Handle Redirection for Classic", shouldContinue, () => {
        const expected_redirection = fixtures.confirmBody["return_url"];
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handleRewardRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      });

      step("Retrieve Payment", shouldContinue, () => {
        const confirmData = getConnectorDetails(
          globalState.get("connectorId")
        )["reward_pm"]["Classic"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
      });
    });
  });
});
