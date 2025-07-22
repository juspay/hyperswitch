const billingDetails = {
  address: {
    first_name: "Joseph",
    last_name: "Doe",
    line1: "10 Downing Street",
    line2: "Westminster",
    country: "GB",
    city: "London",
    zip: "SW1A 1AA",
  },
  phone: {
    number: "7912345678",
    country_code: "+44",
  },
};

export const connectorDetails = {
  wallet_pm: {
    PaymentIntent: () => ({
      Request: {
        currency: "GBP",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    }),
    WeChatPay: {
      Request: {
        payment_method: "wallet",
        payment_method_type: "we_chat_pay",
        payment_method_data: {
          wallet: {
            we_chat_pay_qr: {},
          },
        },
        billing: billingDetails,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    Alipay: {
      Request: {
        payment_method: "wallet",
        payment_method_type: "ali_pay",
        payment_method_data: {
          wallet: {
            ali_pay_qr: {},
          },
        },
        billing: billingDetails,
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
