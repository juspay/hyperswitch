import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";

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
        customer_id: globalState.get("customerId"),
        profile_id: globalState.get("profileId_CM1"),
      };

      cy.createPaymentWithHeaderCallTest(
        paymentRequestBody,
        globalState.get("apiKey"),
        globalState.get("connectedMerchantId_1"),
        globalState
      ).then((response) => {
        if (response.status !== 200) {
          cy.task(
            "cli_log",
            `Payment create failed: ${JSON.stringify(response.body)}`
          );
        }
        expect(response.status).to.equal(200);
        expect(response.body).to.have.property("payment_id");
        globalState.set("platformPaymentId_CM1", response.body.payment_id);
      });
    });

    it("platform-creates-payment-for-cm2-using-header", () => {
      const paymentRequestBody = {
        ...fixtures.createConfirmPaymentBody,
        customer_id: globalState.get("customerId"),
        profile_id: globalState.get("profileId_CM2"),
      };

      cy.createPaymentWithHeaderCallTest(
        paymentRequestBody,
        globalState.get("apiKey"),
        globalState.get("connectedMerchantId_2"),
        globalState
      ).then((response) => {
        expect(response.status).to.equal(200);
        expect(response.body).to.have.property("payment_id");
        globalState.set("platformPaymentId_CM2", response.body.payment_id);
      });
    });

    it("platform-cannot-create-payment-without-header", () => {
      const paymentRequestBody = {
        ...fixtures.createConfirmPaymentBody,
        customer_id: globalState.get("customerId"),
      };

      cy.createPaymentWithApiKeyCallTest(
        paymentRequestBody,
        globalState.get("apiKey"),
        globalState
      ).then((response) => {
        expect(response.status).to.be.oneOf([400, 403, 422]);
      });
    });
  });

  context("Connected Merchants Create Own Payments", () => {
    it("cm1-creates-payment-for-shared-customer", () => {
      const paymentRequestBody = {
        ...fixtures.createConfirmPaymentBody,
        customer_id: globalState.get("customerId"),
      };

      cy.createPaymentWithApiKeyCallTest(
        paymentRequestBody,
        globalState.get("apiKey_CM1"),
        globalState
      ).then((response) => {
        expect(response.status).to.equal(200);
        expect(response.body).to.have.property("payment_id");
        globalState.set("cm1PaymentId", response.body.payment_id);
      });
    });

    it("cm2-creates-payment-for-same-shared-customer", () => {
      const paymentRequestBody = {
        ...fixtures.createConfirmPaymentBody,
        customer_id: globalState.get("customerId"),
      };

      cy.createPaymentWithApiKeyCallTest(
        paymentRequestBody,
        globalState.get("apiKey_CM2"),
        globalState
      ).then((response) => {
        expect(response.status).to.equal(200);
        expect(response.body).to.have.property("payment_id");
        globalState.set("cm2PaymentId", response.body.payment_id);
      });
    });
  });

  context("Connected Merchant Cannot Act On Behalf Of Another", () => {
    it("cm1-tries-to-create-payment-for-cm2-using-header", () => {
      const paymentRequestBody = {
        ...fixtures.createConfirmPaymentBody,
        customer_id: globalState.get("customerId"),
      };

      cy.createPaymentWithHeaderCallTest(
        paymentRequestBody,
        globalState.get("apiKey_CM1"),
        globalState.get("connectedMerchantId_2"),
        globalState
      ).then((response) => {
        expect(response.status).to.be.oneOf([401, 403]);
      });
    });
  });

  context("Payment Isolation", () => {
    it("cm1-lists-only-own-payments", () => {
      cy.listPaymentsWithApiKeyCallTest(
        globalState.get("apiKey_CM1"),
        globalState
      ).then((response) => {
        expect(response.status).to.equal(200);
        expect(response.body).to.have.property("data");
        expect(response.body.data).to.be.an("array");

        const cm2PaymentId = globalState.get("cm2PaymentId");
        const hasCM2Payment = response.body.data.some(
          (payment) => payment.payment_id === cm2PaymentId
        );
        expect(hasCM2Payment).to.be.false;
      });
    });

    it("cm2-lists-only-own-payments", () => {
      cy.listPaymentsWithApiKeyCallTest(
        globalState.get("apiKey_CM2"),
        globalState
      ).then((response) => {
        expect(response.status).to.equal(200);
        expect(response.body).to.have.property("data");
        expect(response.body.data).to.be.an("array");

        const cm1PaymentId = globalState.get("cm1PaymentId");
        const hasCM1Payment = response.body.data.some(
          (payment) => payment.payment_id === cm1PaymentId
        );
        expect(hasCM1Payment).to.be.false;
      });
    });
  });
});
