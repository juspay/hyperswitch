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
        profile_id: globalState.get("profileIdCm1"),
        customer_id: globalState.get("customerIdCm1Created"),
      };

      cy.createPaymentWithHeaderCall(
        paymentRequestBody,
        globalState.get("apiKey"),
        globalState.get("connectedMerchantId1"),
        globalState,
        200,
        "platformPaymentIdCm1"
      );
    });

    it("platform-creates-payment-for-cm2-using-header", () => {
      const paymentRequestBody = {
        ...fixtures.createConfirmPaymentBody,
        profile_id: globalState.get("profileIdCm2"),
        customer_id: globalState.get("customerIdCm1Created"),
      };

      cy.createPaymentWithHeaderCall(
        paymentRequestBody,
        globalState.get("apiKey"),
        globalState.get("connectedMerchantId2"),
        globalState,
        200,
        "platformPaymentIdCm2"
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
        profile_id: globalState.get("profileId_CM1"),
        customer_id: globalState.get("customerId_CM1_Created"),
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
        profile_id: globalState.get("profileId_CM2"),
        customer_id: globalState.get("customerId_CM1_Created"),
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
  });

  context("Platform Cannot Create Payment Without On-Behalf-Of Header", () => {
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
        expect(response.status).to.equal(400);
      });
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
        globalState
      ).then((response) => {
        expect(response.status).to.equal(401);
      });
    });
  });

  context("Connected Merchants Create Own Payments", () => {
    it("cm1-creates-payment-for-shared-customer", () => {
      const paymentRequestBody = {
        ...fixtures.createConfirmPaymentBody,
        customer_id: globalState.get("customerId_CM1_Created"),
        profile_id: globalState.get("profileId_CM1"),
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
        customer_id: globalState.get("customerId_CM1_Created"),
        profile_id: globalState.get("profileId_CM2"),
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
        globalState
      ).then((response) => {
        expect(response.status).to.equal(401);
      });
    });
  });

  context("Payment List Isolation", () => {
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
