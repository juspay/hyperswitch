import { getCustomExchange } from "./Modifiers";

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

const connectorIntentMetadata = {
  santander: {
    max_mandate_amount: 10000,
  },
};

export const connectorDetails = {
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
        connector_intent_metadata: connectorIntentMetadata,
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
        connector_intent_metadata: connectorIntentMetadata,
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
        connector_intent_metadata: connectorIntentMetadata,
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
