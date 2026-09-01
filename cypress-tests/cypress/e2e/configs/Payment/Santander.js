import { getCustomExchange } from "./Modifiers";

const billingAddress = {
  address: {
    line1: "1467",
    city: "Sao Paulo",
    state: "SP",
    zip: "01310100",
    country: "BR",
    first_name: "john",
    last_name: "doe",
  },
};

export const connectorDetails = {
  bank_transfer_pm: {
    PaymentIntent: (paymentMethodType) => {
      const currencyMap = {
        PixAutomaticoQrSetupMandate: "BRL",
        PixAutomaticoQrAutomaticCapture: "BRL",
      };
      return {
        Request: {
          currency: currencyMap[paymentMethodType] || "BRL",
        },
        Response: {
          status: 200,
          body: {
            status: "requires_payment_method",
          },
        },
      };
    },
    Pix: getCustomExchange({
      Request: {
        payment_method: "bank_transfer",
        payment_method_type: "pix",
        payment_method_data: {
          bank_transfer: {
            pix: {},
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            line3: "Harrison Street",
            city: "San Fransico",
            state: "California",
            zip: "94122",
            country: "BR",
            first_name: "john",
            last_name: "doe",
          },
        },
        currency: "BRL",
      },
      Response: {
        status: 500,
        body: {
          error: {
            type: "api",
            code: "HE_00",
          },
        },
      },
      Configs: {
        TRIGGER_SKIP: true,
      },
    }),
    Ach: getCustomExchange({
      Request: {
        payment_method: "bank_transfer",
        payment_method_type: "ach",
        payment_method_data: {
          bank_transfer: {
            ach_bank_transfer: {},
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            line3: "Harrison Street",
            city: "San Fransico",
            state: "California",
            zip: "94122",
            country: "BR",
            first_name: "john",
            last_name: "doe",
          },
        },
        currency: "BRL",
      },
      Response: {
        status: 500,
        body: {
          error: {
            type: "api",
            code: "HE_00",
          },
        },
      },
      Configs: {
        TRIGGER_SKIP: true,
      },
    }),
    InstantBankTransferFinland: getCustomExchange({
      Request: {
        payment_method: "bank_transfer",
        payment_method_type: "instant_bank_transfer_finland",
        payment_method_data: {
          bank_transfer: {
            instant_bank_transfer_finland: {},
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            line3: "Harrison Street",
            city: "San Fransico",
            state: "California",
            zip: "94122",
            country: "FI",
            first_name: "john",
            last_name: "doe",
          },
        },
        currency: "EUR",
      },
      Response: {
        status: 500,
        body: {
          error: {
            type: "api",
            code: "HE_00",
          },
        },
      },
      Configs: {
        TRIGGER_SKIP: true,
      },
    }),
    InstantBankTransferPoland: getCustomExchange({
      Request: {
        payment_method: "bank_transfer",
        payment_method_type: "instant_bank_transfer_poland",
        payment_method_data: {
          bank_transfer: {
            instant_bank_transfer_poland: {},
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            line3: "Harrison Street",
            city: "San Fransico",
            state: "California",
            zip: "94122",
            country: "PL",
            first_name: "john",
            last_name: "doe",
          },
        },
        currency: "PLN",
      },
      Response: {
        status: 500,
        body: {
          error: {
            type: "api",
            code: "HE_00",
          },
        },
      },
      Configs: {
        TRIGGER_SKIP: true,
      },
    }),
    PixAutomaticoQrSetupMandate: getCustomExchange({
      Request: {
        amount: 0,
        currency: "BRL",
        payment_type: "setup_mandate",
        setup_future_usage: "off_session",
        customer_acceptance: {
          acceptance_type: "offline",
          accepted_at: "2026-08-31T00:00:00Z",
          online: {
            ip_address: "192.168.1.1",
            user_agent: "Mozilla/5.0",
          },
        },
        payment_method: "bank_transfer",
        payment_method_type: "pix_automatico_qr",
        payment_method_data: {
          bank_transfer: {
            pix_automatico_qr: {},
          },
        },
        customer: {
          document_details: {
            document_type: "cpf",
            document_number: "44494387100",
          },
        },
        billing: billingAddress,
        feature_metadata: {
          pix_automatico_additional_details: {
            type: "pix_automatico_qr",
            retry_policy: true,
            mandate_details: {
              fixed_recurring_amount: 5000,
              start_date: "2026-08-31",
              end_date: "2027-08-31",
              periodicity: "monthly",
            },
          },
        },
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            code: "IR_04",
          },
        },
      },
      Configs: {
        TRIGGER_SKIP: true,
      },
    }),
    PixAutomaticoQrAutomaticCapture: getCustomExchange({
      Request: {
        amount: 3700,
        currency: "BRL",
        payment_method: "bank_transfer",
        payment_method_type: "pix_automatico_qr",
        payment_method_data: {
          bank_transfer: {
            pix_automatico_qr: {},
          },
        },
        customer: {
          document_details: {
            document_type: "cpf",
            document_number: "12345678909",
          },
        },
        billing: billingAddress,
        feature_metadata: {
          pix_additional_details: {
            immediate: {
              time: 86400,
            },
          },
          pix_automatico_additional_details: {
            type: "pix_automatico_qr",
            retry_policy: true,
            time: 8600,
            mandate_details: {
              fixed_recurring_amount: 5000,
              start_date: "2026-08-31",
              end_date: "2027-08-31",
              periodicity: "monthly",
            },
          },
        },
      },
      Response: {
        status: 500,
        body: {
          error: {
            type: "api",
            code: "HE_00",
          },
        },
      },
      Configs: {
        TRIGGER_SKIP: true,
      },
    }),
  },
};
