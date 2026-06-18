import { customerAcceptance, multiUseMandateData } from "./Commons";
import {
  getCurrency,
  getCustomExchange,
  getIframeRedirectionConfig,
} from "./Modifiers";

// Reusable billing addresses for bank debit tests
const sepaBillingAddress = {
  address: {
    line1: "1467",
    line2: "Harrison Street",
    line3: "Harrison Street",
    city: "Amsterdam",
    state: "North Holland",
    zip: "1012",
    country: "NL",
    first_name: "John",
    last_name: "Doe",
  },
};

const achBillingAddress = {
  address: {
    line1: "1467",
    line2: "Harrison Street",
    line3: "Harrison Street",
    city: "San Francisco",
    state: "California",
    zip: "94122",
    country: "US",
    first_name: "John",
    last_name: "Doe",
  },
};

const bacsBillingAddress = {
  address: {
    line1: "1467",
    line2: "Harrison Street",
    line3: "Harrison Street",
    city: "London",
    state: "England",
    zip: "SW1A 1AA",
    country: "GB",
    first_name: "John",
    last_name: "Doe",
  },
};

const successfulNo3DSCardDetails = {
  card_number: "4111111111111111",
  card_exp_month: "03",
  card_exp_year: "30",
  card_holder_name: "John Doe",
  card_cvc: "737",
};

const successfulThreeDSTestCardDetails = {
  card_number: "4917610000000000",
  card_exp_month: "03",
  card_exp_year: "30",
  card_holder_name: "Joseph Doe",
  card_cvc: "737",
};

const failedNo3DSCardDetails = {
  card_number: "4242424242424242",
  card_exp_month: "01",
  card_exp_year: "35",
  card_holder_name: "joseph Doe",
  card_cvc: "123",
};

const singleUseMandateData = {
  customer_acceptance: customerAcceptance,
  mandate_type: {
    single_use: {
      amount: 8000,
      currency: "USD",
    },
  },
};

const voucherCurrencyMap = {
  Boleto: "BRL",
  Oxxo: "MXN",
  Alfamart: "IDR",
  Indomaret: "IDR",
  SevenEleven: "JPY",
  Lawson: "JPY",
  MiniStop: "JPY",
  FamilyMart: "JPY",
  Seicomart: "JPY",
  PayEasy: "JPY",
};

const mandateBrowserInfo = {
  user_agent:
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/70.0.3538.110 Safari/537.36",
  accept_header:
    "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,image/apng,*/*;q=0.8",
  language: "nl-NL",
  color_depth: 24,
  screen_height: 723,
  screen_width: 1536,
  time_zone: 0,
  java_enabled: true,
  java_script_enabled: true,
  ip_address: "127.0.0.1",
};

const getMandateData = (currency) => ({
  customer_acceptance: {
    acceptance_type: "online",
    accepted_at: "2025-01-01T00:00:00.000Z",
    online: {
      ip_address: "127.0.0.1",
      user_agent: "Mozilla/5.0",
    },
  },
  mandate_type: {
    multi_use: {
      amount: 6540,
      currency,
    },
  },
});

const onlineCustomerAcceptance = {
  ...customerAcceptance,
  acceptance_type: "online",
};

export const connectorDetails = {
  card_pm: {
    PaymentIntent: {
      Request: {
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    },
    PaymentIntentOffSession: {
      Request: {
        amount: 6000,
        authentication_type: "no_three_ds",
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "off_session",
        billing: sepaBillingAddress,
        payment_type: "new_mandate",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    MandateSingleUseSepa: {
      Request: {
        payment_method: "bank_debit",
        payment_method_type: "sepa",
        currency: "EUR",
        payment_method_data: {
          bank_debit: {
            sepa_bank_debit: {
              iban: "DE89370400440532013000",
              bank_account_holder_name: "John Doe",
            },
          },
          billing: {
            address: {
              line1: "1467",
              line2: "Harrison Street",
              city: "Amsterdam",
              state: "North Holland",
              zip: "1012",
              country: "NL",
              first_name: "John",
              last_name: "Doe",
            },
            email: "test@example.com",
          },
        },
        mandate_data: {
          customer_acceptance: onlineCustomerAcceptance,
          mandate_type: {
            multi_use: {
              amount: 1000,
              currency: "EUR",
              start_date: "2026-04-21T00:00:00Z",
              end_date: "2026-05-21T00:00:00Z",
              metadata: {
                frequency: "13",
              },
            },
          },
        },
        setup_future_usage: "off_session",
        billing: sepaBillingAddress,
        payment_type: "new_mandate",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    MITAutoCaptureSepa: {
      Request: {
        off_session: true,
        confirm: true,
        currency: "EUR",
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    Ach: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "bank_debit",
        payment_method_type: "ach",
        payment_method_data: {
          bank_debit: {
            ach_bank_debit: {
              account_number: "000123456789",
              routing_number: "121000358",
              bank_type: "checking",
              bank_account_holder_name: "John Doe",
            },
          },
          billing: {
            address: {
              line1: "1467",
              line2: "Harrison Street",
              city: "San Francisco",
              state: "California",
              zip: "94122",
              country: "US",
              first_name: "John",
              last_name: "Doe",
            },
            email: "test@example.com",
          },
        },
        billing: achBillingAddress,
          mandate_data: {
          customer_acceptance: onlineCustomerAcceptance,
          mandate_type: {
            multi_use: {
              amount: 8000,
              currency: "USD",
              start_date: "2026-04-21T00:00:00Z",
              end_date: "2026-05-21T00:00:00Z",
              metadata: {
                frequency: "13",
              },
            },
          },
        },
        payment_type: "new_mandate",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    Bacs: {
      Request: {
        payment_method: "bank_debit",
        payment_method_type: "bacs",
        payment_method_data: {
          bank_debit: {
            bacs_bank_debit: {
              account_number: "09083055",
              sort_code: "560036",
              bank_account_holder_name: "David Archer",
            },
          },
          billing: {
            address: {
              line1: "1467",
              line2: "Harrison Street",
              city: "London",
              state: "England",
              zip: "SW1A 1AA",
              country: "GB",
              first_name: "John",
              last_name: "Doe",
            },
            email: "test@example.com",
          },
        },
        billing: bacsBillingAddress,
        customer_acceptance: customerAcceptance,
        mandate_data: {
          customer_acceptance: customerAcceptance,
          mandate_type: {
            multi_use: {
              amount: 8000,
              currency: "GBP",
            },
          },
        },
        payment_type: "new_mandate",
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    MandateSingleUseBacs: {
      Request: {
        payment_method: "bank_debit",
        payment_method_type: "bacs",
        currency: "GBP",
        payment_method_data: {
          bank_debit: {
            bacs_bank_debit: {
              account_number: "09083055",
              sort_code: "560036",
              bank_account_holder_name: "David Archer",
            },
          },
        },
        mandate_data: {
          customer_acceptance: onlineCustomerAcceptance,
          mandate_type: {
            multi_use: {
              amount: 1000,
              currency: "GBP",
              start_date: "2026-04-21T00:00:00Z",
              end_date: "2026-05-21T00:00:00Z",
              metadata: {
                frequency: "13",
              },
            },
          },
        },
        setup_future_usage: "off_session",
        billing: bacsBillingAddress,
        payment_type: "new_mandate",
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    MITAutoCaptureBacs: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        off_session: true,
        confirm: true,
        currency: "GBP",
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    Becs: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "bank_debit",
        payment_method_type: "becs",
        payment_method_data: {
          bank_debit: {
            becs_bank_debit: {
              account_number: "000123456",
              bsb_number: "000000",
              bank_account_holder_name: "John Doe",
            },
          },
        },
        currency: "AUD",
        customer_acceptance: onlineCustomerAcceptance,
        setup_future_usage: "off_session",
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Selected payment method through Adyen is not implemented",
            code: "IR_00",
          },
        },
      },
    },
  },

  bank_debit_pm: {
    PaymentIntent: (paymentMethodType) => {
      if (paymentMethodType === "Ach") {
        return {
          Configs: {
            TRIGGER_SKIP: true,
          },
          Request: {
            currency: "USD",
            setup_future_usage: "off_session",
          },
          Response: {
            status: 200,
            body: {
              status: "requires_payment_method",
            },
          },
        };
      }
      if (paymentMethodType === "Sepa") {
        return {
          Request: {
            currency: "EUR",
          },
          Response: {
            status: 200,
            body: {
              status: "requires_payment_method",
            },
          },
        };
      }
      const currencyMap = {
        Bacs: "GBP",
      };
      return {
        Request: {
          currency: currencyMap[paymentMethodType] || "USD",
          setup_future_usage: "off_session",
        },
        Response: {
          status: 200,
          body: {
            status: "requires_payment_method",
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
            sepa_bank_debit: {
              iban: "DE89370400440532013000",
              bank_account_holder_name: "John Doe",
            },
          },
          billing: {
            address: {
              line1: "1467",
              line2: "Harrison Street",
              city: "Amsterdam",
              state: "North Holland",
              zip: "1012",
              country: "NL",
              first_name: "John",
              last_name: "Doe",
            },
            email: "test@example.com",
          },
        },
        currency: "EUR",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    Ach: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "bank_debit",
        payment_method_type: "ach",
        payment_method_data: {
          bank_debit: {
            ach_bank_debit: {
              account_number: "000123456789",
              routing_number: "121000358",
              bank_type: "checking",
              bank_account_holder_name: "John Doe",
            },
          },
          billing: {
            address: {
              line1: "1467",
              line2: "Harrison Street",
              city: "San Francisco",
              state: "California",
              zip: "94122",
              country: "US",
              first_name: "John",
              last_name: "Doe",
            },
            email: "test@example.com",
          },
        },
        currency: "USD",
        customer_acceptance: customerAcceptance,
        mandate_data: {
          customer_acceptance: customerAcceptance,
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
        body: {
          status: "succeeded",
        },
      },
    },
    Bacs: {
      Request: {
        payment_method: "bank_debit",
        payment_method_type: "bacs",
        payment_method_data: {
          bank_debit: {
            bacs_bank_debit: {
              account_number: "09083055",
              sort_code: "560036",
              bank_account_holder_name: "David Archer",
            },
          },
          billing: {
            address: {
              line1: "1467",
              line2: "Harrison Street",
              city: "London",
              state: "England",
              zip: "SW1A 1AA",
              country: "GB",
              first_name: "John",
              last_name: "Doe",
            },
            email: "test@example.com",
          },
        },
        currency: "GBP",
        customer_acceptance: customerAcceptance,
        mandate_data: {
          customer_acceptance: customerAcceptance,
          mandate_type: {
            multi_use: {
              amount: 8000,
              currency: "GBP",
            },
          },
        },
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
  },
};
