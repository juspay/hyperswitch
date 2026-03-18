import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails from "../../configs/Payment/Utils";

let globalState;

describe("Platform Payment Flows", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Platform Acts On Behalf Of Connected Merchants", () => {
    it("platform-creates-payment-for-cm1-using-header", () => {
      const paymentRequestBody = {
        ...fixtures.createConfirmPaymentBody,
        profile_id: globalState.get("profileId_CM1"),
        customer_id: globalState.get("customerId_CM1_Created"),
      };

      cy.createPaymentWithHeaderCallTest(
        paymentRequestBody,
        globalState.get("apiKey"),
        globalState.get("connectedMerchantId_1"),
        globalState,
        200,
        "platformPaymentId_CM1"
      );
    });

    it("platform-creates-payment-for-cm2-using-header", () => {
      const paymentRequestBody = {
        ...fixtures.createConfirmPaymentBody,
        profile_id: globalState.get("profileId_CM2"),
        customer_id: globalState.get("customerId_CM1_Created"),
      };

      cy.createPaymentWithHeaderCallTest(
        paymentRequestBody,
        globalState.get("apiKey"),
        globalState.get("connectedMerchantId_2"),
        globalState,
        200,
        "platformPaymentId_CM2"
      );
    });
  });

  context("Platform Cannot Create Payment For Standard Merchant", () => {
    it("platform-cannot-create-payment-for-standard-merchant", () => {
      const paymentRequestBody = {
        ...fixtures.createConfirmPaymentBody,
        profile_id: globalState.get("profileId_SM"),
      };

      cy.createPaymentWithHeaderCallTest(
        paymentRequestBody,
        globalState.get("apiKey"),
        globalState.get("standardMerchantId"),
        globalState,
        401
      );
    });
  });

  context("Connected Merchants Create Own Payments", () => {
    it("cm1-creates-payment-for-shared-customer", () => {
      globalState.set("apiKey", globalState.get("apiKey_CM1"));
      globalState.set("customerId", globalState.get("customerId_CM1_Created"));
      globalState.set("profileId", globalState.get("profileId_CM1"));
      globalState.set(
        "merchantConnectorId",
        globalState.get("connectorId_CM1")
      );

      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSAutoCapture"];

      cy.createConfirmPaymentTest(
        fixtures.createConfirmPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
    });

    it("cm2-creates-payment-for-same-shared-customer", () => {
      globalState.set("apiKey", globalState.get("apiKey_CM2"));
      globalState.set("customerId", globalState.get("customerId_CM1_Created"));
      globalState.set("profileId", globalState.get("profileId_CM2"));
      globalState.set(
        "merchantConnectorId",
        globalState.get("connectorId_CM2")
      );

      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSAutoCapture"];

      cy.createConfirmPaymentTest(
        fixtures.createConfirmPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
    });
  });

  context("Connected Merchant Cannot Act On Behalf Of Another Merchant", () => {
    it("connected-merchant-cannot-act-on-behalf-of-another-merchant", () => {
      const paymentRequestBody = {
        ...fixtures.createConfirmPaymentBody,
        customer_id: globalState.get("customerId_CM1_Created"),
      };

      cy.createPaymentWithHeaderCallTest(
        paymentRequestBody,
        globalState.get("apiKey_CM1"),
        globalState.get("connectedMerchantId_2"),
        globalState,
        401
      );
    });
  });

  context("Payment List Isolation", () => {
    it("cm1-lists-only-own-payments", () => {
      cy.listPaymentsWithApiKeyCallTest(
        globalState.get("apiKey_CM1"),
        globalState,
        "cm2PaymentId"
      );
    });

    it("cm2-lists-only-own-payments", () => {
      cy.listPaymentsWithApiKeyCallTest(
        globalState.get("apiKey_CM2"),
        globalState,
        "cm1PaymentId"
      );
    });
  });
});
