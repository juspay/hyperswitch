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
      globalState.set("customerId", globalState.get("customerIdCm1Created"));
      globalState.set("profileId", globalState.get("profileIdCm1"));
      globalState.set("merchantConnectorId", globalState.get("connectorIdCm1"));

      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSAutoCapture"];

      cy.createConfirmPaymentTest(
        fixtures.createConfirmPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState,
        globalState.get("connectedMerchantId1")
      );
    });

    it("platform-creates-payment-for-cm2-using-header", () => {
      globalState.set("customerId", globalState.get("customerIdCm1Created"));
      globalState.set("profileId", globalState.get("profileIdCm2"));
      globalState.set("merchantConnectorId", globalState.get("connectorIdCm2"));

      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSAutoCapture"];

      cy.createConfirmPaymentTest(
        fixtures.createConfirmPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState,
        globalState.get("connectedMerchantId2")
      );
    });
  });

  context("Platform Cannot Create Payment For Standard Merchant", () => {
    it("platform-cannot-create-payment-for-standard-merchant", () => {
      const paymentRequestBody = {
        ...fixtures.createConfirmPaymentBody,
        profile_id: globalState.get("profileIdSm"),
      };

      cy.createPaymentWithHeaderCall(
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
      globalState.set("apiKey", globalState.get("apiKeyCm1"));
      globalState.set("customerId", globalState.get("customerIdCm1Created"));
      globalState.set("profileId", globalState.get("profileIdCm1"));
      globalState.set("merchantConnectorId", globalState.get("connectorIdCm1"));

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
      globalState.set("apiKey", globalState.get("apiKeyCm2"));
      globalState.set("customerId", globalState.get("customerIdCm1Created"));
      globalState.set("profileId", globalState.get("profileIdCm2"));
      globalState.set("merchantConnectorId", globalState.get("connectorIdCm2"));

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
        customer_id: globalState.get("customerIdCm1Created"),
      };

      cy.createPaymentWithHeaderCall(
        paymentRequestBody,
        globalState.get("apiKeyCm1"),
        globalState.get("connectedMerchantId2"),
        globalState,
        401
      );
    });
  });

  context("Payment List Isolation", () => {
    it("cm1-lists-only-own-payments", () => {
      cy.listPaymentsWithApiKeyCall(
        globalState.get("apiKeyCm1"),
        globalState,
        "cm2PaymentId"
      );
    });

    it("cm2-lists-only-own-payments", () => {
      cy.listPaymentsWithApiKeyCall(
        globalState.get("apiKeyCm2"),
        globalState,
        "cm1PaymentId"
      );
    });
  });
});
