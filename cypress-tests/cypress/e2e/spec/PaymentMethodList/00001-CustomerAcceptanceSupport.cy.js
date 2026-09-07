import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import {
  cardCreditAndAchEnabledInUs,
  openBankingEnabledInGb,
  createPaymentBodyWithCurrency,
  createPaymentBodyWithCurrencyCountry,
} from "../../configs/PaymentMethodList/Commons";
import getConnectorDetails from "../../configs/PaymentMethodList/Utils";

let globalState;

describe("Payment Method List - customer_acceptance_support field", () => {
  context(
    `
    MCA1 -> Stripe configured with credit = { country = "US" } and bank_debit.ach = { country = "US" }\n
    Payment is done with country as US and currency as USD\n
    card.credit should report "supported" (configured in customer_acceptance_support.card.credit)\n
    bank_debit.ach should report "unsupported" (bank_debit has no entry in customer_acceptance_support)\n
    `,
    () => {
      before("seed global state", () => {
        cy.task("getGlobalState").then((state) => {
          globalState = new State(state);
        });
      });

      after("flush global state", () => {
        cy.task("setGlobalState", globalState.data);
      });

      it("merchant-create-call-test", () => {
        cy.merchantCreateCallTest(fixtures.merchantCreateBody, globalState);
      });

      it("api-key-create-call-test", () => {
        cy.apiKeyCreateTest(fixtures.apiKeyCreateBody, globalState);
      });

      it("customer-create-call-test", () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      it("connector-create-call-test", () => {
        cy.createNamedConnectorCallTest(
          "payment_processor",
          fixtures.createConnectorBody,
          cardCreditAndAchEnabledInUs,
          globalState,
          "stripe",
          "stripe_US_default"
        );
      });

      it("create-payment-call-test", () => {
        const data =
          getConnectorDetails("connector")["pm_list"]["PaymentIntent"];

        const newData = {
          ...data,
          Request: data.RequestCurrencyUSD,
          RequestCurrencyUSD: undefined,
        };

        cy.createPaymentIntentTest(
          createPaymentBodyWithCurrency("USD"),
          newData,
          "no_three_ds",
          "automatic",
          globalState
        );
      });

      it("payment-method-list-call-test: card.credit is supported, bank_debit.ach is unsupported", () => {
        const validValues = [
          "supported",
          "partially_supported",
          "unsupported",
        ];

        cy.getPaymentMethodsList(globalState).then((response) => {
          expect(response.headers["content-type"]).to.include(
            "application/json"
          );

          if (response.status === 200) {
            const paymentMethods = response.body["payment_methods"];
            expect(paymentMethods).to.be.an("array");
            expect(paymentMethods.length).to.be.greaterThan(0);

            let sawCardCredit = false;
            let sawBankDebitAch = false;

            paymentMethods.forEach((paymentMethod) => {
              expect(paymentMethod["payment_method_types"]).to.be.an(
                "array"
              );
              paymentMethod["payment_method_types"].forEach(
                (paymentMethodType) => {
                  expect(paymentMethodType).to.have.property(
                    "customer_acceptance_support"
                  );
                  expect(validValues).to.include(
                    paymentMethodType["customer_acceptance_support"]
                  );

                  if (
                    paymentMethod["payment_method"] === "card" &&
                    paymentMethodType["payment_method_type"] === "credit"
                  ) {
                    sawCardCredit = true;
                    expect(
                      paymentMethodType["customer_acceptance_support"]
                    ).to.equal("supported");
                  }

                  if (
                    paymentMethod["payment_method"] === "bank_debit" &&
                    paymentMethodType["payment_method_type"] === "ach"
                  ) {
                    sawBankDebitAch = true;
                    expect(
                      paymentMethodType["customer_acceptance_support"]
                    ).to.equal("unsupported");
                  }
                }
              );
            });

            expect(sawCardCredit, "card.credit entry present").to.be.true;
            expect(sawBankDebitAch, "bank_debit.ach entry present").to.be
              .true;
          } else {
            throw new Error(
              `List payment methods failed with status code "${response.status}"`
            );
          }
        });
      });
    }
  );

  context(
    `
    MCA2 -> Volt configured with bank_redirect.open_banking = { country = "GB", currency = "GBP" }\n
    Payment is done with country as GB and currency as GBP\n
    bank_redirect.open_banking should report "partially_supported" (configured in customer_acceptance_support.bank_redirect.open_banking)\n
    `,
    () => {
      before("seed global state", () => {
        cy.task("getGlobalState").then((state) => {
          globalState = new State(state);
        });
      });

      after("flush global state", () => {
        cy.task("setGlobalState", globalState.data);
      });

      it("merchant-create-call-test", () => {
        cy.merchantCreateCallTest(fixtures.merchantCreateBody, globalState);
      });

      it("api-key-create-call-test", () => {
        cy.apiKeyCreateTest(fixtures.apiKeyCreateBody, globalState);
      });

      it("customer-create-call-test", () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      it("connector-create-call-test", () => {
        cy.createNamedConnectorCallTest(
          "payment_processor",
          fixtures.createConnectorBody,
          openBankingEnabledInGb,
          globalState,
          "volt",
          "volt_GB_default"
        );
      });

      it("create-payment-call-test", () => {
        const data =
          getConnectorDetails("connector")["pm_list"]["PaymentIntent"];

        const newData = {
          ...data,
          Request: {
            currency: "GBP",
            customer_acceptance: null,
            setup_future_usage: "off_session",
            authentication_type: "no_three_ds",
          },
        };

        cy.createPaymentIntentTest(
          createPaymentBodyWithCurrencyCountry("GBP", "GB", "GB"),
          newData,
          "no_three_ds",
          "automatic",
          globalState
        );
      });

      it("payment-method-list-call-test: bank_redirect.open_banking is partially_supported", () => {
        cy.getPaymentMethodsList(globalState).then((response) => {
          expect(response.headers["content-type"]).to.include(
            "application/json"
          );

          if (response.status === 200) {
            const paymentMethods = response.body["payment_methods"];
            expect(paymentMethods).to.be.an("array");

            let sawOpenBanking = false;

            paymentMethods.forEach((paymentMethod) => {
              if (paymentMethod["payment_method"] !== "bank_redirect") {
                return;
              }
              (paymentMethod["payment_method_types"] || []).forEach(
                (paymentMethodType) => {
                  if (
                    paymentMethodType["payment_method_type"] ===
                    "open_banking"
                  ) {
                    sawOpenBanking = true;
                    expect(
                      paymentMethodType["customer_acceptance_support"]
                    ).to.equal("partially_supported");
                  }
                }
              );
            });

            expect(sawOpenBanking, "bank_redirect.open_banking entry present")
              .to.be.true;
          } else {
            throw new Error(
              `List payment methods failed with status code "${response.status}"`
            );
          }
        });
      });
    }
  );
});
