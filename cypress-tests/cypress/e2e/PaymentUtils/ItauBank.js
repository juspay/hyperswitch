export const connectorDetails = {
    card_pm: {
        ZeroAuthMandate: {
            Response: {
              status: 500,
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
                    pix_key: "a1f4102e-a446-4a57-bcce-6fa48899c1d1",
                    cnpj: 74469027417312,
                    cpf: 10599054689
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
            currency: "BRL",
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
