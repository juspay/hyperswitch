const requiredFields = {
  payment_methods: [
    {
      payment_method: "wallet",
      payment_method_types: [
        {
          payment_method_type: "ali_pay",
          eligible_connectors: ["globepay"],
        },
        {
          payment_method_type: "we_chat_pay",
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
          setup_future_usage: "on_session",
        },
        Response: {
          status: 200,
          body: {
            status: "requires_payment_method",
            setup_future_usage: "on_session",
          },
        },
      };
    },
    WeChatPay: {
      Request: {
        payment_method: "wallet",
        payment_method_type: "we_chat_pay",
        currency: "GBP",
        customer_acceptance: null,
        setup_future_usage: "on_session",
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
        currency: "GBP",
        customer_acceptance: null,
        setup_future_usage: "on_session",
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
  refund: {
    Refund: {
      Request: {
        amount: 6000,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    PartialRefund: {
      Request: {
        amount: 2000,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    SyncRefund: {
      Response: {
        status: 200,
        body: {
          status: "succeeded",
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
