const successfulRefundResponse = {
  status: 200,
  body: {
    status: "succeeded",
  },
};

export const connectorDetails = {
  amazonpay_wallet: {
    PaymentIntent: {
      Request: {
        currency: "USD",
        customer_acceptance: null,
        amount: 6540,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    },
    PaymentConfirm: {
      Request: {
        payment_method: "wallet",
        payment_method_data: {
          wallet: {
            amazon_pay: {},
          },
        },
        customer_acceptance: null,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          amount: 6540,
        },
      },
    },
    Refund: {
      Request: {
        amount: 6540,
        reason: "Customer request",
      },
      Response: successfulRefundResponse,
    },
    PartialRefund: {
      Request: {
        amount: 3000,
        reason: "Partial refund",
      },
      Response: successfulRefundResponse,
    },
    SyncRefund: {
      Response: successfulRefundResponse,
    },
  },
  metadata: {
    connector_name: "amazonpay",
    display_name: "Amazon Pay",
    payment_methods: ["wallet"],
    supported_flows: ["payment", "refund"],
    refund_supported: true,
  },
};

export default connectorDetails;
