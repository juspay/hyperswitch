import { isoTimeTomorrow } from "../../../utils/RequestBodyUtils";
import { getCustomExchange } from "./Modifiers";

const billingAddress = {
  address: {
    line1: "1467",
    line2: "Harrison Street",
    line3: "Harrison Street",
    city: "San Fransico",
    state: "California",
    zip: "94122",
    country: "US",
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
    // Auto fallback capture method flow (PR #14452) - facilitapay does not support
    // manual capture; with auto_fallback_capture_method enabled on the business
    // profile, manual capture payments fall back to automatic capture.
    // Pix in local currency (BRL) settles synchronously, so the confirmed
    // payment reaches the `succeeded` status without any redirection.
    AutoFallbackCaptureMethod: {
      PaymentIntent: {
        Request: {
          currency: "BRL",
        },
        Response: {
          status: 200,
          body: {
            status: "requires_payment_method",
            capture_method_applied: null,
          },
        },
      },
      AutoFallbackDisabled: {
        Configs: {
          skipBillingAssertion: true,
        },
        Request: {
          payment_method: "bank_transfer",
          payment_method_type: "pix",
          payment_method_data: {
            bank_transfer: {
              pix: {
                // since we pass the same cpf number, the connector customer id will be updated instead of new ones being created
                cpf: "86665623580",
                source_bank_account_id: "739d6b0a-e92a-40fd-9f58-6d4cdeb699bb",
                pix_qr_expiry: isoTimeTomorrow(), // 1 day expiration
              },
            },
          },
          billing: {
            ...billingAddress,
            address: {
              ...billingAddress.address,
              country: "BR",
            },
          },
        },
        Response: {
          status: 400,
          body: {
            error: {
              type: "invalid_request",
              message: "Payment method type not supported",
              code: "IR_19",
              reason: "manual is not supported by facilitapay",
            },
          },
        },
      },
      AutoFallbackEnabled: {
        Request: {
          payment_method: "bank_transfer",
          payment_method_type: "pix",
          payment_method_data: {
            bank_transfer: {
              pix: {
                // since we pass the same cpf number, the connector customer id will be updated instead of new ones being created
                cpf: "86665623580",
                source_bank_account_id: "739d6b0a-e92a-40fd-9f58-6d4cdeb699bb",
                pix_qr_expiry: isoTimeTomorrow(), // 1 day expiration
              },
            },
          },
          billing: {
            ...billingAddress,
            address: {
              ...billingAddress.address,
              country: "BR",
            },
          },
        },
        Response: {
          status: 200,
          body: {
            status: "succeeded",
            capture_method: "manual",
            capture_method_applied: "automatic",
          },
        },
      },
    },
    ZeroAuthPaymentIntent: {
      Request: {
        currency: "BRL",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    },
  },
  bank_transfer_pm: {
    Pix: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true, // Skip this test as HandleRedirection is not required to complete the payment flow for Pix.
      },
      Request: {
        payment_method: "bank_transfer",
        payment_method_type: "pix",
        payment_method_data: {
          bank_transfer: {
            pix: {
              // since we pass the same cpf number, the connector customer id will be updated instead of new ones being created
              cpf: "86665623580",
              source_bank_account_id: "739d6b0a-e92a-40fd-9f58-6d4cdeb699bb",
              pix_qr_expiry: isoTimeTomorrow(), // 1 day expiration
            },
          },
        },
        billing: {
          ...billingAddress,
          address: {
            ...billingAddress.address,
            country: "BR",
          },
        },
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
      ResponseCustom: {
        status: 200,
        body: {
          error_code: "Cancelled",
          error_reason: "Unable to generate Pix QRCode",
        },
      },
    }),
    Refund: getCustomExchange({
      Request: {
        amount: 6000,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
      ResponseCustom: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    }),
    SyncRefund: getCustomExchange({
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    }),
  },
};
