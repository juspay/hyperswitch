import { getCustomExchange } from "./Modifiers";

const successfulNo3DSCardDetails = {
  card_number: "4111111111111111",
  card_exp_month: "08",
  card_exp_year: "30",
  card_holder_name: "joseph Doe",
  card_cvc: "999",
};

const successfulThreeDSTestCardDetails = {
  card_number: "4000000000001091",
  card_exp_month: "08",
  card_exp_year: "30",
  card_holder_name: "joseph Doe",
  card_cvc: "999",
};

const billingAddress = {
  address: {
    line1: "1467",
    line2: "Harrison Street",
    line3: "Harrison Street",
    city: "San Fransico",
    state: "California",
    zip: "94122",
    country: "BR",
    first_name: "joseph",
    last_name: "Doe",
  },
  phone: {
    number: "9123456789",
    country_code: "+91",
  },
};

export const connectorDetails = {
  card_pm: {
    PaymentIntent: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_confirmation",
        },
      },
    }),
    No3DSAutoCapture: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "api",
            message: "Error processing payment",
            code: "IR_00",
          },
        },
      },
    }),
    "3DSAutoCapture": getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "api",
            message: "Error processing payment",
            code: "IR_00",
          },
        },
      },
    }),
    Refund: getCustomExchange({
      Request: {},
      Response: {
        status: 400,
        body: {
          error: {
            type: "api",
            message: "Refund not supported",
            code: "IR_00",
          },
        },
      },
    }),
    SyncRefund: getCustomExchange({
      Request: {},
      Response: {
        status: 400,
        body: {
          error: {
            type: "api",
            message: "Refund not supported",
            code: "IR_00",
          },
        },
      },
    }),
  },
  bank_transfer_pm: {
    Pix: getCustomExchange({
      Request: {
        payment_method: "bank_transfer",
        payment_method_type: "pix",
        payment_method_data: {
          bank_transfer: {
            pix: {
              cpf: "86665623580",
            },
          },
        },
        billing: billingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    }),
    Boleto: getCustomExchange({
      Request: {
        payment_method: "bank_transfer",
        payment_method_type: "boleto",
        payment_method_data: {
          bank_transfer: {
            boleto: {
              cpf: "86665623580",
            },
          },
        },
        billing: billingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    }),
    PixAutomatico: getCustomExchange({
      Request: {
        payment_method: "bank_transfer",
        payment_method_type: "pix_automatico",
        payment_method_data: {
          bank_transfer: {
            pix_automatico: {
              cpf: "86665623580",
            },
          },
        },
        billing: billingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    }),
    PixAutomaticoPush: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "bank_transfer",
        payment_method_type: "pix_automatico_push",
        payment_method_data: {
          bank_transfer: {
            pix_automatico_push: {
              cpf: "86665623580",
              customer_email: "test@example.com",
            },
          },
        },
        billing: billingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    }),
    PixAutomaticoQr: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "bank_transfer",
        payment_method_type: "pix_automatico_qr",
        payment_method_data: {
          bank_transfer: {
            pix_automatico_qr: {
              cpf: "86665623580",
            },
          },
        },
        billing: billingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    }),
  },
};
