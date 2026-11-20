import State from "../../../../utils/State";
import getConnectorDetails from "../../../configs/Payment/Utils";

// Customer, Payment Method, and Mandate are merchant-scoped only - none of
// their DB tables carry a profile_id column at all, so only the M1-vs-M2
// (unrelated merchant) isolation model applies here, not PA-vs-PB.
//
// Routes (verified): GET/POST/DELETE /customers/{id}, GET/POST/DELETE
// /payment_methods/{id}, POST /payment_methods, GET /mandates/{id},
// POST /mandates/revoke/{id} - all ApiKeyAuth or JWTAuth. Customer/Mandate
// lookups are query-scoped by merchant_id (composite key); Payment Method
// ownership is a manual per-handler merchant_id comparison rather than a
// query-level filter (works today, but is a fragile pattern - any future
// handler that forgets to repeat the check would reopen the gap).
let globalState;
let apiKeyM1;
let apiKeyM2;
let customerIdM1;
let paymentMethodIdM1;
let mandateIdPA;

describe("OMP Scoping - Customer, Payment Method, Mandate", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      apiKeyM1 = globalState.get("apiKeyM1");
      apiKeyM2 = globalState.get("apiKeyM2");
      customerIdM1 = globalState.get("customerIdM1");
      paymentMethodIdM1 = globalState.get("paymentMethodIdM1");
      mandateIdPA = globalState.get("mandateIdPA");

      if (!mandateIdPA) {
        cy.task(
          "cli_log",
          "Mandate tests in this suite will skip: no mandateIdPA in " +
            "globalState - the CIT mandate creation step in " +
            "00-Setup.cy.js may have failed or the configured connector " +
            "may not support MandateSingleUseNo3DSAutoCapture."
        );
      }
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Customer", () => {
    it("M2 retrieves M1's customer - denied", () => {
      cy.ompRetrieveCustomer(
        customerIdM1,
        { type: "apiKey", apiKey: apiKeyM2 },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(404);
      });
    });

    it("M2 updates/deletes M1's customer - denied", () => {
      cy.ompUpdateCustomer(
        customerIdM1,
        { type: "apiKey", apiKey: apiKeyM2 },
        { name: "should not be applied" },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(404);
      });

      cy.ompDeleteCustomer(
        customerIdM1,
        { type: "apiKey", apiKey: apiKeyM2 },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(404);
      });
    });

    it("M1 retrieves its own customer (positive)", () => {
      cy.ompRetrieveCustomer(
        customerIdM1,
        { type: "apiKey", apiKey: apiKeyM1 },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(200);
        expect(res.body.customer_id).to.eq(customerIdM1);
      });
    });
  });

  context("Payment Method", () => {
    it("M2 retrieves M1's payment method - denied", () => {
      // Expected: 404 (clean ownership denial). If this fails with a 500
      // instead, that's a real defect - the ownership check isn't
      // preventing a decrypt attempt on M1's payment method data using
      // M2's merchant key before any response is built. Report it rather
      // than loosen this assertion to mask it.
      cy.ompRetrievePaymentMethod(
        paymentMethodIdM1,
        { type: "apiKey", apiKey: apiKeyM2 },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(404);
      });
    });

    it("M2 updates/deletes M1's payment method - denied", () => {
      cy.ompUpdatePaymentMethod(
        paymentMethodIdM1,
        { type: "apiKey", apiKey: apiKeyM2 },
        { card: { card_holder_name: "should not be applied" } },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(404);
      });

      cy.ompDeletePaymentMethod(
        paymentMethodIdM1,
        { type: "apiKey", apiKey: apiKeyM2 },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(404);
      });
    });

    it("M2 creates a payment method against M1's customer_id - denied", () => {
      // Reuses the shared card_pm "PaymentMethod" fixture's Request
      // (configs/Payment/Commons.js, same one 24-PaymentMethods.cy.js and
      // 00-Setup.cy.js's PAYMENT_METHOD_M1 step use) rather than hand-rolling
      // card details, per review feedback.
      const { Request: paymentMethodRequest } =
        getConnectorDetails("commons")["card_pm"]["PaymentMethod"];

      cy.ompCreatePaymentMethod(
        { type: "apiKey", apiKey: apiKeyM2 },
        { ...paymentMethodRequest, customer_id: customerIdM1 },
        globalState
      ).then((res) => {
        expect(res.status).to.be.within(400, 499);
      });
    });

    it("M1 retrieves its own payment method (positive)", () => {
      cy.ompRetrievePaymentMethod(
        paymentMethodIdM1,
        { type: "apiKey", apiKey: apiKeyM1 },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(200);
        expect(res.body.payment_method_id).to.eq(paymentMethodIdM1);
      });
    });
  });

  context("Mandate", () => {
    beforeEach(function () {
      if (!mandateIdPA) this.skip();
    });

    it("M2 retrieves/revokes M1's mandate - denied", () => {
      cy.ompRetrieveMandate(
        mandateIdPA,
        { type: "apiKey", apiKey: apiKeyM2 },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(404);
      });

      cy.ompRevokeMandate(
        mandateIdPA,
        { type: "apiKey", apiKey: apiKeyM2 },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(404);
      });
    });

    it("M1 retrieves its own mandate (positive)", () => {
      cy.ompRetrieveMandate(
        mandateIdPA,
        { type: "apiKey", apiKey: apiKeyM1 },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(200);
      });
    });
  });

  context("Edge cases", () => {
    it("retrieving a non-existent customer returns 404", () => {
      cy.ompRetrieveCustomer(
        "cus_nonexistent_omp_test",
        { type: "apiKey", apiKey: apiKeyM1 },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(404);
      });
    });

    it("retrieving a non-existent payment method returns 404", () => {
      cy.ompRetrievePaymentMethod(
        "pm_nonexistent_omp_test",
        { type: "apiKey", apiKey: apiKeyM1 },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(404);
      });
    });

    it("invalid credential on customer retrieve is rejected", () => {
      cy.ompRetrieveCustomer(
        customerIdM1,
        { type: "invalid" },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(401);
      });
    });
  });
});
