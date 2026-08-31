export const connectorDetails = {
  bank_debit_pm: {
    PaymentIntent: (paymentMethodType) => {
      const currencyMap = { EftDebitOrder: "ZAR" };
      return {
        Request: {
          currency: currencyMap[paymentMethodType] || "ZAR",
        },
        Response: {
          status: 200,
          body: {
            status: "requires_payment_method",
          },
        },
      };
    },
    EftDebitOrder: {
      Request: {
        payment_method: "bank_debit",
        payment_method_type: "eft_debit_order",
        payment_method_data: {
          bank_debit: {
            eft_debit_order: {
              account_number: "000123456789",
              bank_name: "absa",
              bank_account_holder_name: "John Doe",
              bank_type: "checking",
              branch_code: "110000000",
            },
          },
        },
        billing: {
          address: {
            line1: "123 Test St",
            city: "Johannesburg",
            state: "Gauteng",
            zip: "2000",
            country: "ZA",
          },
          email: "test@example.com",
        },
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "No eligible connector was found for the current payment method configuration",
            code: "IR_39",
          },
        },
      },
      Configs: {
        TRIGGER_SKIP: false,
        skipBillingAssertion: true,
      },
    },
  },
};
