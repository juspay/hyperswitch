const successfulNo3DSCardDetails = {
  card_number: "4242424242424242",
  card_exp_month: "10",
  card_exp_year: "30",
  card_holder_name: "John",
  card_cvc: "737",
};

const mandateNotSupported = {
  status: 400,
  body: {
    error: {
      type: "invalid_request",
      message: "setup mandate flow not supported",
      code: "IR_20",
    },
  },
};

export const connectorDetails = {
  card_pm: {
    ZeroAuthPaymentIntent: {
      Request: {
        currency: "USD",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    },
    ZeroAuthConfirmPayment: {
      Request: {
        payment_type: "setup_mandate",
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
      },
      Response: mandateNotSupported,
    },
    ZeroAuthMandate: {
      Response: mandateNotSupported,
    },
  },
  bank_transfer_pm: {
    Pix: {
      Request: {
        payment_method: "bank_transfer",
        payment_method_type: "pix",
        payment_method_data: {
          bank_transfer: {
            pix: {
              pix_key: "a1f4102e-a446-4a57-bcce-6fa48899c1d1",
              cnpj: "74469027417312",
              cpf: "10599054689",
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
