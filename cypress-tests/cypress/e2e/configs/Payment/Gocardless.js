const sepaBankDebitDetails = {
  iban: "DE89370400440532013000",
  account_holder_name: "Test User",
};

const becsBankDebitDetails = {
  bsb_number: "000000",
  account_number: "000123456",
  bank_account_holder_name: "Test User",
};

const achBankDebitDetails = {
  routing_number: "110000000",
  account_number: "000123456789",
  account_type: "checking",
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
        {
          payment_method_type: "becs",
          recurring_enabled: false,
          installment_payment_enabled: false,
        },
        {
          payment_method_type: "ach",
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
        Becs: "AUD",
        Ach: "USD",
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
            line1: "Friedrichstrasse",
            line2: "123",
            city: "Berlin",
            state: "Berlin",
            zip: "10117",
            country: "DE",
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
            line1: "Friedrichstrasse",
            line2: "123",
            city: "Berlin",
            state: "Berlin",
            zip: "10117",
            country: "DE",
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
    Becs: {
      Request: {
        payment_method: "bank_debit",
        payment_method_type: "becs",
        payment_method_data: {
          bank_debit: {
            becs_bank_debit: becsBankDebitDetails,
          },
        },
        currency: "AUD",
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            city: "Sydney",
            state: "NSW",
            zip: "2000",
            country: "AU",
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
    BecsNo3DSAutoCapture: {
      Request: {
        payment_method: "bank_debit",
        payment_method_type: "becs",
        payment_method_data: {
          bank_debit: {
            becs_bank_debit: becsBankDebitDetails,
          },
        },
        currency: "AUD",
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            city: "Sydney",
            state: "NSW",
            zip: "2000",
            country: "AU",
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
    Ach: {
      Request: {
        payment_method: "bank_debit",
        payment_method_type: "ach",
        payment_method_data: {
          bank_debit: {
            ach_bank_debit: achBankDebitDetails,
          },
        },
        currency: "USD",
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            city: "San Francisco",
            state: "California",
            zip: "94122",
            country: "US",
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
    AchNo3DSAutoCapture: {
      Request: {
        payment_method: "bank_debit",
        payment_method_type: "ach",
        payment_method_data: {
          bank_debit: {
            ach_bank_debit: achBankDebitDetails,
          },
        },
        currency: "USD",
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            city: "San Francisco",
            state: "California",
            zip: "94122",
            country: "US",
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
          status: "succeeded",
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
          status: "succeeded",
        },
      },
    },
    SyncRefund: {
      Response: {
        status: 200,
        body: {
          status: "succeeded",
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
