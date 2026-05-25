const sepaBankDebitDetails = {
  iban: "ES9121000418450200051332",
  account_holder_name: "Test User",
};

const requiredFields = {
  payment_methods: [
    {
      payment_method: "bank_debit",
      payment_method_types: [
        {
          payment_method_type: "sepa",
          recurring_enabled: false,
          installment_payment_enabled: false,
        },
      ],
    },
  ],
};

export const connectorDetails = {
  bank_debit_pm: {
    PaymentIntent: (paymentMethodType) => {
      const currencyMap = {
        Sepa: "EUR",
      };
      return {
        Request: {
          currency: currencyMap[paymentMethodType] || "EUR",
          customer_acceptance: null,
          setup_future_usage: "on_session",
        },
        Response: {
          status: 200,
          body: {
            status: "requires_payment_method",
            setup_future_usage: "on_session",
          },
        },
      };
    },
    Sepa: {
      Request: {
        payment_method: "bank_debit",
        payment_method_type: "sepa",
        payment_method_data: {
          bank_debit: {
            sepa_bank_debit: sepaBankDebitDetails,
          },
        },
        currency: "EUR",
        billing: {
          address: {
            line1: "Calle de Alcalá",
            line2: "45",
            city: "Madrid",
            state: "Madrid",
            zip: "28014",
            country: "ES",
            first_name: "joseph",
            last_name: "Doe",
          },
          email: "johndoe@mail.com",
        },
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method: "bank_debit",
          attempt_count: 1,
        },
      },
    },
    SepaNo3DSAutoCapture: {
      Request: {
        payment_method: "bank_debit",
        payment_method_type: "sepa",
        payment_method_data: {
          bank_debit: {
            sepa_bank_debit: sepaBankDebitDetails,
          },
        },
        currency: "EUR",
        billing: {
          address: {
            line1: "Calle de Alcalá",
            line2: "45",
            city: "Madrid",
            state: "Madrid",
            zip: "28014",
            country: "ES",
            first_name: "joseph",
            last_name: "Doe",
          },
          email: "johndoe@mail.com",
        },
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method: "bank_debit",
          attempt_count: 1,
        },
      },
    },
    Refund: {
      Request: {
        amount: 6000,
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
        },
      },
    },
    PartialRefund: {
      Request: {
        amount: 2000,
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
        },
      },
    },
    SyncRefund: {
      Response: {
        status: 200,
        body: {
          status: "failed",
        },
      },
    },
    manualPaymentRefund: {
      Request: {
        amount: 6000,
      },
      Response: {
        status: 200,
        body: {
          status: "pending",
        },
      },
    },
    manualPaymentPartialRefund: {
      Request: {
        amount: 2000,
      },
      Response: {
        status: 200,
        body: {
          status: "pending",
        },
      },
    },
  },
  pm_list: {
    PmListResponse: {
      PmListNull: {
        payment_methods: [],
      },
      pmListDynamicFieldWithoutBilling: requiredFields,
      pmListDynamicFieldWithBilling: requiredFields,
      pmListDynamicFieldWithNames: requiredFields,
      pmListDynamicFieldWithEmail: requiredFields,
    },
  },
};
