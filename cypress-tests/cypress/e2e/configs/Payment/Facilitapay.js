export const connectorDetails = {
  card_pm: {
    ZeroAuthMandate: {
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "setup mandate flow not supported",
            code: "IR_20",
          },
        },
      },
    },
  },
  bank_transfer_pm: {
    PaymentIntent: {
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
    Pix: {
      Request: {
        payment_method: "bank_transfer",
        payment_method_type: "pix",
        payment_method_data: {
          bank_transfer: {
            pix: {
              cpf: "86665623580",
              source_bank_account_id: "739d6b0a-e92a-40fd-9f58-6d4cdeb699bb",
              destination_bank_account_id:
                "91f5cac1-9058-44b7-80e1-80c6f4a6f0bc",
              pix_qr_expiry: "2025-04-10T19:53:54.807Z",
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
            country: "BR",
            first_name: "joseph",
            last_name: "Doe",
          },
          phone: {
            number: "9123456789",
            country_code: "+91",
          },
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
  },
};
