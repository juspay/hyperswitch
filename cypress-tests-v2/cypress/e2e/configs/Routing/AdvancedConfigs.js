const data = [
  {
    connector: "adyen",
    merchant_connector_id: "",
  },
];

const rules = [
  {
    name: "rule_1",
    connectorSelection: {
      type: "priority",
      data: [
        {
          connector: "stripe",
          merchant_connector_id: "",
        },
        {
          connector: "bluesnap",
          merchant_connector_id: "",
        },
      ],
    },
    statements: [
      {
        condition: [
          {
            lhs: "payment_method",
            comparison: "equal",
            value: {
              type: "enum_variant",
              value: "card",
            },
            metadata: {},
          },
        ],
      },
    ],
  },
  {
    name: "rule_2",
    connectorSelection: {
      type: "priority",
      data: [
        {
          connector: "adyen",
          merchant_connector_id: "",
        },
      ],
    },
    statements: [
      {
        condition: [
          {
            lhs: "payment_method",
            comparison: "equal",
            value: {
              type: "enum_variant",
              value: "bank_redirect",
            },
            metadata: {},
          },
        ],
      },
    ],
  },
];

export const configs = {
  name: "Rule Based routing",
  description: "Advanced configuration (Rule based routing) for core flows.",
  data: {
    defaultSelection: {
      type: "priority",
      data: data,
    },
    rules: rules,
    metadata: {},
  },
};
