const requiredFields = {
  payment_methods: [
    {
      payment_method: "wallet",
      payment_method_types: [
        {
          payment_method_type: "ali_pay",
          payment_experience: "qr_code_information",
          eligible_connectors: ["globepay"],
        },
        {
          payment_method_type: "we_chat_pay",
          payment_experience: "qr_code_information",
          eligible_connectors: ["globepay"],
        },
      ],
    },
  ],
};

export const connectorDetails = {
  wallet_pm: {
    PaymentIntent: (walletType) => {
      const currencyMap = {
        WeChatPay: "GBP",
        AliPay: "GBP",
      };
      return {
        Request: {
          currency: currencyMap[walletType] || "GBP",
          customer_acceptance: null,
        },
        Response: {
          status: 200,
          body: {
            status: "requires_payment_method",
          },
        },
      };
    },
    WeChatPay: {
      Request: {
        payment_method: "wallet",
        payment_method_type: "we_chat_pay",
        payment_method_data: {
          wallet: {
            we_chat_pay_qr: {},
          },
        },
        billing: {
          address: {
            country: "GB",
          },
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method: "wallet",
          payment_method_type: "we_chat_pay",
          attempt_count: 1,
        },
      },
    },
    AliPay: {
      Request: {
        payment_method: "wallet",
        payment_method_type: "ali_pay",
        payment_method_data: {
          wallet: {
            ali_pay_qr: {},
          },
        },
        billing: {
          address: {
            country: "GB",
          },
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method: "wallet",
          payment_method_type: "ali_pay",
          attempt_count: 1,
        },
      },
    },
  },
  pm_list: {
    PmListResponse: {
      PmListNull: {
        payment_methods: [],
      },
      pmListDynamicFieldWithoutBilling: requiredFields,
      pmListDynamicFieldWithBilling: requiredFields,
      pmListDynamicFieldWithNames: requiredFields,
      pmListDynamicFieldWithEmail: requiredFields,
    },
  },
};
