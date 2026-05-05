import { getCustomExchange } from "./Modifiers";

const billingAddress = {
  address: {
    line1: "Rua Augusta",
    line2: "2000",
    line3: "Consolação",
    city: "São Paulo",
    state: "SP",
    zip: "01412-000",
    country: "BR",
    first_name: "joseph",
    last_name: "Doe",
  },
  phone: {
    number: "11991234567",
    country_code: "+55",
  },
};

const connectorIntentMetadata = {
  santander: {
    max_mandate_amount: 10000,
  },
};

const cardSkipConfig = getCustomExchange({
  Configs: { TRIGGER_SKIP: true },
  Request: {},
  Response: {},
});

export const connectorDetails = {
  // Santander does not support card payments — all card flows skipped
  card_pm: {
    PaymentIntent: cardSkipConfig,
    PaymentIntentOffSession: cardSkipConfig,
    PaymentIntentWithShippingCost: cardSkipConfig,
    PaymentConfirmWithShippingCost: cardSkipConfig,
    "3DSManualCapture": cardSkipConfig,
    "3DSAutoCapture": cardSkipConfig,
    No3DSManualCapture: cardSkipConfig,
    No3DSAutoCapture: cardSkipConfig,
    Capture: cardSkipConfig,
    PartialCapture: cardSkipConfig,
    Void: cardSkipConfig,
    VoidAfterConfirm: cardSkipConfig,
    Refund: cardSkipConfig,
    PartialRefund: cardSkipConfig,
    SyncRefund: cardSkipConfig,
    manualPaymentRefund: cardSkipConfig,
    manualPaymentPartialRefund: cardSkipConfig,
    ConnectorMetadata: cardSkipConfig,
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
        customer: {
          customer_document_details: {
            document_type: "CPF",
            document_number: "86665623580",
          },
        },
        pix_additional_details: {
          cpf: "86665623580",
        },
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
        customer: {
          customer_document_details: {
            document_type: "CPF",
            document_number: "86665623580",
          },
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    }),
    // PixAutomatico requires connector_intent_metadata for mandate setup (CIT flow)
    // see transformers.rs: SetupMandate -> connector_intent_metadata.santander.pix_automatico.cit
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
        customer: {
          customer_document_details: {
            document_type: "CPF",
            document_number: "86665623580",
          },
        },
        connector_intent_metadata: connectorIntentMetadata,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    }),
    // PixAutomaticoPush skipped: requires MIT connector_intent_metadata with receiver_details
    // (branch_code, account_number, account_type) not available in test environment
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
    // PixAutomaticoQr skipped: requires MIT connector_intent_metadata with receiver_details
    // (branch_code, account_number, account_type) not available in test environment
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
  },
};
