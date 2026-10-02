import { standardBillingAddress } from "./Commons";
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
        status: 400,
        body: {
          error: {
            type: "invalid_request",
          },
        },
      },
      Configs: {
        TRIGGER_SKIP: true,
      },
    }),
  },
  card_pm: {
    No3DSAutoCapture: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: {
            card_number: "4111111111111111",
            card_exp_month: "03",
            card_exp_year: "30",
            card_holder_name: "John Doe",
            card_cvc: "737",
          },
        },
        currency: "USD",
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
  bank_redirect_pm: {
    Blik: getCustomExchange({
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "blik",
        payment_method_data: {
          bank_redirect: {
            blik: {
              blik_code: "777987",
            },
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
    Eps: getCustomExchange({
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "eps",
        payment_method_data: {
          bank_redirect: {
            eps: {
              bank_name: "ing",
            },
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
            country: "AT",
            first_name: "john",
            last_name: "doe",
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
    Giropay: getCustomExchange({
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "giropay",
        payment_method_data: {
          bank_redirect: {
            giropay: {
              bank_name: "",
              bank_account_bic: "",
              bank_account_iban: "",
              preferred_language: "en",
              country: "DE",
            },
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
            country: "DE",
            first_name: "john",
            last_name: "doe",
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
    Ideal: getCustomExchange({
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "ideal",
        payment_method_data: {
          bank_redirect: {
            ideal: {
              bank_name: "ing",
              country: "NL",
            },
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
            country: "NL",
            first_name: "john",
            last_name: "doe",
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
    Sofort: getCustomExchange({
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "sofort",
        payment_method_data: {
          bank_redirect: {
            sofort: {
              country: "DE",
              preferred_language: "en",
            },
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
            country: "DE",
            first_name: "john",
            last_name: "doe",
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
    Przelewy24: getCustomExchange({
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "przelewy24",
        payment_method_data: {
          bank_redirect: {
            przelewy24: {
              bank_name: "citi",
              billing_details: {
                email: "guest@juspay.in",
              },
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
    OpenBankingUk: getCustomExchange({
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "open_banking_uk",
        payment_method_data: {
          bank_redirect: {
            open_banking_uk: {
              issuer: "citi",
              country: "GB",
            },
          },
        },
        billing: standardBillingAddress,
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
    OnlineBankingFpx: getCustomExchange({
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "online_banking_fpx",
        payment_method_data: {
          bank_redirect: {
            online_banking_fpx: {
              issuer: "affin_bank",
            },
          },
        },
        billing: standardBillingAddress,
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
    Interac: getCustomExchange({
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "interac",
        payment_method_data: {
          bank_redirect: {
            interac: {
              bank_name: "ing",
            },
          },
        },
        billing: {
          ...standardBillingAddress,
          address: {
            ...standardBillingAddress.address,
            country: "CA",
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
    Eft: getCustomExchange({
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "eft",
        payment_method_data: {
          bank_redirect: {
            eft: {
              provider: "ozow",
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
