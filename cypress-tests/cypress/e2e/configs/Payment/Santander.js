import { getCustomExchange, getCurrency } from "./Modifiers";

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

const customerDocumentDetails = {
  customer_document_details: {
    document_type: "CPF",
    document_number: "86665623580",
  },
};

const connectorMetadata = {
  santander: {
    pix_automatico: {
      cit: {
        mandate_details: {
          start_date: "2026-06-01",
          end_date: "2027-06-01",
          periodicity: "mensal",
          max_mandate_amount: 10000,
        },
        retry_policy: false,
      },
    },
  },
};

const cardCreateSkipConfig = getCustomExchange({
  Configs: { TRIGGER_SKIP: true },
  Request: { currency: "BRL" },
  Response: {
    status: 200,
    body: {
      status: "requires_payment_method",
    },
  },
});

const cardConfirmSkipConfig = getCustomExchange({
  Configs: { TRIGGER_SKIP: true },
  Request: { currency: "BRL" },
  Response: {
    status: 500,
    body: {
      error: {
        type: "api",
        message: "Something went wrong",
        code: "HE_00",
      },
    },
  },
});

const bankTransferSkipConfigs = {
  InstantBankTransferFinland: getCustomExchange({
    Configs: { TRIGGER_SKIP: true },
    Request: { currency: "EUR" },
    Response: {
      status: 200,
      body: {
        status: "requires_payment_method",
      },
    },
  }),
  InstantBankTransferPoland: getCustomExchange({
    Configs: { TRIGGER_SKIP: true },
    Request: { currency: "PLN" },
    Response: {
      status: 200,
      body: {
        status: "requires_payment_method",
      },
    },
  }),
  Ach: getCustomExchange({
    Configs: { TRIGGER_SKIP: true },
    Request: { currency: "BRL" },
    Response: {
      status: 200,
      body: {
        status: "requires_payment_method",
      },
    },
  }),
};

export const connectorDetails = {
  card_pm: {
    PaymentIntent: cardCreateSkipConfig,
    PaymentIntentOffSession: cardCreateSkipConfig,
    PaymentIntentWithShippingCost: cardCreateSkipConfig,
    PaymentConfirmWithShippingCost: cardConfirmSkipConfig,
    "3DSManualCapture": cardConfirmSkipConfig,
    "3DSAutoCapture": cardConfirmSkipConfig,
    No3DSManualCapture: cardConfirmSkipConfig,
    No3DSAutoCapture: cardConfirmSkipConfig,
    Capture: cardConfirmSkipConfig,
    PartialCapture: cardConfirmSkipConfig,
    Void: cardConfirmSkipConfig,
    VoidAfterConfirm: cardConfirmSkipConfig,
    Refund: cardConfirmSkipConfig,
    PartialRefund: cardConfirmSkipConfig,
    SyncRefund: cardConfirmSkipConfig,
    manualPaymentRefund: cardConfirmSkipConfig,
    manualPaymentPartialRefund: cardConfirmSkipConfig,
    ConnectorMetadata: cardConfirmSkipConfig,
  },
  bank_transfer_pm: {
    PaymentIntent: (paymentMethodType) => {
      const unsupportedMethods = [
        "InstantBankTransferFinland",
        "InstantBankTransferPoland",
        "Ach",
      ];
      if (unsupportedMethods.includes(paymentMethodType)) {
        return bankTransferSkipConfigs[paymentMethodType];
      }
      return getCustomExchange({
        Request: { currency: getCurrency(paymentMethodType) },
        Response: {
          status: 200,
          body: {
            status: "requires_payment_method",
          },
        },
      });
    },
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
        customer: customerDocumentDetails,
        feature_metadata: {
          pix_additional_details: {
            immediate: {
              time: 3600,
            },
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
        customer: customerDocumentDetails,
        feature_metadata: {
          boleto_additional_details: {
            due_date: "2030-12-31",
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
        customer: customerDocumentDetails,
        connector_metadata: connectorMetadata,
        setup_future_usage: "off_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    }),
    // PixAutomaticoPush skipped: requires MIT connector_metadata with receiver_details
    // (branch_code, account_number, account_type) not available in test environment
    PixAutomaticoPush: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        currency: "BRL",
      },
    }),
    // PixAutomaticoQr skipped: requires MIT connector_metadata with receiver_details
    // (branch_code, account_number, account_type) not available in test environment
    PixAutomaticoQr: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        currency: "BRL",
      },
    }),
    InstantBankTransferFinland:
      bankTransferSkipConfigs.InstantBankTransferFinland,
    InstantBankTransferPoland:
      bankTransferSkipConfigs.InstantBankTransferPoland,
    Ach: bankTransferSkipConfigs.Ach,
  },
};
