import { customerAcceptance } from "./Commons";
import { getCustomExchange } from "./Modifiers";

const successful3DSCardDetails = {
  card_number: "4761739090000088",
  card_exp_month: "12",
  card_exp_year: "2034",
  card_holder_name: "John Doe",
  card_cvc: "123",
};

const paymentMethodData3DSResponse = {
  card: {
    last4: "0088",
    card_type: "DEBIT",
    card_network: "Visa",
    card_issuer: "INTL HDQTRS-CENTER OWNED",
    card_issuing_country: "UNITEDSTATES",
    card_isin: "476173",
    card_extended_bin: null,
    card_exp_month: "12",
    card_exp_year: "2034",
    card_holder_name: "John Doe",
    payment_checks: null,
    authentication_data: null,
    auth_code: null,
  },
  billing: null,
};

export const connectorDetails = {
  card_pm: {
    PaymentIntent: {
      Request: {
        currency: "USD",
        customer_acceptance: null,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    },
    "3DSManualCapture": {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successful3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method_data: paymentMethodData3DSResponse,
        },
      },
    },

    "3DSAutoCapture": {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successful3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method_data: paymentMethodData3DSResponse,
        },
      },
    },
    No3DSManualCapture: {
      Request: {
        currency: "USD",
        payment_method: "card",
        payment_method_data: {
          card: successful3DSCardDetails,
        },
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Payment method type not supported",
            code: "IR_19",
          },
        },
      },
    },
    No3DSAutoCapture: {
      Request: {
        currency: "USD",
        payment_method: "card",
        payment_method_data: {
          card: successful3DSCardDetails,
        },
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Payment method type not supported",
            code: "IR_19",
          },
        },
      },
    },
    Capture: {
      Request: {
        amount_to_capture: 6000,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          amount: 6000,
          amount_capturable: 0,
          amount_received: 6000,
        },
      },
    },
    PartialCapture: {
      Request: {
        amount_to_capture: 2000,
      },
      Response: {
        status: 200,
        body: {
          status: "partially_captured",
          amount: 6000,
          amount_capturable: 0,
          amount_received: 2000,
        },
      },
    },
    Refund: {
      Configs: {
        TRIGGER_SKIP: true,
      },
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
    manualPaymentRefund: {
      Configs: {
        TRIGGER_SKIP: true,
      },
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
    manualPaymentPartialRefund: {
      Configs: {
        TRIGGER_SKIP: true,
      },
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
    PartialRefund: {
      Configs: {
        TRIGGER_SKIP: true,
      },
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
      Configs: {
        TRIGGER_SKIP: true,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    ZeroAuthMandate: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Response: {
        status: 200,
        body: {
          amount: 0,
          status: "processing",
        },
      },
    },
    PaymentMethodIdMandate3DSAutoCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successful3DSCardDetails,
        },
        currency: "USD",
        mandate_data: null,
        authentication_type: "three_ds",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method_data: paymentMethodData3DSResponse,
        },
      },
    },
    PaymentMethodIdMandate3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successful3DSCardDetails,
        },
        mandate_data: null,
        authentication_type: "three_ds",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method_data: paymentMethodData3DSResponse,
        },
      },
    },
    ZeroAuthPaymentIntent: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        amount: 0,
        setup_future_usage: "off_session",
        currency: "USD",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
          setup_future_usage: "off_session",
        },
      },
    },
  },
  bank_debit_pm: {
    PaymentIntent: (paymentMethodType) => {
      const currencyMap = { Sepa: "EUR" };
      return {
        Request: {
          currency: currencyMap[paymentMethodType] || "EUR",
        },
        Response: {
          status: 200,
          body: {
            status: "requires_payment_method",
          },
        },
      };
    },
    Sepa: getCustomExchange({
      Request: {
        payment_method: "bank_debit",
        payment_method_type: "sepa",
        payment_method_data: {
          bank_debit: {
            sepa_bank_debit: {
              iban: "DE89370400440532013000",
              bank_account_holder_name: "Test Account",
            },
          },
        },
        billing: {
          address: {
            country: "DE",
            first_name: "Test",
            last_name: "Account",
          },
          email: "test@example.com",
        },
      },
      Response: {
        status: 200,
        body: { status: "processing" },
      },
    }),
    SepaDebitMandate: getCustomExchange({
      Request: {
        payment_method: "bank_debit",
        payment_method_type: "sepa",
        payment_method_data: {
          bank_debit: {
            sepa_bank_debit: {
              iban: "DE89370400440532013000",
              bank_account_holder_name: "Test Account",
            },
          },
        },
        billing: {
          address: {
            country: "DE",
            first_name: "Test",
            last_name: "Account",
          },
          email: "test@example.com",
        },
        setup_future_usage: "off_session",
        mandate_data: {
          customer_acceptance: {
            acceptance_type: "online",
            accepted_at: "1963-05-03T04:07:52.723Z",
            online: {
              ip_address: "127.0.0.1",
              user_agent:
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/70.0.3538.110 Safari/537.36",
            },
          },
          mandate_type: {
            multi_use: {
              amount: 8000,
              currency: "EUR",
            },
          },
        },
      },
      Response: {
        status: 200,
        body: { status: "processing" },
      },
    }),
    AchMandate: getCustomExchange({
      Request: {
        payment_method: "bank_debit",
        payment_method_type: "ach",
        payment_method_data: {
          bank_debit: {
            ach_bank_debit: {
              account_number: "000123456789",
              routing_number: "110000000",
              bank_account_holder_name: "Test Account",
            },
          },
        },
        billing: {
          address: {
            country: "US",
            first_name: "Test",
            last_name: "Account",
          },
          email: "test@example.com",
        },
        setup_future_usage: "off_session",
        mandate_data: {
          customer_acceptance: {
            acceptance_type: "online",
            accepted_at: "1963-05-03T04:07:52.723Z",
            online: {
              ip_address: "127.0.0.1",
              user_agent:
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/70.0.3538.110 Safari/537.36",
            },
          },
          mandate_type: {
            multi_use: {
              amount: 8000,
              currency: "USD",
            },
          },
        },
      },
      Response: {
        status: 200,
        body: { status: "processing" },
      },
    }),
  },
};
