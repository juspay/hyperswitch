import apiKeyCreateBody from "../../fixtures/create-api-key-body.json";
import createConnectorBody from "../../fixtures/create-connector-body.json";
import getConnectorDetails from "../PaymentMethodListUtils/utils";
import merchantCreateBody from "../../fixtures/merchant-create-body.json";
import * as utils from "../PaymentMethodListUtils/utils";
import {
  card_credit_enabled,
  card_credit_enabled_in_US,
  bank_redirect_ideal_enabled,
  create_payment_body_in_USD,
} from "../PaymentMethodListUtils/Common";
import State from "../../utils/State";

// Testing for scenario:
// MCA1 -> Stripe configured with credit = { country = "US" }
// MCA2 -> Cybersource configured with credit = { country = "US" }
// Payment is done with country as US and currency as USD
// The resultant Payment Method list should have both Stripe and cybersource

let globalState;
describe("Account Create flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  it("merchant-create-call-test", () => {
    cy.merchantCreateCallTest(merchantCreateBody, globalState);
  });

  it("api-key-create-call-test", () => {
    cy.apiKeyCreateTest(apiKeyCreateBody, globalState);
  });

  // stripe connector create with credit enabled for US
  it("connector-create-call-test", () => {
    cy.createNamedConnectorCallTest(
      createConnectorBody,
      card_credit_enabled_in_US,
      globalState,
      "stripe"
    );
  });

  // cybersource connector create with card credit enabled in US
  it("connector-create-call-test", () => {
    cy.createNamedConnectorCallTest(
      createConnectorBody,
      card_credit_enabled_in_US,
      globalState,
      "cybersource"
    );
  });

  // creating payment with currency as USD and billing address as US
  it("create-payment-call-test", () => {
    let data = getConnectorDetails("stripe")["pm_list"]["PaymentIntent"];
    let req_data = data["RequestCurrencyUSD"];
    let res_data = data["Response"];

    cy.createPaymentIntentTest(
      create_payment_body_in_USD,
      req_data,
      res_data,
      "no_three_ds",
      "automatic",
      globalState
    );
  });

  // payment method list which should only have credit with Stripe and Cybersource
  it("payment-method-list-call-test", () => {
    let data = getConnectorDetails(globalState.get("connectorId"))["pm_list"][
      "PmListResponse"
    ]["PmListWithCreditTwoConnector"];
    cy.paymentMethodListTestTwoConnectorsForOnePaymentMethodCredit(
      data,
      globalState
    );
  });
});
