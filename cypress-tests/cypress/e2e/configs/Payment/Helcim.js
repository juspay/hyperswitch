import { cardRequiredField } from "./Commons";
import { getCustomExchange } from "./Modifiers";

const requiredFields = {
  payment_methods: [
    {
      payment_method: "card",
      payment_method_types: [
        {
          payment_method_type: "credit",
          card_networks: [
            {
              eligible_connectors: ["helcim"],
            },
          ],
          required_fields: cardRequiredField,
        },
      ],
    },
  ],
};

export const connectorDetails = {
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
  card_pm: {
    PaymentIntent: getCustomExchange({
      Request: {
        currency: "USD",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    }),
    No3DSAutoCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
    },
    No3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
    },
    Refund: getCustomExchange({
      Request: {
        amount: 6000,
      },
    }),
    PartialRefund: getCustomExchange({
      Request: {
        amount: 2000,
      },
    }),
    SyncRefund: getCustomExchange({}),
  },
};
