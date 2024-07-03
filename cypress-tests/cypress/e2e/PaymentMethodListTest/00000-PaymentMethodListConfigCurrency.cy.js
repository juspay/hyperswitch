import apiKeyCreateBody from "../../fixtures/create-api-key-body.json";
import createConnectorBody from "../../fixtures/create-connector-body.json";
import getConnectorDetails from "../PaymentMethodListUtils/utils";
import merchantCreateBody from "../../fixtures/merchant-create-body.json";
import * as utils from "../PaymentMethodListUtils/utils";
import {
  card_credit_enabled,
  bank_redirect_ideal_enabled,
  create_payment_body_in_EUR,
} from "../PaymentMethodListUtils/Common";
import State from "../../utils/State";

// Testing for scenario:
// MCA1 -> Stripe configured with ideal = { country = "NL", currency = "EUR" }
// MCA2 -> Cybersource configured with credit = { currency = "USD" }
// Payment is done with currency as EUR and no billing address
// The resultant Payment Method list should only have ideal with stripe

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

  // stripe connector create with ideal enabled
  it("connector-create-call-test", () => {
    cy.createConnectorCallTest(
      createConnectorBody,
      bank_redirect_ideal_enabled,
      globalState
    );
  });

  // cybersource connector create with card credit enabled
  it("connector-create-call-test", () => {
    cy.createNamedConnectorCallTest(
      createConnectorBody,
      card_credit_enabled,
      globalState,
      "cybersource"
    );
  });

  // creating payment with currency as EUR and no billing address
  it("create-payment-call-test", () => {
    let data = getConnectorDetails("stripe")["pm_list"]["PaymentIntent"];
    let req_data = data["RequestCurrencyEUR"];
    let res_data = data["Response"];

    cy.createPaymentIntentTest(
      create_payment_body_in_EUR,
      req_data,
      res_data,
      "no_three_ds",
      "automatic",
      globalState
    );
  });

  // payment method list which should only have ideal with stripe
  it("payment-method-list-call-test", () => {
    let data = getConnectorDetails(globalState.get("connectorId"))["pm_list"][
      "PmListResponse"
    ]["PmListWithStripeForIdeal"];
    cy.paymentMethodListTestLessThanEqualToOneConnector(data, globalState);
  });
});
